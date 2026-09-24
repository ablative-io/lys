//! Claude Code JSONL into the home record.
//!
//! Each `user` or `assistant` record becomes a message entry in Pi's grammar
//! with the record's `uuid` as the entry id and its `parentUuid` as the
//! parent, so the file's tree is the home's tree. Tool results inside a user
//! record become Pi `toolResult` messages, one each. A `summary` record
//! becomes a compaction entry. A sidechain (a subagent's records) hangs under
//! the last main-path message before it with a label naming the agent. Every
//! content part is also stored as a block, so a proxy call that resends the
//! conversation adds no new blocks. Every other record type is counted and
//! left in the byte-for-byte original; R8 maps the harness events among them.
//!
//! An assistant record whose model is `authored` marks a hand-written
//! demonstration: it imports with provider, api and model `authored` behind
//! one `lys.authored` entry, so it is never mistaken for a model's turn.

use std::collections::{BTreeMap, HashMap};
use std::io::BufRead;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::{API, AUTHORED, PROVIDER};
use crate::record::blocks::BlockStore;
use crate::record::entries::{CUSTOM_AUTHORED, Entry, EntryBase, EntryBody};
use crate::record::{Session, fresh_id};

/// What an import reported: counts only, no text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    /// Lines in the source file.
    pub records: u64,
    /// Entries written.
    pub entries: u64,
    /// Content parts stored as blocks (new plus reused).
    pub blocks: u64,
    /// Blocks that were new to the store.
    pub blocks_new: u64,
    /// Source bytes read.
    pub bytes_in: u64,
    /// Bytes written to the session file.
    pub bytes_out: u64,
    /// Records by type that were counted and left in the original.
    pub counted_types: BTreeMap<String, u64>,
    /// Whether an authored turn was found.
    pub authored: bool,
    /// Sidechains attached, by agent id count.
    pub sidechains: u64,
}

/// Import one Claude Code JSONL into an open, empty session.
pub fn import_claude_code(
    source: &Path,
    session: &mut Session,
    blocks: &BlockStore,
) -> Result<ImportReport, HomeError> {
    let file = std::fs::File::open(source)
        .map_err(|e| HomeError::io("opening the transcript", source, e))?;
    let reader = std::io::BufReader::new(file);
    let mut report = ImportReport::default();
    let mut tool_names: HashMap<String, String> = HashMap::new();
    let mut last_main: Option<String> = None;
    let mut authored_marked = false;
    let start_len = session_len(session)?;
    for (n, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| HomeError::io("reading the transcript", source, e))?;
        report.records += 1;
        report.bytes_in += line.len() as u64 + 1;
        if line.trim().is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(&line).map_err(|e| HomeError::Malformed {
            path: source.to_path_buf(),
            line: n + 1,
            what: "Claude Code record",
            reason: e.to_string(),
        })?;
        let kind = record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_owned();
        match kind.as_str() {
            "user" | "assistant" => {
                let uuid = field(&record, "uuid", source, n)?;
                let parent = record
                    .get("parentUuid")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let timestamp = record
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let sidechain = record
                    .get("isSidechain")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let parent_id = match parent {
                    Some(p) => {
                        if !session.contains(&p) {
                            return Err(HomeError::UnknownParent {
                                id: uuid,
                                parent: p,
                            });
                        }
                        Some(p)
                    }
                    None if sidechain => {
                        // A subagent's first record: hang it under the last main-path message.
                        let under = last_main.clone();
                        if let Some(agent) = record.get("agentId").and_then(Value::as_str) {
                            let label = Entry {
                                base: EntryBase {
                                    id: fresh_id(),
                                    parent_id: under.clone(),
                                    timestamp: timestamp.clone(),
                                },
                                body: EntryBody::Label {
                                    target_id: uuid.clone(),
                                    label: Some(format!("agent {agent}")),
                                },
                            };
                            session.append_entry(&label)?;
                            report.entries += 1;
                            report.sidechains += 1;
                        }
                        under
                    }
                    None => None,
                };
                let Some(message) = record.get("message") else {
                    count(&mut report, &kind);
                    continue;
                };
                let ms = millis(&timestamp);
                let mut parent_for_next = parent_id.clone();
                if kind == "assistant" {
                    let model = message
                        .get("model")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned();
                    let authored = model == AUTHORED;
                    if authored {
                        report.authored = true;
                        if !authored_marked {
                            let mark = Entry {
                                base: EntryBase {
                                    id: fresh_id(),
                                    parent_id: parent_for_next.clone(),
                                    timestamp: timestamp.clone(),
                                },
                                body: EntryBody::Custom {
                                    custom_type: CUSTOM_AUTHORED.to_owned(),
                                    data: None,
                                },
                            };
                            session.append_entry(&mark)?;
                            report.entries += 1;
                            parent_for_next = Some(mark.base.id.clone());
                            authored_marked = true;
                        }
                    }
                    let (content, stored) =
                        assistant_content(message, blocks, &mut tool_names, &mut report)?;
                    let _ = stored;
                    let pi = json!({
                        "role": "assistant",
                        "content": content,
                        "api": if authored { AUTHORED } else { API },
                        "provider": if authored { AUTHORED } else { PROVIDER },
                        "model": model,
                        "usage": usage(message),
                        "stopReason": stop_reason(message),
                        "timestamp": ms,
                    });
                    let entry = Entry {
                        base: EntryBase {
                            id: uuid.clone(),
                            parent_id: parent_for_next,
                            timestamp,
                        },
                        body: EntryBody::Message { message: pi },
                    };
                    session.append_entry(&entry)?;
                    report.entries += 1;
                } else {
                    // A user record: tool results first (each its own message), then the user's own parts.
                    let (user_parts, results) =
                        user_content(message, blocks, &tool_names, &mut report)?;
                    let mut chain = parent_for_next;
                    let mut wrote_user = false;
                    for (i, result) in results.into_iter().enumerate() {
                        let id = if i == 0 && user_parts.is_empty() {
                            uuid.clone()
                        } else {
                            format!("{uuid}-r{i}")
                        };
                        let entry = Entry {
                            base: EntryBase {
                                id: id.clone(),
                                parent_id: chain.clone(),
                                timestamp: timestamp.clone(),
                            },
                            body: EntryBody::Message { message: result },
                        };
                        session.append_entry(&entry)?;
                        report.entries += 1;
                        chain = Some(id);
                        wrote_user = true;
                    }
                    if !user_parts.is_empty() || !wrote_user {
                        let pi = json!({"role": "user", "content": user_parts, "timestamp": ms});
                        let entry = Entry {
                            base: EntryBase {
                                id: uuid.clone(),
                                parent_id: chain,
                                timestamp,
                            },
                            body: EntryBody::Message { message: pi },
                        };
                        session.append_entry(&entry)?;
                        report.entries += 1;
                    }
                }
                if !sidechain {
                    last_main = Some(uuid);
                }
            }
            "summary" => {
                let summary = record
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let id = fresh_id();
                let entry = Entry {
                    base: EntryBase {
                        id: id.clone(),
                        parent_id: last_main.clone(),
                        timestamp: record
                            .get("timestamp")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                    },
                    body: EntryBody::Compaction {
                        summary,
                        first_kept_entry_id: id.clone(),
                        tokens_before: 0,
                        rest: Map::new(),
                    },
                };
                session.append_entry(&entry)?;
                report.entries += 1;
                last_main = Some(id);
            }
            other => count(&mut report, other),
        }
    }
    report.bytes_out = session_len(session)?.saturating_sub(start_len);
    Ok(report)
}

fn count(report: &mut ImportReport, kind: &str) {
    *report.counted_types.entry(kind.to_owned()).or_insert(0) += 1;
}

fn field(record: &Value, name: &str, source: &Path, n: usize) -> Result<String, HomeError> {
    record
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| HomeError::Malformed {
            path: source.to_path_buf(),
            line: n + 1,
            what: "Claude Code record",
            reason: format!("no {name}"),
        })
}

fn session_len(session: &Session) -> Result<u64, HomeError> {
    std::fs::metadata(session.file())
        .map(|m| m.len())
        .map_err(|e| HomeError::io("measuring the session file", session.file(), e))
}

fn millis(timestamp: &str) -> i64 {
    time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
        .map_or(0, |t| {
            i64::try_from(t.unix_timestamp_nanos() / 1_000_000).unwrap_or(0)
        })
}

fn usage(message: &Value) -> Value {
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

fn stop_reason(message: &Value) -> &'static str {
    match message.get("stop_reason").and_then(Value::as_str) {
        Some("tool_use") => "toolUse",
        Some("max_tokens") => "length",
        _ => "stop",
    }
}

fn store_part(
    part: &Value,
    blocks: &BlockStore,
    report: &mut ImportReport,
) -> Result<(), HomeError> {
    let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
        context: "a part could not be serialised",
        source,
    })?;
    let put = blocks.put(&bytes)?;
    report.blocks += 1;
    if put.new {
        report.blocks_new += 1;
    }
    Ok(())
}

/// Claude Code assistant parts into Pi's, storing each as a block.
fn assistant_content(
    message: &Value,
    blocks: &BlockStore,
    tool_names: &mut HashMap<String, String>,
    report: &mut ImportReport,
) -> Result<(Vec<Value>, u64), HomeError> {
    let mut out = Vec::new();
    let mut stored = 0u64;
    let parts: Vec<Value> = match message.get("content") {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::String(s)) => vec![json!({"type": "text", "text": s})],
        _ => Vec::new(),
    };
    for part in &parts {
        store_part(part, blocks, report)?;
        stored += 1;
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
fn user_content(
    message: &Value,
    blocks: &BlockStore,
    tool_names: &HashMap<String, String>,
    report: &mut ImportReport,
) -> Result<(Vec<Value>, Vec<Value>), HomeError> {
    let mut own = Vec::new();
    let mut results = Vec::new();
    let ms = message
        .get("timestamp")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    match message.get("content") {
        Some(Value::String(s)) => {
            let part = json!({"type": "text", "text": s});
            store_part(&part, blocks, report)?;
            own.push(part);
        }
        Some(Value::Array(parts)) => {
            for part in parts {
                store_part(part, blocks, report)?;
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
    Ok((own, results))
}
