//! Real candidates must answer turns, deliver at a boundary, compact and stop.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

use super::{Candidate, admit};
use super::super::{Spawned, frame, spawn};
use crate::harness_control::{Binding, Controller, Kind, Pending, Transport, Update};
use crate::operations::OperationState;
use crate::protocol::Launch;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

struct Owned {
    spawned: Spawned,
    stopped: bool,
}

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.stopped {
            match self.spawned.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {},
                Err(error) => crate::error::said(&format!("qualification_exit_unconfirmed: {error}")),
            }
            if let Err(error) = crate::pty::end(&self.spawned.binding.leader) {
                crate::error::said(&format!("qualification_cleanup_failed: {error}"));
            }
            if let Err(error) = self.spawned.child.wait() {
                crate::error::said(&format!("qualification_exit_unconfirmed: {error}"));
            }
        }
    }
}

fn write(writer: &mut dyn Write, update: &Update) -> TestResult {
    for dispatch in &update.dispatches {
        serde_json::to_writer(&mut *writer, &dispatch.frame)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn answer_present(value: &Value, transport: Transport) -> bool {
    match transport {
        Transport::Claude => value.get("type").and_then(Value::as_str) == Some("assistant")
            && value.pointer("/message/content").and_then(Value::as_array)
                .is_some_and(|content| content.iter().any(|part|
                    part.get("text").and_then(Value::as_str).is_some_and(|text| !text.is_empty()))),
        Transport::Codex => value.pointer("/params/item/type").and_then(Value::as_str) == Some("agentMessage")
            && value.pointer("/params/item/text").and_then(Value::as_str).is_some_and(|text| !text.is_empty()),
        Transport::Pty => false,
    }
}

fn ingest(
    writer: &mut dyn Write,
    controller: &mut Controller,
    source: &Binding,
    value: &Value,
) -> Result<Update, Box<dyn Error>> {
    let update = controller.ingest(source, value)?;
    write(writer, &update)?;
    Ok(update)
}

fn turn(
    reader: &mut BufReader<Box<dyn Read + Send>>,
    writer: &mut dyn Write,
    controller: &mut Controller,
    source: &Binding,
    transport: Transport,
    operation: &str,
    boundary_delivery: bool,
) -> TestResult {
    let mut answered = false;
    let mut admitted = false;
    loop {
        let value = frame(reader)?;
        answered |= answer_present(&value, transport);
        let update = ingest(writer, controller, source, &value)?;
        for receipt in &update.receipts {
            if receipt.operation == operation {
                assert_eq!(receipt.state, OperationState::Confirmed, "{}", receipt.reason);
                admitted |= receipt.reason == "harness_admitted";
            }
        }
        let ended = update.events.iter().any(|event|
            event.event == "turn_end" && event.operation.as_deref() == Some(operation));
        if boundary_delivery {
            for dispatch in &update.dispatches {
                if dispatch.operation == "qualification-boundary" {
                    assert!(ended, "queued delivery preceded the first turn boundary");
                }
            }
        }
        if ended {
            assert!(answered, "the harness ended the turn without an answer");
            assert!(admitted, "the harness did not prove delivery of the turn");
            return Ok(());
        }
    }
}

fn compact(
    reader: &mut BufReader<Box<dyn Read + Send>>,
    writer: &mut dyn Write,
    controller: &mut Controller,
    source: &Binding,
    transport: Transport,
) -> TestResult {
    let pending = Pending::new("qualification-compact".to_owned(), Kind::Compact, String::new());
    write(writer, &controller.enqueue(pending)?)?;
    let mut evidence = false;
    loop {
        let value = frame(reader)?;
        evidence |= match transport {
            Transport::Claude => value.get("type").and_then(Value::as_str) == Some("system")
                && value.get("subtype").and_then(Value::as_str) == Some("compact_boundary")
                && value.get("session_id").and_then(Value::as_str) == Some(controller.binding.conversation.as_str())
                && value.pointer("/compact_metadata/trigger").and_then(Value::as_str) == Some("manual"),
            Transport::Codex => value.get("method").and_then(Value::as_str) == Some("item/completed")
                && value.pointer("/params/item/type").and_then(Value::as_str) == Some("contextCompaction")
                && value.pointer("/params/threadId").and_then(Value::as_str) == Some(controller.binding.conversation.as_str()),
            Transport::Pty => false,
        };
        let update = ingest(writer, controller, source, &value)?;
        if let Some(receipt) = update.receipts.iter().find(|receipt| receipt.operation == "qualification-compact") {
            assert_eq!(receipt.state, OperationState::Confirmed, "{}", receipt.reason);
            assert_eq!(receipt.reason, "harness_compacted");
            assert!(evidence, "compaction lacks the harness's correlated evidence");
            assert!(controller.idle());
            return Ok(());
        }
    }
}

fn qualify(variable: &str, transport: Transport, adapter: &'static str, version: &'static str) -> TestResult {
    let Some(binary) = std::env::var_os(variable) else {
        eprintln!("qualification_unrun: {variable} names no real binary");
        return Ok(());
    };
    let entry = std::env::var_os("LYS_QUALIFY_ENTRY")
        .map(PathBuf::from).ok_or("qualification_entry_missing: LYS_QUALIFY_ENTRY must name the real lys binary")?;
    let _admission = admit(Candidate { adapter, version, entry })?;
    let scratch = tempfile::tempdir()?;
    let home = scratch.path().join("home");
    let working = scratch.path().join("working");
    std::fs::create_dir(&home)?;
    std::fs::create_dir(&working)?;
    let conversation = if transport == Transport::Claude {
        Pending::new("qualification".to_owned(), Kind::Human, String::new()).uuid
    } else { String::new() };
    let launch = Launch {
        session:"qualification".to_owned(), program:binary.into_string().map_err(|_| "qualification_binary_path_not_text")?,
        arguments:Vec::new(), directory:working.to_str().ok_or("qualification_working_path_not_text")?.to_owned(),
        environment:BTreeMap::from([("HOME".to_owned(), home.to_str().ok_or("qualification_home_path_not_text")?.to_owned())]),
        config:None, columns:80, rows:24, rotation:None, policy:None,
    };
    let mut owned = Owned { spawned:spawn(&launch, transport, &conversation, 1, false)?, stopped:false };
    let binding = owned.spawned.binding.clone();
    assert_eq!(binding.adapter, adapter);
    assert_eq!(binding.harness_version, version);
    let mut reader = BufReader::new(std::mem::replace(&mut owned.spawned.reader, Box::new(std::io::empty())));
    let writer = &mut owned.spawned.writer;
    let mut controller = Controller::new(binding.clone(), transport)?;
    write(writer.as_mut(), &controller.bootstrap())?;
    while !controller.ready() {
        let value = frame(&mut reader)?;
        ingest(writer.as_mut(), &mut controller, &binding, &value)?;
    }
    let first = Pending::new("qualification-answer".to_owned(), Kind::Human,
        "Reply with one short acknowledgement. Do not use tools.".to_owned());
    write(writer.as_mut(), &controller.enqueue(first)?)?;
    let queued = controller.enqueue(Pending::new("qualification-boundary".to_owned(), Kind::Reminder,
        "Reply with a short acknowledgement of this boundary delivery. Do not use tools.".to_owned()))?;
    assert!(queued.dispatches.is_empty(), "delivery did not wait for a turn boundary");
    turn(&mut reader, writer.as_mut(), &mut controller, &binding, transport, "qualification-answer", true)?;
    turn(&mut reader, writer.as_mut(), &mut controller, &binding, transport, "qualification-boundary", false)?;
    compact(&mut reader, writer.as_mut(), &mut controller, &binding, transport)?;
    drop(std::mem::replace(writer, Box::new(std::io::sink())));
    let drain = std::thread::Builder::new().spawn(move || std::io::copy(&mut reader, &mut std::io::sink()))?;
    let status = owned.spawned.child.wait();
    let drained = drain.join().map_err(|_| "qualification_reader_panicked")?;
    drained?;
    let status = status?;
    owned.stopped = true;
    assert!(status.success(), "qualification_stop_failed: {status:?}");
    println!("QUALIFIED adapter={adapter} version={} executable_sha256={} start=proved answer=received boundary=received compaction=proved stop=clean",
        binding.harness_version, binding.harness.sha256);
    Ok(())
}

#[test]
fn qualify_launched_claude() -> TestResult {
    qualify("LYS_QUALIFY_CLAUDE", Transport::Claude, "claude-stream-json/1", "2.1.296")
}

#[test]
fn qualify_launched_codex() -> TestResult {
    qualify("LYS_QUALIFY_CODEX", Transport::Codex, "codex-app-server/1", "0.161.0-alpha.3")
}
