//! Project typed Codex rejections without inventing a policy-denial cause.
//! Only the session-owned app-server stream supplies these notifications;
//! notify text, command output and ordinary failures are never evidence.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::hex;

/// What native item was rejected; no command, file contents or output is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectedItem {
    /// A command execution was reported declined.
    Command,
    /// A file change was reported declined.
    FileChange,
}

/// The source of this evidence, distinct from a Lys judge or OS denial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Codex's typed item lifecycle event.
    CodexReported,
}

/// The pinned harness does not distinguish setup rejection from policy denial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    /// No structured authority identified why the item was declined.
    Unavailable,
}

/// A safe rejection projection. The transport owner must append it and its
/// source cursor together through the existing durable refusal store before
/// acknowledging the source. Producing this value is not a persistence receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredRejection")]
pub struct Rejection {
    source_id: String,
    source: Source,
    cause: Cause,
    item: RejectedItem,
    thread: String,
    turn: String,
    item_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRejection {
    source_id: String,
    source: Source,
    cause: Cause,
    item: RejectedItem,
    thread: String,
    turn: String,
    item_id: String,
}

fn source_id(thread: &str, turn: &str, id: &str, kind: RejectedItem) -> String {
    let identity = json!(["lys-codex-rejection/v1", thread, turn, id, kind]).to_string();
    hex(&Sha256::digest(identity.as_bytes()))
}

impl TryFrom<StoredRejection> for Rejection {
    type Error = RunnerError;

    fn try_from(stored: StoredRejection) -> Result<Self, Self::Error> {
        if [&stored.thread, &stored.turn, &stored.item_id]
            .into_iter()
            .any(|id| id.trim().is_empty())
            || source_id(&stored.thread, &stored.turn, &stored.item_id, stored.item)
                != stored.source_id
        {
            return Err(RunnerError::refused(
                "codex_rejection_record_invalid",
                "stored rejection identity does not match its native item",
            ));
        }
        Ok(Self {
            source_id: stored.source_id,
            source: stored.source,
            cause: stored.cause,
            item: stored.item,
            thread: stored.thread,
            turn: stored.turn,
            item_id: stored.item_id,
        })
    }
}

impl Rejection {
    /// Stable across replay of the same thread, turn and item. Domain and
    /// item kind participate, preventing unrelated evidence from collapsing.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Native thread identity, for comparison with the durable event's source.
    pub fn thread_id(&self) -> &str {
        &self.thread
    }

    /// Plain, safe wording; it never says a person or policy denied the act.
    pub fn summary(&self) -> &'static str {
        match self.item {
            RejectedItem::Command => "Codex rejected a command; the cause is unavailable.",
            RejectedItem::FileChange => "Codex rejected a file change; the cause is unavailable.",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Status {
    InProgress,
    Completed,
    Failed,
    Declined,
}

fn malformed(field: &str) -> RunnerError {
    RunnerError::refused(
        "codex_policy_contract_unsupported",
        format!("native item/completed has no supported {field}"),
    )
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, RunnerError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| malformed(field))
}

/// Read one native app-server notification from the admitted session's
/// transport. Unrelated events answer None. Known command/file events with
/// unknown status refuse instead of silently erasing a possible rejection.
/// `thread` is bound by the transport owner, never read from hook stdin.
pub fn rejection(thread: &str, notification: &Value) -> Result<Option<Rejection>, RunnerError> {
    if notification.get("method").and_then(Value::as_str) != Some("item/completed") {
        return Ok(None);
    }
    if thread.is_empty() {
        return Err(malformed("admitted thread binding"));
    }
    let params = notification
        .get("params")
        .ok_or_else(|| malformed("params"))?;
    let observed_thread = text(params, "threadId")?;
    if observed_thread != thread {
        return Err(RunnerError::refused(
            "codex_event_session_mismatch",
            "native item/completed belongs to another thread",
        ));
    }
    let item = params.get("item").ok_or_else(|| malformed("item"))?;
    let kind = match text(item, "type")? {
        "commandExecution" => RejectedItem::Command,
        "fileChange" => RejectedItem::FileChange,
        _ => return Ok(None),
    };
    let status: Status = serde_json::from_value(
        item.get("status")
            .cloned()
            .ok_or_else(|| malformed("status"))?,
    )
    .map_err(|error| malformed(&format!("status ({:?})", error.classify())))?;
    match status {
        Status::Completed | Status::Failed => return Ok(None),
        Status::InProgress => return Err(malformed("terminal status")),
        Status::Declined => {}
    }
    let turn = text(params, "turnId")?;
    let id = text(item, "id")?;
    Ok(Some(Rejection {
        source_id: source_id(thread, turn, id, kind),
        source: Source::CodexReported,
        cause: Cause::Unavailable,
        item: kind,
        thread: thread.to_owned(),
        turn: turn.to_owned(),
        item_id: id.to_owned(),
    }))
}
