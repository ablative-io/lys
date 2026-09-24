//! The home record rendered as a Claude Code JSONL.
//!
//! The context path is walked and each message entry becomes a Claude Code
//! record with the parent chain intact and the chosen session id. A thinking
//! block renders whole, signature included, only when its provider, api and
//! model equal the target's (Pi's rule, transform-messages.ts:95-109);
//! otherwise readable thinking becomes a text part and an opaque or redacted
//! block is dropped and named by hash in the loss account beside the file. A
//! compaction becomes Claude Code's `summary` record. Custom entries (the lys
//! ones included) and labels do not render. An existing target path is
//! refused by name and nothing is written.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::{API, PROVIDER, projects_slug};
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_AUTHORED, EntryBody};
use crate::record::{Session, fresh_id};

/// Where and for whom to render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderTarget {
    /// The session id the rendered file carries and Claude Code resumes by.
    pub session_id: String,
    /// The working directory the records name.
    pub cwd: String,
    /// The model the file is rendered for; decides whether thinking renders whole.
    pub model: String,
    /// The Claude Code version to write into records.
    pub version: String,
    /// Where to write; `None` means Claude Code's own place for `cwd`,
    /// `~/.claude/projects/<slug>/<session_id>.jsonl`.
    pub out: Option<PathBuf>,
}

/// One thing a render could not carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loss {
    /// The hash of the part as stored.
    pub hash: String,
    /// Why, naming no content.
    pub reason: String,
}

/// What a render reported: paths, counts and the loss account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderReport {
    /// The file written.
    pub path: PathBuf,
    /// The loss account written beside it.
    pub loss_path: PathBuf,
    /// Records written.
    pub records: u64,
    /// Thinking blocks rendered whole with their signature.
    pub thinking_kept: u64,
    /// Thinking blocks rendered as text.
    pub thinking_as_text: u64,
    /// Blocks dropped, each in the loss account.
    pub dropped: u64,
    /// Whether the session is a hand-authored demonstration.
    pub authored: bool,
}

/// The default place for a session file.
#[must_use]
pub fn default_path(home_dir: &Path, cwd: &str, session_id: &str) -> PathBuf {
    home_dir
        .join(".claude")
        .join("projects")
        .join(projects_slug(cwd))
        .join(format!("{session_id}.jsonl"))
}

/// Render the session's context path for Claude Code.
pub fn render_claude_code(
    session: &Session,
    target: &RenderTarget,
    user_home: &Path,
) -> Result<RenderReport, HomeError> {
    let path = target
        .out
        .clone()
        .unwrap_or_else(|| default_path(user_home, &target.cwd, &target.session_id));
    if path.exists() {
        return Err(HomeError::Exists { path });
    }
    let entries = session.context_path()?;
    let authored = entries.iter().any(|e| e.is_custom(CUSTOM_AUTHORED));
    let mut records: Vec<Value> = Vec::new();
    let mut losses: Vec<Loss> = Vec::new();
    let mut kept = 0u64;
    let mut as_text = 0u64;
    let mut prev: Option<String> = None;
    for entry in &entries {
        match &entry.body {
            EntryBody::Message { message } => {
                let uuid = record_uuid(entry.id());
                let role = message.get("role").and_then(Value::as_str).unwrap_or("");
                let ts = &entry.base.timestamp;
                let base = |kind: &str, msg: Value| {
                    json!({
                        "parentUuid": prev,
                        "isSidechain": false,
                        "userType": "external",
                        "cwd": target.cwd,
                        "sessionId": target.session_id,
                        "version": target.version,
                        "gitBranch": "",
                        "uuid": uuid,
                        "timestamp": ts,
                        "type": kind,
                        "message": msg,
                    })
                };
                match role {
                    "user" => {
                        let content = user_parts(message);
                        records.push(base("user", json!({"role": "user", "content": content})));
                    }
                    "toolResult" => {
                        let part = json!({
                            "type": "tool_result",
                            "tool_use_id": message.get("toolCallId").cloned().unwrap_or(Value::String(String::new())),
                            "content": message.get("content").cloned().unwrap_or(Value::Array(Vec::new())),
                            "is_error": message.get("isError").cloned().unwrap_or(Value::Bool(false)),
                        });
                        records.push(base("user", json!({"role": "user", "content": [part]})));
                    }
                    "assistant" => {
                        let same = message.get("provider").and_then(Value::as_str)
                            == Some(PROVIDER)
                            && message.get("api").and_then(Value::as_str) == Some(API)
                            && message.get("model").and_then(Value::as_str)
                                == Some(target.model.as_str());
                        let mut content = Vec::new();
                        for part in message
                            .get("content")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default()
                        {
                            match part.get("type").and_then(Value::as_str) {
                                Some("text") => content.push(json!({"type": "text", "text": part.get("text").cloned().unwrap_or(Value::String(String::new()))})),
                                Some("thinking") => {
                                    let redacted = part.get("redacted").and_then(Value::as_bool).unwrap_or(false);
                                    let sig = part.get("thinkingSignature").cloned();
                                    let text = part.get("thinking").and_then(Value::as_str).unwrap_or("");
                                    if same && redacted {
                                        content.push(json!({"type": "redacted_thinking", "data": sig.unwrap_or(Value::Null)}));
                                        kept += 1;
                                    } else if same && sig.is_some() {
                                        content.push(json!({"type": "thinking", "thinking": text, "signature": sig.unwrap_or(Value::Null)}));
                                        kept += 1;
                                    } else if !text.trim().is_empty() && !redacted {
                                        content.push(json!({"type": "text", "text": text}));
                                        as_text += 1;
                                        if sig.is_some() {
                                            losses.push(loss(&part, "signed thinking rendered as text: different provider, api or model"));
                                        }
                                    } else {
                                        losses.push(loss(&part, if redacted { "redacted thinking dropped: different provider, api or model" } else { "empty thinking dropped" }));
                                    }
                                }
                                Some("toolCall") => content.push(json!({
                                    "type": "tool_use",
                                    "id": part.get("id").cloned().unwrap_or(Value::String(String::new())),
                                    "name": part.get("name").cloned().unwrap_or(Value::String(String::new())),
                                    "input": part.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new())),
                                })),
                                _ => content.push(part.clone()),
                            }
                        }
                        let stop = match message.get("stopReason").and_then(Value::as_str) {
                            Some("toolUse") => "tool_use",
                            Some("length") => "max_tokens",
                            _ => "end_turn",
                        };
                        let msg = json!({
                            "id": format!("msg_{}", &uuid[..8]),
                            "type": "message",
                            "role": "assistant",
                            "model": message.get("model").cloned().unwrap_or(Value::String(String::new())),
                            "content": content,
                            "stop_reason": stop,
                            "stop_sequence": Value::Null,
                            "usage": {"input_tokens": 0, "output_tokens": 0},
                        });
                        records.push(base("assistant", msg));
                    }
                    _ => continue,
                }
                prev = Some(uuid);
            }
            EntryBody::Compaction { summary, .. } => {
                records.push(json!({"type": "summary", "summary": summary, "leafUuid": prev}));
            }
            _ => {}
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the render directory", dir, e))?;
    }
    {
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| HomeError::io("creating the rendered file", &path, e))?;
        for r in &records {
            let line = serde_json::to_string(r).map_err(|source| HomeError::Json {
                context: "a record could not be serialised",
                source,
            })?;
            f.write_all(line.as_bytes())
                .map_err(|e| HomeError::io("writing the rendered file", &path, e))?;
            f.write_all(b"\n")
                .map_err(|e| HomeError::io("writing the rendered file", &path, e))?;
        }
        f.sync_all()
            .map_err(|e| HomeError::io("syncing the rendered file", &path, e))?;
    }
    let loss_path = path.with_extension("loss.json");
    let account = json!({"session_id": target.session_id, "model": target.model, "authored": authored, "dropped": losses});
    std::fs::write(
        &loss_path,
        serde_json::to_vec_pretty(&account).unwrap_or_default(),
    )
    .map_err(|e| HomeError::io("writing the loss account", &loss_path, e))?;
    Ok(RenderReport {
        path,
        loss_path,
        records: records.len() as u64,
        thinking_kept: kept,
        thinking_as_text: as_text,
        dropped: losses.len() as u64,
        authored,
    })
}

fn loss(part: &Value, reason: &str) -> Loss {
    let bytes = serde_json::to_vec(part).unwrap_or_default();
    Loss {
        hash: Hash::of(&bytes).to_string(),
        reason: reason.to_owned(),
    }
}

/// Entry ids that already look like Claude Code uuids are kept; others get one.
fn record_uuid(id: &str) -> String {
    let uuid_shaped = id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        });
    if uuid_shaped {
        id.to_owned()
    } else {
        let h = fresh_id();
        format!(
            "{}-{}-4{}-8{}-{}",
            &h[..8],
            &h[8..12],
            &h[13..16],
            &h[17..20],
            &h[20..32]
        )
    }
}

fn user_parts(message: &Value) -> Value {
    match message.get("content") {
        Some(Value::String(s)) => Value::String(s.clone()),
        Some(Value::Array(parts)) => {
            if parts.len() == 1 && parts[0].get("type").and_then(Value::as_str) == Some("text") {
                parts[0]
                    .get("text")
                    .cloned()
                    .unwrap_or(Value::String(String::new()))
            } else {
                Value::Array(parts.clone())
            }
        }
        _ => Value::String(String::new()),
    }
}
