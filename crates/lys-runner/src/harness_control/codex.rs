//! Codex app-server wire projection, grounded at the installed channels package
//! bd3798faee33820aad0044ed732841261d2bbe15. No model is invoked here.
//! The dispatcher owns handshake, actual thread binding, request serialization
//! and durable receipts. Builders require its proved idle boundary; they never
//! steer a turn or change approval, sandbox, credentials or model settings.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::events::{Boundary, Event, Kind, Source};
use crate::error::RunnerError;

fn unsupported(reason: &str) -> RunnerError {
    RunnerError::refused("control_protocol_unsupported", reason)
}

fn nonempty<'a>(value: &'a Value, field: &str) -> Result<&'a str, RunnerError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| unsupported("required native identity is absent"))
}

/// One prepared request. This is neither permission to write nor admission.
/// The existing operation owner must persist it before the sole pipe writes.
pub struct Request {
    id: String,
    thread: String,
    compact: bool,
    frame: Value,
}

impl Request {
    fn prepare(
        id: &str,
        thread: &str,
        boundary: &Boundary,
        text: Option<&str>,
    ) -> Result<Self, RunnerError> {
        if id.is_empty() || thread.is_empty() {
            return Err(unsupported("request and thread identities are required"));
        }
        if boundary != &Boundary::Idle {
            return Err(RunnerError::refused(
                "control_boundary_unknown",
                "a request requires the owned thread's current idle boundary",
            ));
        }
        let (method, params) = match text {
            Some(text) => (
                "turn/start",
                json!({"threadId":thread, "clientUserMessageId":id,
                "input":[{"type":"text", "text":text, "text_elements":[]}]}),
            ),
            None => ("thread/compact/start", json!({"threadId":thread})),
        };
        Ok(Self {
            id: id.to_owned(),
            thread: thread.to_owned(),
            compact: text.is_none(),
            frame: json!({"id":id, "method":method, "params":params}),
        })
    }

    /// Prepare typed text input, preserving saved words as data, not commands.
    pub fn reminder(
        id: &str,
        thread: &str,
        boundary: &Boundary,
        words: &str,
    ) -> Result<Self, RunnerError> {
        Self::prepare(id, thread, boundary, Some(words))
    }

    /// Prepare an explicit native compact request on the idle bound thread.
    pub fn compact(id: &str, thread: &str, boundary: &Boundary) -> Result<Self, RunnerError> {
        Self::prepare(id, thread, boundary, None)
    }

    /// Exact native JSON for the sole writer. No caller can add overrides here.
    pub fn frame(&self) -> &Value {
        &self.frame
    }

    /// Correlate the native response to this request. Empty compact success is
    /// admission alone; it never manufactures a turn or compaction evidence.
    pub fn admission(&self, source: &Source, response: &Value) -> Result<Event, RunnerError> {
        if source.conversation != self.thread
            || response.get("id") != Some(&Value::String(self.id.clone()))
        {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "response does not name the bound request and thread",
            ));
        }
        if response.get("error").is_some() {
            return Err(RunnerError::refused(
                "control_request_refused",
                "native request returned an error; no automatic resend",
            ));
        }
        let result = response
            .get("result")
            .ok_or_else(|| unsupported("native response has no result"))?;
        let turn = if self.compact {
            if result.as_object().is_none_or(|fields| !fields.is_empty()) {
                return Err(unsupported(
                    "compact response does not match the supported empty result",
                ));
            }
            None
        } else {
            Some(
                nonempty(
                    result
                        .get("turn")
                        .ok_or_else(|| unsupported("turn admission has no turn"))?,
                    "id",
                )?
                .to_owned(),
            )
        };
        Ok(event(
            source,
            Kind::Admitted {
                operation: self.id.clone(),
                turn,
            },
        ))
    }
}

fn event(source: &Source, kind: Kind) -> Event {
    // Hash only typed identity facts: no input, model text or credentials.
    let identity = json!(["lys-codex-control/v1", source, kind]).to_string();
    Event {
        source: source.clone(),
        source_id: crate::protocol::hex(&Sha256::digest(identity.as_bytes())),
        kind,
    }
}

/// Project lifecycle notifications from the sole owned reader. Approval
/// requests are returned to the policy owner by a named refusal, never answered
/// automatically. A tool result cannot become a terminal turn observation.
pub fn lifecycle(source: &Source, frame: &Value) -> Result<Option<Event>, RunnerError> {
    if frame.get("method").is_some() && frame.get("id").is_some() {
        return Err(RunnerError::refused(
            "control_policy_answer_required",
            "native server request requires the existing policy authority",
        ));
    }
    let Some(method) = frame.get("method").and_then(Value::as_str) else {
        return Ok(None);
    };
    if !matches!(method, "turn/started" | "turn/completed" | "item/completed") {
        return Ok(None);
    }
    let params = frame
        .get("params")
        .ok_or_else(|| unsupported("notification has no params"))?;
    if nonempty(params, "threadId")? != source.conversation {
        return Err(RunnerError::refused(
            "control_source_mismatch",
            "notification names another thread",
        ));
    }
    if method == "item/completed" {
        return crate::codex_refusals::rejection(&source.conversation, frame).map(|rejected| {
            rejected.map(|rejection| Event {
                source: source.clone(),
                source_id: rejection.source_id().to_owned(),
                kind: Kind::Rejected { rejection },
            })
        });
    }
    let turn = params
        .get("turn")
        .ok_or_else(|| unsupported("notification has no turn"))?;
    let id = nonempty(turn, "id")?.to_owned();
    let status = nonempty(turn, "status")?;
    let kind = match (method, status) {
        ("turn/started", "inProgress") => Kind::TurnStarted { turn: id },
        ("turn/completed", "completed" | "failed" | "interrupted") => {
            Kind::TurnCompleted { turn: id }
        }
        _ => {
            return Err(unsupported(
                "turn notification does not carry the expected native lifecycle state",
            ));
        }
    };
    Ok(Some(event(source, kind)))
}

/// Evidence for the compaction turn the dispatcher actually correlated after
/// its serialized request. Constructing this object does not establish that
/// correlation. An empty reply, other item or failed turn cannot complete it.
pub struct Compaction {
    source: Source,
    operation: String,
    turn: String,
    item: Option<String>,
    terminal: Option<bool>,
}

impl Compaction {
    /// Bind the operation to an observed native turn, never to a guessed id.
    pub fn new(source: Source, operation: String, turn: String) -> Result<Self, RunnerError> {
        if operation.is_empty() || turn.is_empty() {
            return Err(unsupported(
                "compaction requires operation and native turn identities",
            ));
        }
        Ok(Self {
            source,
            operation,
            turn,
            item: None,
            terminal: None,
        })
    }

    /// Accept only matching completed-item and terminal-turn evidence. The
    /// caller retains other frames for their normal lifecycle/policy owners.
    pub fn observe(&mut self, frame: &Value) -> Result<Option<Event>, RunnerError> {
        let method = frame.get("method").and_then(Value::as_str).unwrap_or("");
        if !matches!(method, "item/completed" | "turn/completed") {
            return Ok(None);
        }
        let params = frame
            .get("params")
            .ok_or_else(|| unsupported("notification has no params"))?;
        if nonempty(params, "threadId")? != self.source.conversation {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "compaction notification names another thread",
            ));
        }
        if method == "item/completed" {
            if nonempty(params, "turnId")? != self.turn {
                return Err(unsupported("compaction item belongs to another turn"));
            }
            let item = params
                .get("item")
                .ok_or_else(|| unsupported("completed notification has no item"))?;
            if item.get("type").and_then(Value::as_str) != Some("contextCompaction") {
                return Ok(None);
            }
            let id = nonempty(item, "id")?;
            if self.item.as_deref().is_some_and(|prior| prior != id) {
                return Err(unsupported(
                    "multiple compaction identities require reconciliation",
                ));
            }
            self.item = Some(id.to_owned());
        } else {
            let turn = params
                .get("turn")
                .ok_or_else(|| unsupported("terminal notification has no turn"))?;
            if nonempty(turn, "id")? != self.turn {
                return Err(unsupported("terminal event belongs to another turn"));
            }
            let completed = match nonempty(turn, "status")? {
                "completed" => true,
                "failed" | "interrupted" => false,
                _ => return Err(unsupported("turn is not terminal")),
            };
            if self.terminal.is_some_and(|prior| prior != completed) {
                return Err(unsupported(
                    "conflicting terminal evidence requires reconciliation",
                ));
            }
            self.terminal = Some(completed);
        }
        Ok(self
            .item
            .as_ref()
            .filter(|_| self.terminal == Some(true))
            .map(|item| {
                event(
                    &self.source,
                    Kind::Compacted {
                        operation: self.operation.clone(),
                        turn: self.turn.clone(),
                        item: item.clone(),
                    },
                )
            }))
    }
}
