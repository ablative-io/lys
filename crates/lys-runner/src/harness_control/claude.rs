//! Claude stream envelopes are correlated before they change a boundary.

use serde_json::{Value, json};

use super::{Boundary, Controller, Kind, Observation, Pending, Update};
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
        Some("user" | "result" | "control_request" | "control_response") => false,
        _ => true,
    }
}

impl Controller {
    pub(super) fn observe_claude(&mut self, value: &Value) -> Result<Observation, RunnerError> {
        match observe(
            &self.binding.conversation,
            &self.binding.harness_version,
            value,
            self.current.as_ref(),
        ) {
            Ok(observation) => Ok(observation),
            Err(error) => {
                self.ready = false;
                self.closed = true;
                self.refusal = Some(error.to_string());
                Err(error)
            }
        }
    }

    pub(super) fn defer_claude_admission(&mut self, uuid: &str) -> bool {
        if self.initialized {
            return false;
        }
        if self
            .current
            .as_ref()
            .is_some_and(|current| current.uuid == uuid)
        {
            self.replayed = true;
        }
        true
    }

    pub(super) fn claude_unproved(&mut self) -> RunnerError {
        let error = RunnerError::refused(
            "control_adapter_unqualified",
            "the turn ended before its serving version was proved",
        );
        self.ready = false;
        self.closed = true;
        self.refusal = Some(error.to_string());
        error
    }

    pub(super) fn bind_claude(&mut self, frame: &Value) -> Result<Option<Update>, RunnerError> {
        if frame.get("type").and_then(Value::as_str) != Some("control_response") {
            return Ok(None);
        }
        let response = &frame["response"];
        let refusal = if response.get("request_id").and_then(Value::as_str)
            == Some(self.initialize.as_str())
        {
            match response.get("subtype").and_then(Value::as_str) {
                Some("success") => None,
                Some("error") => Some((
                    "control_initialize_refused",
                    "the harness refused initialization",
                )),
                _ => Some((
                    "control_protocol_unsupported",
                    "initialize response has no success or error outcome",
                )),
            }
        } else {
            Some((
                "control_correlation_unsupported",
                "initialize response has another request identity",
            ))
        };
        if let Some((name, words)) = refusal {
            self.ready = false;
            self.closed = true;
            let error = RunnerError::refused(name, words);
            self.refusal = Some(error.to_string());
            return Err(error);
        }
        if self.ready {
            return Ok(Some(Update::default()));
        }
        self.ready = true;
        self.boundary = Boundary::Idle;
        let mut update = Update::default();
        update.events.push(self.event("control_bound", None, None));
        self.request_boundary(&mut update)?;
        Ok(Some(update))
    }

    pub(super) fn claude_version(&mut self, compact: bool) -> Result<Update, RunnerError> {
        if !self.ready || self.current.is_none() {
            return Err(RunnerError::refused(
                "control_source_unbound",
                "init arrived before a bound turn",
            ));
        }
        self.compact = compact;
        let mut update = Update::default();
        if !self.initialized {
            self.initialized = true;
            update
                .events
                .push(self.event("control_initialized", None, None));
        }
        if self.replayed {
            let Some(uuid) = self.current.as_ref().map(|current| current.uuid.clone()) else {
                return Err(self.claude_unproved());
            };
            let admitted = self.apply(Observation::Admitted { uuid, turn: None })?;
            update.events.extend(admitted.events);
            update.receipts.extend(admitted.receipts);
            update.admissions.extend(admitted.admissions);
            update.dispatches.extend(admitted.dispatches);
            self.replayed = false;
        }
        Ok(update)
    }
}
