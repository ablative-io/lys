//! Codex notify intake sends only completed turns through the bounded peer exchange.

use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::error::RunnerError;
use crate::hook_line::{exchange, report};
use crate::peer::{Collected, PeerAct, PeerRequest};
use crate::protocol::{Answer, PROTOCOL_VERSION};

/// The notify command's argument and captured process streams.
pub struct Args<'a> {
    /// The runner's Unix socket.
    pub socket: &'a Path,
    /// The notification JSON appended to the configured command.
    pub notification: &'a str,
    /// Standard output, kept empty so the harness proceeds.
    pub stdout: &'a mut dyn Write,
    /// Standard error, receiving one named diagnostic on failure.
    pub stderr: &'a mut dyn Write,
}

fn request(json: &str) -> Result<Option<PeerRequest>, RunnerError> {
    let notification: Value = serde_json::from_str(json).map_err(|error| {
        RunnerError::refused(
            "notify_input_malformed",
            format!("notify argument is not JSON ({:?})", error.classify()),
        )
    })?;
    if notification.get("type").and_then(Value::as_str) != Some("agent-turn-complete") {
        return Ok(None);
    }
    Ok(Some(PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Collect(Collected::Notify { notification }),
    }))
}

/// Hand a completed-turn notification to the runner, preserving its answer.
///
/// # Errors
/// Names malformed JSON, an unreachable runner, a refused request or an
/// exhausted three-second exchange budget. Other notification types return
/// `notify_ignored` without opening a connection.
pub fn hand(socket: &Path, json: &str) -> Result<Answer, RunnerError> {
    let request = request(json)?.ok_or_else(|| {
        RunnerError::refused(
            "notify_ignored",
            "notification type is not agent-turn-complete",
        )
    })?;
    exchange(socket, &request)
}

/// Hand one completed turn to its runner; ignore every other notification.
/// Every outcome exits zero with no standard output and at most one named
/// diagnostic on standard error.
pub fn run(args: Args<'_>) -> i32 {
    let handed = request(args.notification).and_then(|request| {
        request
            .as_ref()
            .map(|request| exchange(args.socket, request))
            .transpose()
    });
    if let Err(error) = handed {
        report(args.stderr, "notify", &error);
    }
    0
}
