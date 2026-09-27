//! The rollout (HOME-009 R4): a home session's context path written as the
//! JSONL Codex 0.156.0 keeps for a thread, at Codex's own layout under
//! `--out`, with the loss account beside it.
//!
//! [`translate`] is the one entry point. It runs the checks of
//! [`super::zone`], parses every context-path stamp as RFC 3339 and refuses
//! an existing target, all before any file is written. The thread id is
//! `record_uuid` of the session and its head, so each head makes a new
//! thread; the file is `sessions/YYYY/MM/DD/rollout-YYYY-MM-DDTHH-MM-SS-<thread>.jsonl`
//! with the head's own stamp placed in the zone given, and the account the
//! same path ending `.loss.json`. Line 0 is `session_meta` of five keys, line
//! 1 the marker saying the thread is a fork of the session and not that
//! session, then each context-path entry in order. Every line's outer
//! `timestamp` is an entry's stamp exactly as that entry records it. No clock,
//! random source or network is read, nothing is written outside `--out`, and
//! Codex's thread index is never written.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::render::record_uuid;
use crate::harness::codex::account::{Account, Rows, part_hash, write_new};
use crate::harness::codex::beside::{Beside, carried_prompt};
use crate::harness::codex::parts::{NO_TEXT, kind_of, message_items, not_carried};
use crate::harness::codex::zone::{check_version, local_time, parse_stamp, zone_of};
use crate::record::Session;
use crate::record::entries::{Entry, EntryBody};

/// Why an on-path entry of no conversation kind is lost.
pub const NOT_CONVERSATION: &str = "Codex has no item for it and it is not conversation";

/// What a translation wrote, as paths and counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Translation {
    /// The rollout.
    pub rollout: PathBuf,
    /// The loss account beside it.
    pub account: PathBuf,
    /// The Codex thread id.
    pub thread: String,
    /// Entries walked on the context path.
    pub entries: u64,
    /// Content parts walked.
    pub blocks: u64,
    /// Kept rows.
    pub kept: u64,
    /// Changed rows.
    pub changed: u64,
    /// Lost rows.
    pub lost: u64,
}

/// One rollout line, in the order Codex writes its keys.
#[derive(Serialize)]
struct Line<'a> {
    timestamp: &'a str,
    ordinal: usize,
    #[serde(rename = "type")]
    kind: &'static str,
    payload: Value,
}

/// The rollout's lines as they are built.
#[derive(Default)]
pub(crate) struct Lines {
    text: String,
    count: usize,
}

impl Lines {
    /// Add one line whose outer stamp is `stamp`.
    fn push(&mut self, stamp: &str, kind: &'static str, payload: Value) -> Result<(), HomeError> {
        let line = Line {
            timestamp: stamp,
            ordinal: self.count,
            kind,
            payload,
        };
        let text = serde_json::to_string(&line).map_err(|source| HomeError::Json {
            context: "a rollout line could not be serialised",
            source,
        })?;
        self.text.push_str(&text);
        self.text.push('\n');
        self.count += 1;
        Ok(())
    }

    /// Add one response item.
    pub(crate) fn item(&mut self, stamp: &str, item: Value) -> Result<(), HomeError> {
        self.push(stamp, "response_item", item)
    }

    /// Add one message of one `input_text` part.
    pub(crate) fn message(&mut self, stamp: &str, role: &str, text: &str) -> Result<(), HomeError> {
        self.item(
            stamp,
            json!({"type": "message", "role": role, "content": [{"type": "input_text", "text": text}]}),
        )
    }
}

/// An entry's kind as the account names it: a custom entry's custom type,
/// otherwise its type.
pub(crate) fn entry_kind(entry: &Entry) -> &str {
    match &entry.body {
        EntryBody::Message { .. } => "message",
        EntryBody::ModelChange { .. } => "model_change",
        EntryBody::ThinkingLevelChange { .. } => "thinking_level_change",
        EntryBody::Usage { .. } => "usage",
        EntryBody::Compaction { .. } => "compaction",
        EntryBody::BranchSummary { .. } => "branch_summary",
        EntryBody::Label { .. } => "label",
        EntryBody::SessionInfo { .. } => "session_info",
        EntryBody::Custom { custom_type, .. } => custom_type,
        EntryBody::CustomMessage { .. } => "custom_message",
        EntryBody::ContextEdit { .. } => "context_edit",
    }
}

/// Why a part is left out of marked text.
pub(crate) fn text_only(kind: &str, index: usize) -> String {
    format!("{kind} part {index} not carried: marked text holds text only")
}

/// Translate the session's context path into a Codex rollout under `out`.
pub fn translate(
    session: &mut Session,
    out: &Path,
    codex_version: &str,
    zone: Option<&str>,
) -> Result<Translation, HomeError> {
    check_version(codex_version)?;
    let zone = zone_of(zone)?;
    let (path, _) = session.path()?;
    let context = session.context_path()?;
    let head = path.last().ok_or(HomeError::BodyShape {
        api: "translate-codex",
        reason: "the session holds no entry to translate",
    })?;
    for entry in &context {
        parse_stamp(entry.id(), &entry.base.timestamp)?;
    }
    let head_stamp = head.base.timestamp.as_str();
    let local = local_time(head.id(), head_stamp, &zone)?;
    let session_id = session.header().id.clone();
    let thread = record_uuid(&session_id, head.id());
    let name = format!(
        "rollout-{}-{}.jsonl",
        local.strftime("%Y-%m-%dT%H-%M-%S"),
        thread
    );
    let rollout = out
        .join("sessions")
        .join(local.strftime("%Y").to_string())
        .join(local.strftime("%m").to_string())
        .join(local.strftime("%d").to_string())
        .join(name);
    let account_path = rollout.with_extension("loss.json");
    for target in [&rollout, &account_path] {
        if target.exists() {
            return Err(HomeError::TranslationTargetExists {
                path: target.clone(),
            });
        }
    }
    let head_hash = session.head_hash()?.to_string();
    let mut lines = Lines::default();
    let mut rows = Rows::default();
    let meta = json!({"id": thread, "session_id": thread, "timestamp": head_stamp,
        "cwd": session.header().cwd, "cli_version": codex_version});
    lines.push(head_stamp, "session_meta", meta)?;
    let marker = format!(
        "<TRANSLATED CONTEXT: THIS CODEX THREAD IS A FORK OF HOME SESSION {session_id} AT HEAD HASH {head_hash}, RENDERED FOR CODEX {codex_version}, NOT THAT SESSION>"
    );
    lines.message(head_stamp, "developer", &marker)?;
    let compaction = path
        .iter()
        .rev()
        .find(|entry| matches!(entry.body, EntryBody::Compaction { .. }));
    let beside = Beside::read(session, &path, &context)?;
    if let Some(compaction) = compaction {
        let reason = format!(
            "left off the context path by compaction {}",
            compaction.id()
        );
        for entry in path
            .iter()
            .filter(|e| !context.iter().any(|c| c.id() == e.id()))
        {
            rows.lost(entry.id(), None, entry_kind(entry), &reason);
            beside.left_behind(entry.id(), &reason, &mut rows);
        }
    }
    let mut blocks = 0u64;
    for entry in &context {
        blocks += walk_entry(entry, &mut lines, &mut rows)?;
        beside.after_entry(entry, &mut lines, &mut rows)?;
    }
    carried_prompt(session, &context, &mut lines, &mut rows)?;
    let account = Account {
        session: session_id,
        head: head.id().to_owned(),
        head_hash,
        thread: thread.clone(),
        codex_version: codex_version.to_owned(),
        rows,
    };
    let account_bytes = serde_json::to_vec_pretty(&account).map_err(|source| HomeError::Json {
        context: "the loss account could not be serialised",
        source,
    })?;
    if let Some(dir) = rollout.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the rollout directory", dir, e))?;
    }
    write_new(&rollout, lines.text.as_bytes())?;
    write_new(&account_path, &account_bytes)?;
    Ok(Translation {
        rollout,
        account: account_path,
        thread,
        entries: context.len() as u64,
        blocks,
        kept: account.rows.kept.len() as u64,
        changed: account.rows.changed.len() as u64,
        lost: account.rows.lost.len() as u64,
    })
}

/// One context-path entry's lines and rows; returns the content parts walked.
fn walk_entry(entry: &Entry, lines: &mut Lines, rows: &mut Rows) -> Result<u64, HomeError> {
    let stamp = entry.base.timestamp.as_str();
    let id = entry.id();
    match &entry.body {
        EntryBody::Message { message } => {
            for item in message_items(id, message, rows) {
                lines.item(stamp, item)?;
            }
            Ok(match message.get("content") {
                Some(Value::Array(parts)) => parts.len() as u64,
                Some(_) => 1,
                None => 0,
            })
        }
        EntryBody::Compaction { summary, .. } => {
            let text = format!("<COMPACTION SUMMARY {id}>\n{summary}");
            marked(entry, ("compaction", "COMPACTION SUMMARY", "summary"), rows)?;
            lines.message(stamp, "developer", &text)?;
            Ok(0)
        }
        EntryBody::BranchSummary { summary, .. } => {
            let text = format!("<BRANCH SUMMARY {id}>\n{summary}");
            marked(entry, ("branch_summary", "BRANCH SUMMARY", "summary"), rows)?;
            lines.message(stamp, "developer", &text)?;
            Ok(0)
        }
        EntryBody::CustomMessage { content, .. } => {
            let mut texts = Vec::new();
            let mut walked = 1;
            match content {
                Value::String(text) => texts.push(text.as_str()),
                Value::Array(parts) => {
                    walked = parts.len() as u64;
                    for (index, part) in parts.iter().enumerate() {
                        match (kind_of(part), part.get("text").and_then(Value::as_str)) {
                            ("text", Some(text)) => texts.push(text),
                            ("text", None) => {
                                rows.lost(id, Some(part_hash(part)), "text", NO_TEXT);
                            }
                            (kind, _) => {
                                rows.lost(id, Some(part_hash(part)), kind, &text_only(kind, index));
                            }
                        }
                    }
                }
                _ => walked = 0,
            }
            let text = format!("<CUSTOM MESSAGE {id}>\n{}", texts.join("\n"));
            marked(entry, ("custom_message", "CUSTOM MESSAGE", "content"), rows)?;
            lines.message(stamp, "developer", &text)?;
            Ok(walked)
        }
        _ => {
            rows.lost(id, None, entry_kind(entry), NOT_CONVERSATION);
            Ok(0)
        }
    }
}

/// The changed row of an entry carried as marked developer text, naming each
/// key of the entry beyond its frame and the one whose text is carried.
fn marked(
    entry: &Entry,
    (before, marker, carried): (&str, &str, &str),
    rows: &mut Rows,
) -> Result<(), HomeError> {
    let value = serde_json::to_value(entry).map_err(|source| HomeError::Json {
        context: "an entry could not be serialised",
        source,
    })?;
    let mut how = vec![format!(
        "carried as developer text under the {marker} marker"
    )];
    how.extend(not_carried(
        &value,
        &["type", "id", "parentId", "timestamp", carried],
    ));
    rows.changed(entry.id(), None, before, "marked text", &how.join("; "));
    Ok(())
}
