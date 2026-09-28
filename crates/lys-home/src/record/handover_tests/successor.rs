#![cfg(test)]
//! The successor a handover makes: the letter copied whole into its home, the
//! subcommand's report and refusals, and the successor rendered for its own
//! model and for another.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cli::{Cli, Command, run};
use crate::harness::claude_code::render::{RenderReport, render_claude_code};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_INHERITED, EntryBody};
use crate::record::handover::{HandoverReport, handover};
use crate::record::index::{Index, read_head};

use super::{Gate, OUTGOING, carries_no_content, fixture, ids, snapshot, target};

/// The successor file's lines after the header, as JSON values.
fn successor_lines(file: &Path) -> Result<(Value, Vec<Value>), Box<dyn Error>> {
    let text = std::fs::read_to_string(file)?;
    let mut lines = text.lines().map(serde_json::from_str::<Value>);
    let header = lines.next().ok_or("no header")??;
    Ok((header, lines.collect::<Result<_, _>>()?))
}

fn source_message(root: &Path, id: &str) -> Result<Value, Box<dyn Error>> {
    let reader = Home::read(root)?.read_session(OUTGOING)?;
    let EntryBody::Message { message } = reader.entry(id)?.body else {
        return Err(format!("{id} is not a message").into());
    };
    Ok(message)
}

/// Hand `[a1, a2]` of the fixture to `successor` and check what was written.
fn hand_over_and_check(root: &Path, successor: &Path) -> Result<HandoverReport, Box<dyn Error>> {
    let before = snapshot(root)?;
    let report = handover(root, OUTGOING, &ids(&["a1", "a2"]), successor)?;
    assert_eq!(snapshot(root)?, before);
    let (header, entries) = successor_lines(&report.file)?;
    let id = header["id"].as_str().ok_or("no id")?;
    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    assert!(id.len() == 32 && id.bytes().all(hex), "{id}");
    assert_eq!(header["cwd"], "/work/outgoing");
    assert!(header.get("parentSession").is_none());
    assert_eq!(
        report.file,
        successor.join("sessions").join(format!("{id}.jsonl"))
    );
    assert_eq!(entries.len(), 4);
    let mark = &entries[0];
    assert_eq!(
        (mark["type"].as_str(), mark["customType"].as_str()),
        (Some("custom"), Some(CUSTOM_INHERITED))
    );
    assert!(mark["parentId"].is_null());
    assert_eq!(
        mark["data"],
        json!({"authored": false, "from_session": "outgoing", "from_entries": ["a1", "a2"],
            "provider": "anthropic", "api": "anthropic-messages", "model": "claude-fixture-model",
            "curated_at": "fixture-time-a2", "curated_by": "outgoing"})
    );
    assert!(mark["data"].get("rule").is_none());
    for (n, (id, parent)) in [("a1", &mark["id"]), ("a2", &json!("a1"))]
        .into_iter()
        .enumerate()
    {
        let copy = &entries[n + 1];
        assert_eq!((&copy["id"], &copy["parentId"]), (&json!(id), parent));
        assert_eq!(copy["timestamp"], format!("fixture-time-{id}"));
        assert_eq!(copy["message"], source_message(root, id)?);
    }
    let signature = entries[1]["message"]["content"][0]["thinkingSignature"]
        .as_str()
        .ok_or("no signature")?;
    assert_eq!(signature.as_bytes(), b"fixture-signature-a1");
    let info = &entries[3];
    let named = json!({"type": "session_info", "id": info["id"], "parentId": "a2",
        "timestamp": info["timestamp"], "name": "inherited from outgoing"});
    assert_eq!(*info, named);
    let (_, index, _) = Index::read(&report.file)?;
    assert_eq!(
        read_head(&report.file, &index)?.as_deref(),
        info["id"].as_str()
    );
    assert_eq!(std::fs::read_dir(successor.join("blocks"))?.count(), 0);
    Ok(report)
}

#[test]
fn the_letter_is_copied_whole_into_a_new_successor_home() -> Gate {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("successor");
    let report = hand_over_and_check(&root, &successor)?;
    let shown = serde_json::to_value(&report)?;
    let whole = json!({"successor_home": successor, "session": report.session,
        "file": report.file, "inherited": true});
    assert_eq!(shown, whole);
    let (header, _) = successor_lines(&report.file)?;
    assert_eq!(header["id"], report.session.as_str());
    assert!(carries_no_content(&shown.to_string()));
    Ok(())
}

#[test]
fn an_empty_successor_directory_takes_the_same_four_entries() -> Gate {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("empty");
    std::fs::create_dir(&successor)?;
    let report = hand_over_and_check(&root, &successor)?;
    assert!(report.inherited);
    Ok(())
}

#[test]
fn the_handover_subcommand_prints_its_report_and_refuses_by_name() -> Gate {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("successor");
    let argv = |letter: &str| {
        let line = format!(
            "lys-home handover --home {} --from outgoing --letter {letter} --successor {}",
            root.display(),
            successor.display()
        );
        line.split(' ').map(str::to_owned).collect::<Vec<String>>()
    };
    let refused = run(<Cli as clap::Parser>::parse_from(argv("x1")));
    let Err(error) = refused else {
        return Err("x1 was not refused".into());
    };
    let shown = error.to_string();
    assert!(
        shown.starts_with("letter_authored") && shown.contains("x1"),
        "{shown}"
    );
    assert!(!successor.exists());
    let value = run(<Cli as clap::Parser>::parse_from(argv("a1 a2")))?;
    assert_eq!(value["command"], "handover");
    assert_eq!(value["report"]["inherited"], true);
    let file = PathBuf::from(value["report"]["file"].as_str().ok_or("no file")?);
    assert!(file.is_file());
    assert_eq!(
        file.file_stem().and_then(|s| s.to_str()),
        value["report"]["session"].as_str()
    );
    let cli = <Cli as clap::Parser>::parse_from(argv("a1"));
    assert!(matches!(cli.command, Command::Handover(_)));
    assert_eq!(cli.command.refusal_status(), 1);
    let missing = "lys-home handover --home h --from s --successor d".split(' ');
    let missing = <Cli as clap::Parser>::try_parse_from(missing);
    let Err(missing) = missing else {
        return Err("a handover without --letter was accepted".into());
    };
    assert!(missing.to_string().contains("--letter"), "{missing}");
    Ok(())
}

/// A render of the successor: its directory, report, records and the source part.
type Rendered = (TempDir, RenderReport, Vec<Value>, Value);

/// Render the fixture successor of `[a1, a2]` for `model`, returning the
/// report, the records written and the source `a1` thinking part.
fn render_successor(model: &str) -> Result<Rendered, Box<dyn Error>> {
    let (dir, root) = fixture()?;
    let successor = dir.path().join("successor");
    let report = handover(&root, OUTGOING, &ids(&["a1", "a2"]), &successor)?;
    let session = Home::open(&successor)?.open_session(&report.session)?;
    let target = target(model, &dir.path().join("rendered.jsonl"), None);
    let rendered = render_claude_code(&session, &target, None)?;
    let text = std::fs::read_to_string(&rendered.path)?;
    let records = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    let part = source_message(&root, "a1")?["content"][0].clone();
    Ok((dir, rendered, records, part))
}

#[test]
fn a_successor_rendered_for_another_model_carries_its_thinking_as_text() -> Gate {
    let (dir, report, records, part) = render_successor("claude-other-model")?;
    let counts = [
        report.thinking_as_text,
        report.thinking_kept,
        report.dropped,
    ];
    assert_eq!(counts, [1, 0, 1]);
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0]["message"]["content"],
        json!([{"type": "text", "text": "fixture-thinking-a1"}])
    );
    assert!(
        records
            .iter()
            .all(|r| !r.to_string().contains("\"signature\""))
    );
    let account: Value = serde_json::from_slice(&std::fs::read(&report.loss_path)?)?;
    let dropped = account["dropped"].as_array().ok_or("no dropped")?;
    assert_eq!(dropped.len(), 1);
    assert_eq!(
        dropped[0]["hash"],
        Hash::of(&serde_json::to_vec(&part)?).to_string()
    );
    assert_eq!(
        dropped[0]["reason"],
        "signed thinking rendered as text: different provider, api or model"
    );
    drop(dir);
    Ok(())
}

#[test]
fn a_successor_rendered_for_its_own_model_keeps_its_signed_thinking() -> Gate {
    let (_, report, records, _) = render_successor("claude-fixture-model")?;
    assert_eq!((report.thinking_kept, report.dropped), (1, 0));
    let part = &records[0]["message"]["content"][0];
    assert_eq!(
        (&part["type"], &part["signature"]),
        (&json!("thinking"), &json!("fixture-signature-a1"))
    );
    Ok(())
}
