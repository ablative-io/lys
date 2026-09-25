//! Claude Code's harness-local records as `lys.harness_event` entries
//! (HOME-001 R8).
//!
//! A Claude Code file is one chain: `attachment` and `system` records carry a
//! `uuid` and a `parentUuid` like the messages do, and the next message names
//! whichever record came before it, message or not. So each such record
//! becomes an event entry at its exact place on the chain, under its own
//! `uuid`, and the file's tree stays the home's tree. Records without a uuid
//! (`permission-mode`) and events derived from parts (`tool_completed`, one per
//! tool result) hang under the entry they followed as side leaves.
//!
//! An event's data is `{kind, harness, source_uuid, record, detail}`: `record`
//! is the whole source record stored as a block, by hash, so nothing of the
//! original is lost; `detail` carries only names, ids, exit codes and counts,
//! never hook output or a tool result's body, and the serialised data is
//! refused when it would exceed [`MAX_DATA_BYTES`].

use serde_json::{Map, Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::HARNESS;
use crate::record::blocks::BlockStore;

/// A hook outcome (`attachment` of type `hook_success` or `hook_failure`).
pub const KIND_HOOK: &str = "hook";
/// A `permission-mode` record.
pub const KIND_PERMISSION_MODE: &str = "permission_mode";
/// One tool result part.
pub const KIND_TOOL_COMPLETED: &str = "tool_completed";
/// Any other `attachment` record.
pub const KIND_ATTACHMENT: &str = "attachment";
/// A `system` record.
pub const KIND_SYSTEM: &str = "system";
/// The most an event's serialised data may be.
pub const MAX_DATA_BYTES: usize = 512;

/// One harness event, before it becomes an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarnessEvent {
    /// Which kind of event.
    pub kind: String,
    /// The source record's uuid, when it had one.
    pub source_uuid: Option<String>,
    /// The hash of the whole source record, when it was stored.
    pub record: Option<String>,
    /// Names, ids, exit codes and counts.
    pub detail: Map<String, Value>,
}

impl HarnessEvent {
    /// The event's data as it rides in a `lys.harness_event` entry.
    pub fn data(&self) -> Result<Value, HomeError> {
        let data = json!({
            "kind": self.kind,
            "harness": HARNESS,
            "source_uuid": self.source_uuid,
            "record": self.record,
            "detail": self.detail,
        });
        let len = serde_json::to_vec(&data)
            .map_err(|source| HomeError::Json {
                context: "an event could not be serialised",
                source,
            })?
            .len();
        if len > MAX_DATA_BYTES {
            return Err(HomeError::EventTooLarge {
                uuid: self.source_uuid.clone().unwrap_or_default(),
                len,
            });
        }
        Ok(data)
    }
}

/// The event a non-message record maps to, storing the record as a block:
/// `attachment`, `system` and `permission-mode` records; `None` for a record
/// type R8 does not map.
pub fn event_of(record: &Value, blocks: &BlockStore) -> Result<Option<HarnessEvent>, HomeError> {
    let kind = record.get("type").and_then(Value::as_str).unwrap_or("");
    let (event_kind, detail) = match kind {
        "attachment" => {
            let attachment = record.get("attachment").unwrap_or(&Value::Null);
            let attachment_type = str_of(attachment, "type");
            let mut detail = Map::new();
            detail.insert("attachment_type".to_owned(), json!(attachment_type));
            if attachment_type == "hook_success" || attachment_type == "hook_failure" {
                copy_str(&mut detail, attachment, "hookName", "hook_name");
                copy_str(&mut detail, attachment, "hookEvent", "hook_event");
                copy_str(&mut detail, attachment, "toolUseID", "tool_use_id");
                copy_num(&mut detail, attachment, "exitCode", "exit_code");
                copy_num(&mut detail, attachment, "durationMs", "duration_ms");
                detail.insert(
                    "stdout_bytes".to_owned(),
                    json!(str_len(attachment, "stdout")),
                );
                detail.insert(
                    "stderr_bytes".to_owned(),
                    json!(str_len(attachment, "stderr")),
                );
                (KIND_HOOK, detail)
            } else {
                copy_num(&mut detail, attachment, "skillCount", "skill_count");
                copy_len(&mut detail, attachment, "names", "names");
                copy_len(&mut detail, attachment, "addedNames", "added");
                copy_len(&mut detail, attachment, "removedNames", "removed");
                copy_len(&mut detail, attachment, "entries", "entries");
                (KIND_ATTACHMENT, detail)
            }
        }
        "system" => {
            let mut detail = Map::new();
            copy_str(&mut detail, record, "subtype", "subtype");
            copy_str(&mut detail, record, "level", "level");
            copy_str(&mut detail, record, "toolUseID", "tool_use_id");
            copy_num(&mut detail, record, "durationMs", "duration_ms");
            copy_num(&mut detail, record, "messageCount", "message_count");
            copy_num(&mut detail, record, "hookCount", "hook_count");
            copy_bool(
                &mut detail,
                record,
                "preventedContinuation",
                "prevented_continuation",
            );
            (KIND_SYSTEM, detail)
        }
        "permission-mode" => {
            let mut detail = Map::new();
            copy_str(&mut detail, record, "permissionMode", "mode");
            (KIND_PERMISSION_MODE, detail)
        }
        _ => return Ok(None),
    };
    let bytes = serde_json::to_vec(record).map_err(|source| HomeError::Json {
        context: "a record could not be serialised",
        source,
    })?;
    let put = blocks.put(&bytes)?;
    Ok(Some(HarnessEvent {
        kind: event_kind.to_owned(),
        source_uuid: record
            .get("uuid")
            .and_then(Value::as_str)
            .map(str::to_owned),
        record: Some(put.hash.as_str().to_owned()),
        detail,
    }))
}

/// The event for one tool result part: the tool's id and name and whether it
/// errored, nothing of its body.
#[must_use]
pub fn tool_completed(
    tool_use_id: &str,
    tool_name: &str,
    is_error: bool,
    source_uuid: &str,
) -> HarnessEvent {
    let mut detail = Map::new();
    detail.insert("tool_use_id".to_owned(), json!(tool_use_id));
    detail.insert("tool_name".to_owned(), json!(tool_name));
    detail.insert("is_error".to_owned(), json!(is_error));
    HarnessEvent {
        kind: KIND_TOOL_COMPLETED.to_owned(),
        source_uuid: Some(source_uuid.to_owned()),
        record: None,
        detail,
    }
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn str_len(v: &Value, key: &str) -> usize {
    v.get(key).and_then(Value::as_str).map_or(0, str::len)
}

fn copy_str(detail: &mut Map<String, Value>, from: &Value, key: &str, as_name: &str) {
    if let Some(s) = from.get(key).and_then(Value::as_str) {
        detail.insert(as_name.to_owned(), json!(s));
    }
}

fn copy_num(detail: &mut Map<String, Value>, from: &Value, key: &str, as_name: &str) {
    if let Some(n) = from.get(key).filter(|v| v.is_number()) {
        detail.insert(as_name.to_owned(), n.clone());
    }
}

fn copy_bool(detail: &mut Map<String, Value>, from: &Value, key: &str, as_name: &str) {
    if let Some(b) = from.get(key).and_then(Value::as_bool) {
        detail.insert(as_name.to_owned(), json!(b));
    }
}

fn copy_len(detail: &mut Map<String, Value>, from: &Value, key: &str, as_name: &str) {
    if let Some(a) = from.get(key).and_then(Value::as_array) {
        detail.insert(as_name.to_owned(), json!(a.len()));
    }
}
