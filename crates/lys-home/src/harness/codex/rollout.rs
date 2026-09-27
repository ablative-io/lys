//! A home session translated into a Codex 0.156.0 rollout, with its loss
//! account beside it and a side leaf on the session (HOME-009 R4 to R6).
//!
//! [`translate`] is the one entry point. It refuses an unmeasured version,
//! an unnamed or unknown zone, an entry stamp on the context path that is
//! not RFC 3339 and an existing rollout or account path, each by name and
//! before anything is written. Then it walks the context path in entry
//! order and part order and writes the rollout at Codex's own layout,
//! `<out>/sessions/YYYY/MM/DD/rollout-YYYY-MM-DDTHH-MM-SS-<thread>.jsonl`,
//! the date and time the head entry's own stamp in the given zone, and the
//! account beside it as `.loss.json`; then it appends one `lys.translation`
//! side leaf ([`super::leaf`]) that never moves the head.
//!
//! Invariants:
//!
//! - The thread id is `record_uuid` of the session head, so each head makes
//!   a new thread and the same head the same one. Nothing here reads a
//!   clock, a random source, the environment or the network, and no map's
//!   or set's order decides a line: the same head, target and version
//!   writes the same bytes.
//! - Every line is `{"timestamp", "ordinal", "type", "payload"}`, `ordinal`
//!   its 0-based line number, `type` only `session_meta` or
//!   `response_item`, and `timestamp` an entry's stamp copied as that entry
//!   records it, never reformatted or converted to the zone.
//! - `session_meta` holds exactly `id`, `session_id`, `timestamp`, `cwd` and
//!   `cli_version`. The first item is a developer message holding the
//!   in-band marker: the thread is a fork of the named session at the named
//!   head hash, rendered for the named Codex version, and not that session.
//!   No template hash takes part.
//! - Every entry of the root-to-head path, and every entry off it that
//!   descends from it, is carried or is a row in the account.
//! - Nothing is written outside `<out>`, and Codex's thread index is never
//!   written.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::render::record_uuid;
use crate::harness::codex::account::{Account, Rows, account_bytes, part_hash, write_new};
use crate::harness::codex::beside::{
    Beside, Item, MARKED_HOLDS, NOT_CONVERSATION, developer, forked_prompt, kind_of,
};
use crate::harness::codex::leaf::{TranslationData, append_leaf};
use crate::harness::codex::parts::{message_items, not_carried};
use crate::harness::codex::zone::{Local, check_version, parse_stamp, resolve_zone};
use crate::record::Session;
use crate::record::blocks::{Hash, sync_dir};
use crate::record::entries::{Entry, EntryBody};

/// What a translation wrote and counted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Translation {
    /// The rollout written.
    pub rollout: PathBuf,
    /// The loss account written beside it.
    pub account: PathBuf,
    /// The Codex thread id.
    pub thread: String,
    /// The side leaf's entry id.
    pub leaf: String,
    /// Entries walked on the context path.
    pub entries: u64,
    /// Content parts walked on the context path.
    pub blocks: u64,
    /// Kept rows.
    pub kept: u64,
    /// Changed rows.
    pub changed: u64,
    /// Lost rows.
    pub lost: u64,
}

/// One rollout line, in the key order Codex writes.
#[derive(Serialize)]
struct Line<'a> {
    timestamp: &'a str,
    ordinal: u64,
    #[serde(rename = "type")]
    kind: &'static str,
    payload: &'a Value,
}

/// The in-band marker every translated thread opens on.
#[must_use]
pub fn marker(session: &str, head_hash: &str, codex_version: &str) -> String {
    format!(
        "<TRANSLATED CONTEXT: THIS CODEX THREAD IS A FORK OF HOME SESSION {session} AT HEAD HASH {head_hash}, RENDERED FOR CODEX {codex_version}, NOT THAT SESSION>"
    )
}

/// A compaction, branch summary or custom message on the context path as
/// marked developer text, counted changed with every key not carried.
fn marked(
    entry: &Entry,
    heading: &str,
    body: &str,
    carried_key: &str,
    rows: &mut Rows,
) -> Result<Value, HomeError> {
    let value = serde_json::to_value(entry).map_err(|source| HomeError::Json {
        context: "an entry could not be serialised",
        source,
    })?;
    let first = format!("carried as developer text under the {heading} marker");
    let mut how = vec![first];
    let carried = ["type", "id", "parentId", "timestamp", carried_key];
    how.extend(not_carried(&value, &carried));
    rows.change(
        entry.id(),
        None,
        kind_of(entry),
        "marked text",
        &how.join("; "),
    );
    Ok(developer(&format!("<{heading} {}>\n{body}", entry.id())))
}

/// A custom message's text: its string content whole, or its text parts
/// joined by newlines, every other part lost by hash; `None` for content
/// that is neither.
fn custom_text(
    entry: &Entry,
    content: &Value,
    rows: &mut Rows,
) -> Result<Option<String>, HomeError> {
    match content {
        Value::String(text) => Ok(Some(text.clone())),
        Value::Array(parts) => {
            let mut texts = Vec::new();
            for (index, part) in parts.iter().enumerate() {
                let text = match part.get("type").and_then(Value::as_str) {
                    Some("text") => part.get("text").and_then(Value::as_str),
                    _ => None,
                };
                if let Some(text) = text {
                    texts.push(text);
                } else {
                    let kind = part
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("part with no type");
                    rows.lose(
                        entry.id(),
                        Some(part_hash(part)?),
                        kind,
                        &format!("{kind} part {index} not carried: {MARKED_HOLDS}"),
                    );
                }
            }
            Ok(Some(texts.join("\n")))
        }
        _ => Ok(None),
    }
}

/// How many content parts an entry holds.
fn parts_of(entry: &Entry) -> u64 {
    let content = match &entry.body {
        EntryBody::Message { message } => message.get("content"),
        EntryBody::CustomMessage { content, .. } => Some(content),
        _ => None,
    };
    match content {
        Some(Value::Array(parts)) => parts.len() as u64,
        Some(Value::String(_)) => 1,
        _ => 0,
    }
}

/// The items of one context-path entry, with its rows.
fn entry_items(entry: &Entry, rows: &mut Rows) -> Result<Vec<Value>, HomeError> {
    match &entry.body {
        EntryBody::Message { message } => message_items(entry.id(), message, rows),
        EntryBody::Compaction { summary, .. } => {
            let item = marked(entry, "COMPACTION SUMMARY", summary, "summary", rows)?;
            Ok(vec![item])
        }
        EntryBody::BranchSummary { summary, .. } => {
            let item = marked(entry, "BRANCH SUMMARY", summary, "summary", rows)?;
            Ok(vec![item])
        }
        EntryBody::CustomMessage { content, .. } => {
            let mut lost = Rows::default();
            let Some(text) = custom_text(entry, content, &mut lost)? else {
                rows.lose(
                    entry.id(),
                    Some(part_hash(content)?),
                    kind_of(entry),
                    "message content is neither a string nor a list of parts",
                );
                return Ok(Vec::new());
            };
            let item = marked(entry, "CUSTOM MESSAGE", &text, "content", rows)?;
            rows.extend(lost);
            Ok(vec![item])
        }
        _ => {
            rows.lose(entry.id(), None, kind_of(entry), NOT_CONVERSATION);
            Ok(Vec::new())
        }
    }
}

/// Where a rollout and its account go under `<out>`, and the rollout's path
/// relative to `<out>` with `/` separators.
fn target_paths(out: &Path, local: Local, thread: &str) -> (PathBuf, PathBuf, String) {
    let dirs = local.date_dirs();
    let name = format!("rollout-{}-{thread}", local.file_stamp());
    let relative = format!("sessions/{dirs}/{name}.jsonl");
    let mut dir = out.join("sessions");
    for part in dirs.split('/') {
        dir.push(part);
    }
    (
        dir.join(format!("{name}.jsonl")),
        dir.join(format!("{name}.loss.json")),
        relative,
    )
}

/// Translate the session's context path into a Codex rollout under `out`,
/// for `codex_version` with file names in `zone`, and record it beside the
/// context path.
pub fn translate(
    session: &mut Session,
    out: &Path,
    codex_version: &str,
    zone: Option<&str>,
) -> Result<Translation, HomeError> {
    check_version(codex_version)?;
    let zone = resolve_zone(zone)?;
    let context = session.context_path()?;
    for entry in &context {
        parse_stamp(entry.id(), &entry.base.timestamp)?;
    }
    let (path, _) = session.path()?;
    let Some(head) = path.last() else {
        return Err(HomeError::BodyShape {
            api: "translate-codex",
            reason: "the session has no head: there is nothing to translate",
        });
    };
    let head_stamp = head.base.timestamp.clone();
    let local = Local::of(parse_stamp(head.id(), &head_stamp)?, &zone);
    let session_id = session.header().id.clone();
    let thread = record_uuid(&session_id, head.id());
    let (rollout, account_path, relative) = target_paths(out, local, &thread);
    for target in [&rollout, &account_path] {
        if target.exists() {
            return Err(HomeError::TranslationTargetExists {
                path: target.clone(),
            });
        }
    }
    let head_hash = session.head_hash()?.to_string();
    let beside = Beside::read(session, &path, &context)?;
    let mut items = vec![
        Item {
            stamp: head_stamp.clone(),
            payload: json!({
                "id": thread,
                "session_id": thread,
                "timestamp": head_stamp,
                "cwd": session.header().cwd,
                "cli_version": codex_version,
            }),
        },
        Item {
            stamp: head_stamp,
            payload: developer(&marker(&session_id, &head_hash, codex_version)),
        },
    ];
    let mut rows = Rows::default();
    for (n, entry) in context.iter().enumerate() {
        for payload in entry_items(entry, &mut rows)? {
            items.push(Item {
                stamp: entry.base.timestamp.clone(),
                payload,
            });
        }
        if n == 0 {
            beside.left_off(&path, &mut rows);
        }
        beside.under(entry.id(), &mut items, &mut rows)?;
    }
    if let Some(prompt) = forked_prompt(session, &context)? {
        items.push(prompt.item);
        rows.extend(prompt.rows);
    }
    let mut bytes = Vec::new();
    for (ordinal, item) in (0u64..).zip(&items) {
        let line = Line {
            timestamp: &item.stamp,
            ordinal,
            kind: if ordinal == 0 {
                "session_meta"
            } else {
                "response_item"
            },
            payload: &item.payload,
        };
        let text = serde_json::to_string(&line).map_err(|source| HomeError::Json {
            context: "a rollout line could not be serialised",
            source,
        })?;
        bytes.extend_from_slice(text.as_bytes());
        bytes.push(b'\n');
    }
    let (kept, changed, lost) = (
        rows.kept.len() as u64,
        rows.changed.len() as u64,
        rows.lost.len() as u64,
    );
    let account = Account {
        session: session_id,
        head: head.id().to_owned(),
        head_hash: head_hash.clone(),
        thread: thread.clone(),
        codex_version: codex_version.to_owned(),
        kept: rows.kept,
        changed: rows.changed,
        lost: rows.lost,
    };
    let account_text = account_bytes(&account)?;
    let dir = rollout.parent().unwrap_or(out);
    std::fs::create_dir_all(dir)
        .map_err(|e| HomeError::io("creating the rollout directory", dir, e))?;
    write_new(&rollout, &bytes, "writing the rollout")?;
    write_new(&account_path, &account_text, "writing the loss account")?;
    sync_dir(dir)?;
    let data = TranslationData {
        harness: "codex".to_owned(),
        codex_version: codex_version.to_owned(),
        thread: thread.clone(),
        head: account.head,
        head_hash,
        rollout: relative,
        rollout_sha256: Hash::of(&bytes).to_string(),
        account_sha256: Hash::of(&account_text).to_string(),
    };
    let leaf = append_leaf(session, &data)?;
    Ok(Translation {
        rollout,
        account: account_path,
        thread,
        leaf,
        entries: context.len() as u64,
        blocks: context.iter().map(parts_of).sum(),
        kept,
        changed,
        lost,
    })
}
