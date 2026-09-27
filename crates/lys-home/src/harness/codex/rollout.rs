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
//! `timestamp` is an entry's stamp exactly as that entry records it. When
//! both files are written and synced, the `lys.translation` side leaf is
//! appended beside the context path ([`super::leaf`]); when writing the
//! account or appending the leaf fails, the files this run wrote are
//! removed, so a retry finds no target standing. The session file is read
//! once, through its own index, for the path, the context path and every
//! entry beside them. No clock,
//! random source or network is read, nothing is written outside `--out`, and
//! Codex's thread index is never written.

use crate::error::TranslateError;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::error::HomeError;
use crate::harness::claude_code::render::record_uuid;
use crate::harness::codex::account::{Account, Rows, part_hash, remove_written, write_new};
use crate::harness::codex::beside::Beside;
use crate::harness::codex::leaf::{HARNESS, TranslationData, append_leaf};
use crate::harness::codex::parts::{Item, NO_TEXT, Part, items_of, kind_of};
use crate::harness::codex::zone::{check_version, in_zone, parse_stamp, zone_of};
use crate::record::Session;
use crate::record::blocks::Hash;
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
struct Line<'a, P> {
    timestamp: &'a str,
    ordinal: usize,
    #[serde(rename = "type")]
    kind: &'static str,
    payload: P,
}

/// The `session_meta` payload, its keys in sorted order.
#[derive(Serialize)]
struct Meta<'a> {
    cli_version: &'a str,
    cwd: &'a str,
    id: &'a str,
    session_id: &'a str,
    timestamp: &'a str,
}

/// The rollout's lines as they are built, serialised straight into one
/// buffer.
#[derive(Default)]
pub(crate) struct Lines {
    text: Vec<u8>,
    count: usize,
}

impl Lines {
    /// Add one line whose outer stamp is `stamp`.
    fn push<P: Serialize>(
        &mut self,
        stamp: &str,
        kind: &'static str,
        payload: P,
    ) -> Result<(), HomeError> {
        let line = Line {
            timestamp: stamp,
            ordinal: self.count,
            kind,
            payload,
        };
        serde_json::to_writer(&mut self.text, &line).map_err(|source| HomeError::Json {
            context: "a rollout line could not be serialised",
            source,
        })?;
        self.text.push(b'\n');
        self.count += 1;
        Ok(())
    }

    /// Add one response item.
    pub(crate) fn item(&mut self, stamp: &str, item: &Item<'_>) -> Result<(), HomeError> {
        self.push(stamp, "response_item", item)
    }

    /// Add one message of one `input_text` part.
    pub(crate) fn message(&mut self, stamp: &str, role: &str, text: &str) -> Result<(), HomeError> {
        let content = vec![Part::Text {
            text,
            kind: "input_text",
        }];
        self.item(stamp, &Item::message(role, content))
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
    let beside = Beside::read(session)?;
    let head = beside.path().next_back().ok_or(HomeError::BodyShape {
        api: "translate-codex",
        reason: "the session holds no entry to translate",
    })?;
    let mut head_instant = None;
    for entry in beside.context() {
        let instant = parse_stamp(entry.id(), &entry.base.timestamp)?;
        if std::ptr::eq(entry, head) {
            head_instant = Some(instant);
        }
    }
    let head_stamp = head.base.timestamp.as_str();
    let head_instant = match head_instant {
        Some(instant) => instant,
        None => parse_stamp(head.id(), head_stamp)?,
    };
    let local = in_zone(head.id(), head_instant, &zone)?;
    let session_id = session.header().id.as_str();
    let thread = record_uuid(session_id, head.id());
    let name = format!(
        "rollout-{}-{}.jsonl",
        local.strftime("%Y-%m-%dT%H-%M-%S"),
        thread
    );
    let relative = format!("sessions/{}/{name}", local.strftime("%Y/%m/%d"));
    let rollout = relative
        .split('/')
        .fold(out.to_path_buf(), |at, part| at.join(part));
    let account_path = rollout.with_extension("loss.json");
    for target in [&rollout, &account_path] {
        if target.exists() {
            return Err(HomeError::Translate(
                TranslateError::TranslationTargetExists {
                    path: target.clone(),
                },
            ));
        }
    }
    let head_hash = session.head_hash()?.to_string();
    let mut lines = Lines::default();
    let mut rows = Rows::default();
    let meta = Meta {
        cli_version: codex_version,
        cwd: &session.header().cwd,
        id: &thread,
        session_id: &thread,
        timestamp: head_stamp,
    };
    lines.push(head_stamp, "session_meta", &meta)?;
    let marker = format!(
        "<TRANSLATED CONTEXT: THIS CODEX THREAD IS A FORK OF HOME SESSION {session_id} AT HEAD HASH {head_hash}, RENDERED FOR CODEX {codex_version}, NOT THAT SESSION>"
    );
    lines.message(head_stamp, "developer", &marker)?;
    let compaction = beside
        .path()
        .rev()
        .find(|entry| matches!(entry.body, EntryBody::Compaction { .. }));
    if let Some(compaction) = compaction {
        let reason = format!(
            "left off the context path by compaction {}",
            compaction.id()
        );
        beside.compacted_away(&reason, &mut rows);
    }
    let mut blocks = 0u64;
    for &n in beside.context_positions() {
        blocks += walk_entry(beside.entry(n), &mut lines, &mut rows)?;
        beside.after_entry(n, &mut lines, &mut rows)?;
    }
    beside.carried_prompt(session, &mut lines, &mut rows)?;
    let account = Account {
        session: session_id.to_owned(),
        head: head.id().to_owned(),
        head_hash,
        thread,
        codex_version: codex_version.to_owned(),
        rows,
    };
    let account_bytes = serde_json::to_vec_pretty(&account).map_err(|source| HomeError::Json {
        context: "the loss account could not be serialised",
        source,
    })?;
    let leaf = TranslationData {
        harness: HARNESS.to_owned(),
        codex_version: codex_version.to_owned(),
        thread: account.thread.clone(),
        head: account.head.clone(),
        head_hash: account.head_hash.clone(),
        rollout: relative,
        rollout_sha256: Hash::of(&lines.text).to_string(),
        account_sha256: Hash::of(&account_bytes).to_string(),
    };
    let entries = beside.context_positions().len() as u64;
    drop(beside);
    if let Some(dir) = rollout.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the rollout directory", dir, e))?;
    }
    write_new(&rollout, &lines.text)?;
    if let Err(e) = write_new(&account_path, &account_bytes) {
        remove_written(&[&rollout]);
        return Err(e);
    }
    if let Err(e) = append_leaf(session, &leaf) {
        remove_written(&[&rollout, &account_path]);
        return Err(e);
    }
    Ok(Translation {
        rollout,
        account: account_path,
        thread: account.thread,
        entries,
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
            for item in items_of(id, message, rows) {
                lines.item(stamp, &item)?;
            }
            Ok(match message.get("content") {
                Some(Value::Array(parts)) => parts.len() as u64,
                Some(_) => 1,
                None => 0,
            })
        }
        EntryBody::Compaction { summary, .. } => {
            let text = format!("<COMPACTION SUMMARY {id}>\n{summary}");
            marked(entry, ("compaction", "COMPACTION SUMMARY", "summary"), rows);
            lines.message(stamp, "developer", &text)?;
            Ok(0)
        }
        EntryBody::BranchSummary { summary, .. } => {
            let text = format!("<BRANCH SUMMARY {id}>\n{summary}");
            marked(entry, ("branch_summary", "BRANCH SUMMARY", "summary"), rows);
            lines.message(stamp, "developer", &text)?;
            Ok(0)
        }
        EntryBody::CustomMessage { content, .. } => {
            let mut text = format!("<CUSTOM MESSAGE {id}>\n");
            let mut walked = 1;
            let mut first = true;
            let mut add = |said: &str| {
                if !first {
                    text.push('\n');
                }
                text.push_str(said);
                first = false;
            };
            match content {
                Value::String(said) => add(said),
                Value::Array(parts) => {
                    walked = parts.len() as u64;
                    for (index, part) in parts.iter().enumerate() {
                        match (kind_of(part), part.get("text").and_then(Value::as_str)) {
                            ("text", Some(said)) => add(said),
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
            marked(entry, ("custom_message", "CUSTOM MESSAGE", "content"), rows);
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
fn marked(entry: &Entry, (before, marker, carried): (&str, &str, &str), rows: &mut Rows) {
    let mut how = vec![format!(
        "carried as developer text under the {marker} marker"
    )];
    how.extend(
        body_keys(&entry.body)
            .into_iter()
            .filter(|key| *key != carried)
            .map(|key| format!("{key} not carried")),
    );
    rows.changed(entry.id(), None, before, "marked text", &how.join("; "));
}

/// The keys a marked entry's body writes beside `type`, `id`, `parentId`
/// and `timestamp`, sorted and each once, as its line in the session file
/// holds them. Every field is named, so a field added to one of these
/// bodies must be placed here before this compiles.
fn body_keys(body: &EntryBody) -> BTreeSet<&str> {
    let (named, rest): (&[&str], _) = match body {
        EntryBody::Compaction {
            summary: _,
            first_kept_entry_id: _,
            tokens_before: _,
            rest,
        } => (&["summary", "firstKeptEntryId", "tokensBefore"], Some(rest)),
        EntryBody::BranchSummary {
            from_id: _,
            summary: _,
            rest,
        } => (&["fromId", "summary"], Some(rest)),
        EntryBody::CustomMessage {
            custom_type: _,
            content: _,
            rest,
        } => (&["customType", "content"], Some(rest)),
        _ => (&[], None),
    };
    let frame = ["type", "id", "parentId", "timestamp"];
    named
        .iter()
        .copied()
        .chain(
            rest.into_iter()
                .flat_map(|rest| rest.keys().map(String::as_str)),
        )
        .filter(|key| !frame.contains(key))
        .collect()
}
