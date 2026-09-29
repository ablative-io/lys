//! The Claude Code `PreToolUse` hook wire maps into the existing peer judge,
//! the same judge the Codex adapter asks; payload identities never prove a
//! session, and an answer never grants access.
//!
//! Claude Code adds members to its hook input from release to release, so a
//! member this adapter does not read is passed over rather than refused; the
//! members it does read are checked. The runner proves the asking process
//! from the socket, never from the claimed session. A hook the harness does
//! not run protects nothing: this adapter makes a run hook fail closed, not
//! an absent one.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::codex_judge::response;
use crate::codex_judge_client::exchange;
use crate::error::RunnerError;
use crate::peer::{PeerAct, PeerRequest};
use crate::protocol::PROTOCOL_VERSION;
use crate::refusals::{JudgeAsk, Verdict};

#[derive(Deserialize)]
struct NativeCall {
    session_id: String,
    cwd: String,
    hook_event_name: String,
    tool_name: String,
    tool_input: Value,
    tool_use_id: String,
    agent_id: Option<String>,
    agent_type: Option<String>,
}

fn malformed(field: &str) -> RunnerError {
    RunnerError::refused(
        "claude_hook_malformed",
        format!("Claude Code PreToolUse requires a valid {field}; tool input is not retained"),
    )
}

impl NativeCall {
    fn checked(self) -> Result<JudgeAsk, RunnerError> {
        if self.hook_event_name != "PreToolUse" {
            return Err(malformed("hook_event_name"));
        }
        for (field, value) in [
            ("session_id", self.session_id.as_str()),
            ("tool_use_id", self.tool_use_id.as_str()),
            ("tool_name", self.tool_name.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(malformed(field));
            }
        }
        if !Path::new(&self.cwd).is_absolute() {
            return Err(malformed("cwd"));
        }
        if !self.tool_input.is_object() {
            return Err(malformed("tool_input object"));
        }
        // A length-delimited JSON tuple keeps arbitrary ids from colliding.
        let attempt = json!([self.session_id, self.tool_use_id]).to_string();
        Ok(JudgeAsk {
            attempt: Some(attempt),
            tool_name: self.tool_name,
            tool_input: self.tool_input,
            claimed_session: Some(self.session_id),
            subagent: self.agent_id.is_some() || self.agent_type.is_some(),
        })
    }
}

/// Decode Claude Code's hook stdin into the peer protocol. A wrong event or
/// a missing member refuses without echoing the input.
pub fn request(stdin: &str) -> Result<PeerRequest, RunnerError> {
    let hook: NativeCall = serde_json::from_str(stdin)
        .map_err(|error| malformed(&format!("JSON schema ({:?})", error.classify())))?;
    Ok(PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Judge(hook.checked()?),
    })
}

/// Ask the runner on `socket` to judge the hook's stdin.
pub fn ask(socket: &Path, stdin: &str) -> Result<Verdict, RunnerError> {
    exchange(socket, &request(stdin)?)
}

/// The hook's output. Claude Code reads the same `hookSpecificOutput` shape
/// as Codex. Malformed input, a missing runner, an unproved caller or a
/// failed audit is a deny, never an empty answer.
pub fn output(socket: &Path, stdin: &str) -> Value {
    match ask(socket, stdin) {
        Ok(verdict) => response(&verdict),
        Err(error) => response(&Verdict::denied(
            "claude_judge_unavailable",
            format!("Lys could not complete this policy check: {error}"),
            "not_attributed",
        )),
    }
}
