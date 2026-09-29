//! Claude stream-json projections for the single owned input pipe. The launch
//! owner must independently verify the selected executable and replay contract.
//! These codecs neither launch a harness nor establish that live capability.
//! Native capability and compaction semantics: code.claude.com/docs/en/agent-sdk/slash-commands.

use serde_json::{Value, json};

use super::events::Boundary;
use crate::error::RunnerError;

fn refuse(reason: &str) -> RunnerError {
    RunnerError::refused("control_protocol_unsupported", reason)
}

fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(n, byte)| {
            if matches!(n, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

/// Capabilities read from this bound session's actual system/init event.
pub struct Capabilities {
    session: String,
    compact: bool,
}

impl Capabilities {
    /// Inspect the native init, without assuming commands from a source tree.
    pub fn from_init(session: &str, frame: &Value) -> Result<Self, RunnerError> {
        if !uuid(session)
            || frame.get("session_id").and_then(Value::as_str) != Some(session)
            || frame.get("type").and_then(Value::as_str) != Some("system")
            || frame.get("subtype").and_then(Value::as_str) != Some("init")
        {
            return Err(refuse("system/init does not identify the bound session"));
        }
        let commands = frame
            .get("slash_commands")
            .and_then(Value::as_array)
            .ok_or_else(|| refuse("native init has no command capability list"))?;
        if commands.iter().any(|command| !command.is_string()) {
            return Err(refuse("native command capability list is unsupported"));
        }
        Ok(Self {
            session: session.to_owned(),
            compact: commands
                .iter()
                .any(|command| command.as_str() == Some("compact")),
        })
    }
}

/// A receipt observation, not an assertion that the saved goal was satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    /// The native replay named the submitted UUID.
    Admitted {
        /// The native replay UUID, kept for durable correlation.
        input: String,
    },
    /// The result closes this single-flight turn. A successful result without
    /// a compact boundary is explicitly not compacted.
    Finished {
        /// The result's native success subtype; no model text is inspected.
        success: bool,
        /// An actual compact boundary preceded successful termination.
        compacted: bool,
        /// Native result UUID; saved separately from the input UUID.
        result: String,
        /// The actual manual boundary UUID, when observed.
        boundary: Option<String>,
    },
}

/// One envelope and its observations within the sole dispatcher's single
/// flight. A new instance on reconnect is not permission to resend it.
pub struct Request {
    session: String,
    id: String,
    compact: bool,
    frame: Value,
    admitted: bool,
    compact_boundary: Option<String>,
    finished: bool,
}

impl Request {
    fn prepare(
        capabilities: &Capabilities,
        id: &str,
        boundary: &Boundary,
        words: Option<&str>,
    ) -> Result<Self, RunnerError> {
        if !uuid(id) || boundary != &Boundary::Idle {
            return Err(refuse(
                "a native UUID and proved idle boundary are required",
            ));
        }
        if words.is_none() && !capabilities.compact {
            return Err(refuse("the bound harness does not advertise compact"));
        }
        let content = words.map_or_else(
            || "/compact".to_owned(),
            |words| format!("Lys reminder\n{words}"),
        );
        let frame = json!({"type":"user","uuid":id,"session_id":capabilities.session,
            "parent_tool_use_id":null,"message":{"role":"user","content":content}});
        Ok(Self {
            session: capabilities.session.clone(),
            id: id.to_owned(),
            compact: words.is_none(),
            frame,
            admitted: false,
            compact_boundary: None,
            finished: false,
        })
    }

    /// Encode saved words after the fixed data label, never as a slash command.
    pub fn reminder(
        capabilities: &Capabilities,
        id: &str,
        boundary: &Boundary,
        words: &str,
    ) -> Result<Self, RunnerError> {
        Self::prepare(capabilities, id, boundary, Some(words))
    }

    /// Encode the exact native compact command after capability discovery.
    pub fn compact(
        capabilities: &Capabilities,
        id: &str,
        boundary: &Boundary,
    ) -> Result<Self, RunnerError> {
        Self::prepare(capabilities, id, boundary, None)
    }

    /// The exact JSON data for the sole writer and existing durable operation.
    pub fn frame(&self) -> &Value {
        &self.frame
    }

    /// Observe only the bound session. The dispatcher must serialize all input
    /// and route approval requests to the existing policy owner separately.
    pub fn observe(&mut self, frame: &Value) -> Result<Option<Observation>, RunnerError> {
        let kind = frame.get("type").and_then(Value::as_str).unwrap_or("");
        if kind == "control_request" {
            return Err(RunnerError::refused(
                "control_policy_answer_required",
                "native control request needs the existing policy authority",
            ));
        }
        if !matches!(
            kind,
            "user" | "system" | "result" | "assistant" | "stream_event"
        ) {
            return Err(refuse("unsupported native message type"));
        }
        if frame.get("session_id").and_then(Value::as_str) != Some(self.session.as_str()) {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "native message belongs to another session",
            ));
        }
        if self.finished {
            return Err(refuse(
                "a completed flight requires durable readback, not more observations",
            ));
        }
        if kind == "user" {
            if frame.get("uuid").and_then(Value::as_str) != Some(self.id.as_str()) {
                return Err(RunnerError::refused(
                    "control_source_mismatch",
                    "native replay names another input UUID",
                ));
            }
            if frame
                .get("message")
                .and_then(|message| message.get("role"))
                .and_then(Value::as_str)
                != Some("user")
                || frame
                    .get("parent_tool_use_id")
                    .is_some_and(|parent| !parent.is_null())
            {
                return Err(refuse("native replay is not a top-level user envelope"));
            }
            self.admitted = true;
            return Ok(Some(Observation::Admitted {
                input: self.id.clone(),
            }));
        }
        if kind == "system"
            && frame.get("subtype").and_then(Value::as_str) == Some("compact_boundary")
        {
            if !self.admitted || !self.compact {
                return Err(refuse("compact boundary has no admitted compact flight"));
            }
            let id = frame
                .get("uuid")
                .and_then(Value::as_str)
                .filter(|id| uuid(id))
                .ok_or_else(|| refuse("compact boundary has no native event UUID"))?;
            if frame
                .get("compact_metadata")
                .and_then(|metadata| metadata.get("trigger"))
                .and_then(Value::as_str)
                != Some("manual")
            {
                return Err(refuse(
                    "an automatic compact boundary cannot confirm an explicit compact request",
                ));
            }
            if self
                .compact_boundary
                .as_deref()
                .is_some_and(|prior| prior != id)
            {
                return Err(refuse("multiple compact boundaries require reconciliation"));
            }
            self.compact_boundary = Some(id.to_owned());
        }
        if kind != "result" {
            return Ok(None);
        }
        if !self.admitted {
            return Err(refuse("result has no matching admitted input"));
        }
        let success = match frame.get("subtype").and_then(Value::as_str) {
            Some("success") => true,
            Some(
                "error_during_execution"
                | "error_max_turns"
                | "error_max_budget_usd"
                | "error_max_structured_output_retries",
            ) => false,
            _ => return Err(refuse("unsupported native terminal result")),
        };
        let result = frame
            .get("uuid")
            .and_then(Value::as_str)
            .filter(|id| uuid(id))
            .ok_or_else(|| refuse("terminal result has no native event UUID"))?;
        self.finished = true;
        Ok(Some(Observation::Finished {
            success,
            compacted: success && self.compact_boundary.is_some(),
            result: result.to_owned(),
            boundary: self.compact_boundary.clone(),
        }))
    }
}
