//! One event reader renders ACP output and answers unsupported requests by name.
use crate::RunnerError;
use crate::session::{Sessions, output::OutputHandle};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read};
use std::sync::Arc;

impl Sessions {
    pub(crate) fn seat_program_read(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        output: &OutputHandle,
        reader: Box<dyn Read + Send>,
    ) {
        let pipe = match self.lock().and_then(|table| {
            table
                .sessions
                .get(id)
                .and_then(|session| session.live.as_ref())
                .and_then(|live| live.writer.seat.clone())
                .ok_or_else(|| {
                    RunnerError::refused("acp_pipe_missing", "the seat has no kept stdin")
                })
        }) {
            Ok(pipe) => pipe,
            Err(error) => {
                crate::error::said(&error.to_string());
                return;
            }
        };
        let mut reader = BufReader::new(reader);
        loop {
            let mut line = String::new();
            let frame = (|| {
                if reader
                    .read_line(&mut line)
                    .map_err(|error| RunnerError::refused("acp_read_failed", error.to_string()))?
                    == 0
                {
                    return Ok(None);
                }
                if !line.ends_with('\n') {
                    return Err(RunnerError::refused(
                        "acp_frame_incomplete",
                        "the adapter ended before its frame terminator",
                    ));
                }
                let value: Value = serde_json::from_str(&line).map_err(|error| {
                    RunnerError::refused("acp_frame_invalid", error.to_string())
                })?;
                self.seat_attach_frame(id, generation, &value)?;
                output.push(generation, line.as_bytes())?;
                if value["id"] == "lys-initialize" {
                    pipe.send(&json!({"jsonrpc":"2.0", "id":"lys-seat-session", "method":"session/new", "params":{"cwd":pipe.directory,"mcpServers":[]}}))?;
                } else if value["id"] == "lys-seat-session" {
                    let session = value["result"]["sessionId"].as_str().ok_or_else(|| {
                        RunnerError::refused(
                            "acp_session_unbound",
                            "session/new answered no sessionId",
                        )
                    })?;
                    pipe.bind(session)?;
                } else if value.get("id").is_some() && value.get("method").is_some() {
                    pipe.send(&json!({"jsonrpc":"2.0", "id":value["id"], "error":{"code":-32601,"message":"acp_request_unsupported: this adapter does not authorise filesystem, terminal or permission requests"}}))?;
                } else if value.get("error").is_some() {
                    return Err(RunnerError::refused(
                        "acp_request_refused",
                        "the program refused an ACP request",
                    ));
                }
                Ok(Some(()))
            })();
            match frame {
                Ok(Some(())) => {
                    self.wake();
                }
                Ok(None) => break,
                Err(error) => {
                    crate::error::said(&format!("session {id}: {error}"));
                    if let Err(close) = pipe.close() {
                        crate::error::said(&format!("session {id}: acp_close_failed: {close}"));
                    }
                    break;
                }
            }
        }
    }
}
