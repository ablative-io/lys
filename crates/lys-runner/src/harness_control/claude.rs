//! Claude stream envelopes are correlated before they change a boundary.

use serde_json::{Value, json};

use super::{Kind, Observation, Pending};
use crate::error::RunnerError;

/// Add the machine protocol flags without changing the existing settings.
pub fn arguments(given: &[String], conversation: &str) -> Result<Vec<String>, RunnerError> {
    for argument in given {
        let flag = argument.split('=').next().unwrap_or(argument);
        if matches!(
            flag,
            "--print"
                | "-p"
                | "--input-format"
                | "--output-format"
                | "--session-id"
                | "--resume"
                | "-r"
                | "--continue"
                | "-c"
                | "--replay-user-messages"
        ) {
            return Err(RunnerError::refused(
                "control_launch_invalid",
                "given arguments replace the managed conversation or transport",
            ));
        }
    }
    let mut arguments = given.to_vec();
    arguments.extend([
        "--print".to_owned(),
        "--input-format".to_owned(),
        "stream-json".to_owned(),
        "--output-format".to_owned(),
        "stream-json".to_owned(),
        "--verbose".to_owned(),
        "--replay-user-messages".to_owned(),
        "--session-id".to_owned(),
        conversation.to_owned(),
    ]);
    Ok(arguments)
}

pub(super) fn request(conversation: &str, pending: &Pending) -> Value {
    let text = if pending.kind == Kind::Compact {
        "/compact".to_owned()
    } else if pending.kind == Kind::Reminder {
        format!("Lys reminder\n{}", pending.text)
    } else {
        pending.text.clone()
    };
    json!({"type":"user", "uuid":pending.uuid, "session_id":conversation,
        "parent_tool_use_id":null, "message":{"role":"user", "content":text}})
}

pub(super) fn observe(
    conversation: &str,
    version: &str,
    value: &Value,
    pending: Option<&Pending>,
) -> Result<Observation, RunnerError> {
    if value.get("session_id").and_then(Value::as_str) != Some(conversation) {
        return Ok(Observation::Other);
    }
    let kind = value.get("type").and_then(Value::as_str);
    let subtype = value.get("subtype").and_then(Value::as_str);
    match (kind, subtype) {
        (Some("system"), Some("init")) => {
            if value.get("claude_code_version").and_then(Value::as_str) != Some(version) {
                return Err(RunnerError::refused(
                    "control_adapter_unqualified",
                    "Claude init version differs from its probed version or is absent",
                ));
            }
            let commands = value
                .get("slash_commands")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    RunnerError::refused(
                        "control_capability_unsupported",
                        "init has no command inventory",
                    )
                })?;
            Ok(Observation::Ready {
                compact: commands
                    .iter()
                    .any(|command| command.as_str() == Some("compact")),
            })
        }
        (Some("system"), Some("compact_boundary")) => Ok(Observation::Compacted { turn: None }),
        (Some("user"), _) => {
            let Some(uuid) = value
                .get("uuid")
                .and_then(Value::as_str)
                .filter(|uuid| pending.is_some_and(|pending| pending.uuid == *uuid))
            else {
                return Ok(Observation::Other);
            };
            if value.pointer("/message/role").and_then(Value::as_str) != Some("user")
                || value.get("parent_tool_use_id") != Some(&Value::Null)
            {
                return Err(RunnerError::refused(
                    "control_correlation_unsupported",
                    "user replay has no SDK envelope proof",
                ));
            }
            Ok(Observation::Admitted {
                uuid: uuid.to_owned(),
                turn: None,
            })
        }
        (Some("result"), _) => Ok(Observation::Completed {
            turn: None,
            failed: value
                .get("is_error")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    RunnerError::refused(
                        "control_protocol_unsupported",
                        "result has no terminal outcome",
                    )
                })?,
        }),
        _ => Ok(Observation::Other),
    }
}

pub(super) fn passive_frame(value: &Value) -> bool {
    match value.get("type").and_then(Value::as_str) {
        Some("system") => !matches!(
            value.get("subtype").and_then(Value::as_str),
            Some("init" | "compact_boundary")
        ),
        Some("user" | "result" | "control_request") => false,
        _ => true,
    }
}
