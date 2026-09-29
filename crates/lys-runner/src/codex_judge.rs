//! The pinned Codex `PreToolUse` wire maps into the existing peer judge;
//! payload identities never prove a session, and an answer never grants access.
//!
//! Codex serializes shell calls as `Bash` already. Its `apply_patch` carries
//! patch text, not a proved file path: retaining that name lets the shared
//! judge refuse uninspectable effects under a path-restrictive policy.
//! This adapter alone does not make hooks fail closed: the pinned harness
//! may proceed on a disconnected or untrusted hook. Required protection must
//! be proved at launch by the native containment and capability owners.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::RunnerError;
use crate::peer::{PeerAct, PeerRequest};
use crate::protocol::PROTOCOL_VERSION;
use crate::refusals::{JudgeAsk, Verdict};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeCall {
    session_id: String,
    turn_id: String,
    agent_id: Option<String>,
    agent_type: Option<String>,
    transcript_path: Option<String>,
    cwd: String,
    hook_event_name: String,
    model: String,
    permission_mode: String,
    tool_name: String,
    tool_input: Value,
    tool_use_id: String,
}

fn malformed(field: &str) -> RunnerError {
    RunnerError::refused(
        "codex_hook_malformed",
        format!("Codex PreToolUse requires a valid {field}; tool input is not retained"),
    )
}

impl NativeCall {
    fn checked(self) -> Result<JudgeAsk, RunnerError> {
        if self.hook_event_name != "PreToolUse" {
            return Err(malformed("hook_event_name"));
        }
        for (field, value) in [
            ("session_id", self.session_id.as_str()),
            ("turn_id", self.turn_id.as_str()),
            ("tool_use_id", self.tool_use_id.as_str()),
            ("tool_name", self.tool_name.as_str()),
            ("model", self.model.as_str()),
            ("permission_mode", self.permission_mode.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(malformed(field));
            }
        }
        if !std::path::Path::new(&self.cwd).is_absolute() {
            return Err(malformed("cwd"));
        }
        if self.transcript_path.as_ref().is_some_and(String::is_empty) {
            return Err(malformed("transcript_path"));
        }
        if !self.tool_input.is_object() {
            return Err(malformed("tool_input object"));
        }
        match self.tool_name.as_str() {
            "Bash" | "apply_patch" => {
                if !self.tool_input.get("command").is_some_and(Value::is_string) {
                    return Err(malformed("tool_input.command"));
                }
            }
            // These are invocation names, not hook names in the pinned build.
            // In particular, write_stdin does not emit another PreToolUse.
            "exec_command" | "shell_command" | "write_stdin" => {
                return Err(malformed("native hook tool_name"));
            }
            _ => {}
        }
        // A length-delimited JSON tuple keeps arbitrary ids from colliding.
        let attempt = json!([self.session_id, self.turn_id, self.tool_use_id]).to_string();
        Ok(JudgeAsk {
            attempt: Some(attempt),
            tool_name: self.tool_name,
            tool_input: self.tool_input,
            claimed_session: Some(self.session_id),
            subagent: self.agent_id.is_some() || self.agent_type.is_some(),
        })
    }
}

/// Decode native hook stdin into the existing peer protocol. The runner
/// proves identity from the socket, never from the claimed session or cwd.
/// Unknown fields and wrong events refuse without echoing their contents.
pub fn request(stdin: &str) -> Result<PeerRequest, RunnerError> {
    let hook: NativeCall = serde_json::from_str(stdin)
        .map_err(|error| malformed(&format!("JSON schema ({:?})", error.classify())))?;
    Ok(PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Judge(hook.checked()?),
    })
}

/// Encode the shared judge's verdict in the native control-output shape.
/// A pass emits no permission decision. A denial always has a reason;
/// failed audit persistence remains a denial, never an allow or empty answer.
pub fn response(verdict: &Verdict) -> Value {
    if !verdict.deny {
        return json!({});
    }
    let reason = if verdict.reason.trim().is_empty() {
        "Lys refused this tool call; the judge returned no reason."
    } else {
        verdict.reason.as_str()
    };
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    })
}
