//! The content of an imported message: its usage, its stop reason, and its
//! parts stored as blocks, each with the hash the store returned, its index
//! in the source record's content and the message it went into, so the
//! importer can write its block row (HOME-030 R2).

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::error::HomeError;
use crate::record::blocks::{BlockStore, Hash};

use super::ImportReport;

/// One part stored as a block: its 0-based index in the source record's
/// content, the hash [`BlockStore::put`] returned, the block's length, and
/// the tool result message it went into (`None` for the record's own entry).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Stored {
    pub(super) part: u64,
    pub(super) hash: Hash,
    pub(super) len: u64,
    pub(super) result: Option<usize>,
}

/// A user record's content: its own parts, one `toolResult` message per
/// tool result, and every part as stored.
pub(super) struct UserContent {
    pub(super) own: Vec<Value>,
    pub(super) results: Vec<Value>,
    pub(super) stored: Vec<Stored>,
}

pub(super) fn usage(message: &Value) -> Value {
    let u = message.get("usage");
    let n = |k: &str| {
        u.and_then(|u| u.get(k))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    json!({
        "input": n("input_tokens"),
        "output": n("output_tokens"),
        "cacheRead": n("cache_read_input_tokens"),
        "cacheWrite": n("cache_creation_input_tokens"),
        "totalTokens": n("input_tokens") + n("output_tokens") + n("cache_read_input_tokens") + n("cache_creation_input_tokens"),
        "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}
    })
}

pub(super) fn stop_reason(message: &Value) -> &'static str {
    match message.get("stop_reason").and_then(Value::as_str) {
        Some("tool_use") => "toolUse",
        Some("max_tokens") => "length",
        _ => "stop",
    }
}

/// Store one part as a block and count it; returns the hash the store
/// returned and the block's length in bytes.
pub(super) fn store_part(
    part: &Value,
    blocks: &BlockStore,
    report: &mut ImportReport,
) -> Result<(Hash, u64), HomeError> {
    let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
        context: "a part could not be serialised",
        source,
    })?;
    let put = blocks.put(&bytes)?;
    report.blocks += 1;
    if put.new {
        report.blocks_new += 1;
    }
    Ok((put.hash, bytes.len() as u64))
}

/// Claude Code assistant parts into Pi's, storing each as a block.
pub(super) fn assistant_content(
    message: &Value,
    blocks: &BlockStore,
    tool_names: &mut HashMap<String, String>,
    report: &mut ImportReport,
) -> Result<(Vec<Value>, Vec<Stored>), HomeError> {
    let mut out = Vec::new();
    let mut stored = Vec::new();
    let text;
    let parts: &[Value] = match message.get("content") {
        Some(Value::Array(a)) => a,
        Some(Value::String(s)) => {
            text = [json!({"type": "text", "text": s})];
            &text
        }
        _ => &[],
    };
    for (index, part) in (0u64..).zip(parts) {
        let (hash, len) = store_part(part, blocks, report)?;
        stored.push(Stored {
            part: index,
            hash,
            len,
            result: None,
        });
        let kind = part.get("type").and_then(Value::as_str).unwrap_or("");
        let mapped = match kind {
            "text" => {
                json!({"type": "text", "text": part.get("text").cloned().unwrap_or(Value::String(String::new()))})
            }
            "thinking" => {
                let mut m = json!({"type": "thinking", "thinking": part.get("thinking").cloned().unwrap_or(Value::String(String::new()))});
                if let Some(sig) = part.get("signature") {
                    m["thinkingSignature"] = sig.clone();
                }
                m
            }
            "redacted_thinking" => {
                json!({"type": "thinking", "thinking": "", "thinkingSignature": part.get("data").cloned().unwrap_or(Value::Null), "redacted": true})
            }
            "tool_use" => {
                let id = part
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let name = part
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                tool_names.insert(id.clone(), name.clone());
                json!({"type": "toolCall", "id": id, "name": name, "arguments": part.get("input").cloned().unwrap_or(Value::Object(Map::new()))})
            }
            _ => part.clone(),
        };
        out.push(mapped);
    }
    Ok((out, stored))
}

/// Claude Code user parts into Pi's: the user's own parts, and one
/// `toolResult` message per tool result.
pub(super) fn user_content(
    message: &Value,
    blocks: &BlockStore,
    tool_names: &HashMap<String, String>,
    report: &mut ImportReport,
) -> Result<UserContent, HomeError> {
    let mut own = Vec::new();
    let mut results = Vec::new();
    let mut stored = Vec::new();
    let ms = message
        .get("timestamp")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    match message.get("content") {
        Some(Value::String(s)) => {
            let part = json!({"type": "text", "text": s});
            let (hash, len) = store_part(&part, blocks, report)?;
            stored.push(Stored {
                part: 0,
                hash,
                len,
                result: None,
            });
            own.push(part);
        }
        Some(Value::Array(parts)) => {
            for (index, part) in (0u64..).zip(parts) {
                let (hash, len) = store_part(part, blocks, report)?;
                let is_result = part.get("type").and_then(Value::as_str) == Some("tool_result");
                stored.push(Stored {
                    part: index,
                    hash,
                    len,
                    result: is_result.then_some(results.len()),
                });
                match part.get("type").and_then(Value::as_str) {
                    Some("tool_result") => {
                        let id = part.get("tool_use_id").and_then(Value::as_str).unwrap_or("").to_owned();
                        let content = match part.get("content") {
                            Some(Value::String(s)) => vec![json!({"type": "text", "text": s})],
                            Some(Value::Array(a)) => a.clone(),
                            _ => Vec::new(),
                        };
                        results.push(json!({
                            "role": "toolResult",
                            "toolCallId": id,
                            "toolName": tool_names.get(&id).cloned().unwrap_or_default(),
                            "content": content,
                            "isError": part.get("is_error").and_then(Value::as_bool).unwrap_or(false),
                            "timestamp": ms,
                        }));
                    }
                    Some("text") => own.push(json!({"type": "text", "text": part.get("text").cloned().unwrap_or(Value::String(String::new()))})),
                    _ => own.push(part.clone()),
                }
            }
        }
        _ => {}
    }
    Ok(UserContent {
        own,
        results,
        stored,
    })
}
