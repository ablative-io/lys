//! A seat enters through the proved child and is ready on its first ACP answer.
use super::{Prepared, SpawnPlan, Spawned};
use crate::RunnerError;
use crate::harness_control::seat_program::Pipe;
use crate::session::Sessions;
use serde_json::json;
use std::io::{BufReader, Read};

impl Sessions {
    pub(super) fn run_seat_program(plan: &SpawnPlan) -> Result<Prepared, RunnerError> {
        let launch = crate::Launch {
            session: plan.session.clone(),
            program: plan.program.clone(),
            arguments: plan.arguments.clone(),
            directory: plan.directory.clone(),
            environment: plan.environment.clone(),
            config: None,
            columns: plan.columns,
            rows: plan.rows,
            rotation: None,
            policy: None,
        };
        let mut spawned = crate::harness_control::process::spawn_seat(&launch, plan.generation)?;
        let leader = spawned.binding.leader.clone();
        let mut proof: serde_json::Value = serde_json::from_str(
            plan.environment
                .get(crate::harness_control::seat_program::PROOF_VARIABLE)
                .ok_or_else(|| {
                    RunnerError::refused(
                        "seat_program_record_missing",
                        "the launch has no checked digests",
                    )
                })?,
        )
        .map_err(|error| RunnerError::refused("seat_program_record_invalid", error.to_string()))?;
        proof["found_executable_sha256"] = json!(spawned.binding.harness.sha256);
        let pipe = Pipe::new(spawned.writer, plan.directory.clone(), proof);
        let mut reader = BufReader::new(spawned.reader);
        let ready = (|| {
            pipe.send(&json!({"jsonrpc":"2.0","id":"lys-initialize","method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"lys","version":env!("CARGO_PKG_VERSION")}}})).map_err(|error| RunnerError::refused("seat_program_answer_missing", format!("initialize could not be sent before the first ACP answer: {}", error.name())))?;
            let answer = crate::harness_control::process::frame(&mut reader).map_err(|error| {
                RunnerError::refused(
                    "seat_program_answer_missing",
                    format!("stdout ended before the first ACP answer: {}", error.name()),
                )
            })?;
            if answer["id"] != "lys-initialize"
                || answer["result"]["protocolVersion"] != 1
                || answer.get("error").is_some()
            {
                return Err(RunnerError::refused(
                    "acp_initialize_refused",
                    "the first adapter answer did not accept ACP protocol version 1",
                ));
            }
            pipe.answer.set(answer.clone()).map_err(|answer| {
                RunnerError::refused(
                    "seat_program_answer_repeated",
                    format!("initialize answered twice for request {}", answer["id"]),
                )
            })?;
            let mut first = serde_json::to_vec(&answer)
                .map_err(|error| RunnerError::refused("acp_frame_invalid", error.to_string()))?;
            first.push(b'\n');
            Ok(first)
        })();
        let first = match ready {
            Ok(first) => first,
            Err(error) => {
                pipe.close()?;
                let running = spawned
                    .child
                    .try_wait()
                    .map_err(|wait| {
                        RunnerError::refused("seat_program_cleanup_failed", wait.to_string())
                    })?
                    .is_none();
                if running && let Err(cleanup) = crate::pty::end_group(leader.pid) {
                    crate::error::said(&format!("seat_program_cleanup_failed: {cleanup}"));
                }
                let status = spawned.child.wait().map_err(|wait| {
                    RunnerError::refused("seat_program_cleanup_failed", wait.to_string())
                })?;
                return Err(RunnerError::refused(
                    &error.name(),
                    format!("{error}; seat_program_exit: {status}"),
                ));
            }
        };
        Ok(Prepared {
            leader: leader.clone(),
            spawned: Some(Spawned {
                reader: Box::new(std::io::Cursor::new(first).chain(reader)),
                writer: crate::input::Input::seat(pipe),
                master: None,
                child: spawned.child,
                pid: leader.pid,
                control: None,
            }),
        })
    }
}
