//! What hangs beside the context path, and a forked child's carried prompt
//! (HOME-009 R5).
//!
//! Invariants:
//!
//! - Every entry off the root-to-head path that descends from an entry of
//!   it is a row in the account, whatever its type. An entry descending from
//!   one a compaction left off the context path is lost naming that
//!   compaction. Of the rest, a labelled sidechain is carried and every
//!   other entry is lost with its kind and a true reason.
//! - A sidechain is carried when a label off the path whose text begins
//!   `agent ` has its parent on the context path and names an entry off the
//!   path: the target and the message entries descending from it, in file
//!   order, become one developer message placed after the items of the
//!   label's parent, opening with a marker line naming the target, the agent
//!   and the parent. The label is counted changed; each carried message is
//!   counted changed; a part marked text cannot hold is lost with its reason.
//! - A sidechain with no agent label cannot be told from any other branch
//!   off the path, so its message entries are listed as an unlabelled branch
//!   and never carried.
//! - A child forked at a user message carries that message's text, read
//!   through [`seed_of`] from the parent session's file, as the thread's next
//!   user prompt after the whole walked history; each part of it that is not
//!   text is lost by hash, read once more from that same file. No other file
//!   is read, and the seed's text enters no row.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::seed::{seed_of, sessions_dir_of};
use crate::harness::codex::account::{Rows, part_hash};
use crate::harness::codex::parts::{
    EMPTY_THINKING, NO_RESULT_ID, NO_ROLE, OTHER_ROLE, REDACTED, Thinking, arguments_json,
    check_call, named, not_carried, thinking_of,
};
use crate::record::Session;
use crate::record::entries::{CUSTOM_FORKED_FROM, Entry, EntryBody};
use crate::record::fork::ForkedFrom;
use crate::record::lantern::data_of;
use crate::record::reader::SessionReader;

/// Why an entry that is not conversation is not carried.
pub const NOT_CONVERSATION: &str = "Codex has no item for it and it is not conversation";
/// Why an off-path compaction, branch summary or custom message is not
/// carried.
pub const OFF_PATH: &str = "off the context path: not carried";
/// The kind an off-path message outside any carried sidechain is listed
/// under.
pub const UNLABELLED: &str = "unlabelled branch";
/// Why such a message is not carried.
pub const UNLABELLED_REASON: &str = "off the context path with no agent label: not carried";

/// One rollout item before its ordinal: the stamp its entry records, and
/// the payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The stamp, as the entry records it.
    pub stamp: String,
    /// The response item.
    pub payload: Value,
}

/// A developer message holding one text.
#[must_use]
pub fn developer(text: &str) -> Value {
    json!({"type": "message", "role": "developer",
        "content": [{"type": "input_text", "text": text}]})
}

/// An entry's kind as the account names it: its custom type for a custom
/// entry, its type otherwise.
#[must_use]
pub fn kind_of(entry: &Entry) -> &str {
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

/// A part marked text cannot hold, lost with its kind and index.
fn marked_loss(
    entry: &str,
    part: &Value,
    index: usize,
    holds: &str,
    rows: &mut Rows,
) -> Result<(), HomeError> {
    let kind = part
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("part with no type");
    rows.lose(
        entry,
        Some(part_hash(part)?),
        kind,
        &format!("{kind} part {index} not carried: {holds}"),
    );
    Ok(())
}

/// What marked text holds, for the reason of a part it cannot.
pub const MARKED_HOLDS: &str = "marked text holds text only";
/// What a carried prompt holds, for the reason of a part it cannot.
const PROMPT_HOLDS: &str = "a carried prompt holds text parts only";
/// How a carried prompt is reshaped.
const PROMPT_HOW: &str =
    "text parts joined by newlines into one input_text part, read from the parent session's file";

/// A carried sidechain: its label, target, agent and messages.
#[derive(Clone, Debug)]
struct Sidechain {
    agent: String,
    target: String,
    parent: String,
    members: Vec<usize>,
}

/// The whole file, read once, with each off-path entry's nearest ancestor
/// on the root-to-head path and the entries hanging under each such
/// ancestor, in file order.
#[derive(Debug)]
pub struct Beside {
    entries: Vec<Entry>,
    context: BTreeSet<String>,
    anchor: BTreeMap<String, String>,
    hanging: BTreeMap<String, Vec<usize>>,
    sidechains: BTreeMap<String, Sidechain>,
    carried: BTreeSet<String>,
    compaction: Option<String>,
}

impl Beside {
    /// Read every entry of the session's own file and place each one off
    /// the path.
    pub fn read(session: &Session, path: &[Entry], context: &[Entry]) -> Result<Self, HomeError> {
        let reader = SessionReader::open(session.file())?;
        let mut entries = Vec::with_capacity(reader.len());
        for id in reader.ids() {
            entries.push(reader.entry(id)?);
        }
        let on_path: BTreeSet<String> = path.iter().map(|e| e.id().to_owned()).collect();
        let context: BTreeSet<String> = context.iter().map(|e| e.id().to_owned()).collect();
        let parents: BTreeMap<&str, Option<&str>> =
            entries.iter().map(|e| (e.id(), e.parent_id())).collect();
        let mut anchor = BTreeMap::new();
        let mut hanging: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            if on_path.contains(entry.id()) {
                continue;
            }
            let mut current = entry.parent_id();
            for _ in 0..=entries.len() {
                let Some(id) = current else { break };
                if on_path.contains(id) {
                    anchor.insert(entry.id().to_owned(), id.to_owned());
                    hanging.entry(id.to_owned()).or_default().push(i);
                    break;
                }
                current = parents.get(id).copied().flatten();
            }
        }
        let compaction = path
            .iter()
            .rev()
            .find(|e| matches!(e.body, EntryBody::Compaction { .. }))
            .filter(|_| path.iter().any(|e| !context.contains(e.id())))
            .map(|e| e.id().to_owned());
        let mut beside = Self {
            entries,
            context,
            anchor,
            hanging,
            sidechains: BTreeMap::new(),
            carried: BTreeSet::new(),
            compaction,
        };
        beside.find_sidechains();
        Ok(beside)
    }

    /// The entries off the path whose nearest ancestor on it is `anchor`,
    /// in file order.
    fn hanging_under(&self, anchor: &str) -> impl Iterator<Item = &Entry> {
        let found = self.hanging.get(anchor).into_iter().flatten();
        found.map(|&i| &self.entries[i])
    }

    /// Whether an off-path entry hangs under an entry of the context path.
    fn under_context(&self, id: &str) -> bool {
        let anchor = self.anchor.get(id);
        anchor.is_some_and(|a| self.context.contains(a))
    }

    /// Mark each labelled sidechain that is carried, and its messages.
    fn find_sidechains(&mut self) {
        let mut position: BTreeMap<&str, usize> = BTreeMap::new();
        let mut children: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, entry) in self.entries.iter().enumerate() {
            position.insert(entry.id(), i);
            if let Some(parent) = entry.parent_id() {
                children.entry(parent).or_default().push(i);
            }
        }
        let mut found = Vec::new();
        let mut claimed = BTreeSet::new();
        for entry in &self.entries {
            let EntryBody::Label {
                target_id,
                label: Some(text),
            } = &entry.body
            else {
                continue;
            };
            let Some(agent) = text.strip_prefix("agent ") else {
                continue;
            };
            let Some(parent) = entry.parent_id().filter(|p| self.context.contains(*p)) else {
                continue;
            };
            let Some(&start) = position.get(target_id.as_str()) else {
                continue;
            };
            if !self.under_context(entry.id())
                || !self.under_context(target_id)
                || !claimed.insert(target_id.clone())
            {
                continue;
            }
            let mut members = Vec::new();
            let mut queue = vec![start];
            while let Some(at) = queue.pop() {
                if matches!(self.entries[at].body, EntryBody::Message { .. }) {
                    members.push(at);
                }
                if let Some(below) = children.get(self.entries[at].id()) {
                    queue.extend(below);
                }
            }
            members.sort_unstable();
            found.push((
                entry.id().to_owned(),
                Sidechain {
                    agent: agent.to_owned(),
                    target: target_id.clone(),
                    parent: parent.to_owned(),
                    members,
                },
            ));
        }
        for (label, side) in found {
            for &i in &side.members {
                self.carried.insert(self.entries[i].id().to_owned());
            }
            self.sidechains.insert(label, side);
        }
    }

    /// The rows of the root-to-head entries a compaction left off the
    /// context path, each followed by the entries descending from it.
    pub fn left_off(&self, path: &[Entry], rows: &mut Rows) {
        let Some(compaction) = &self.compaction else {
            return;
        };
        let reason = format!("left off the context path by compaction {compaction}");
        for entry in path.iter().filter(|e| !self.context.contains(e.id())) {
            rows.lose(entry.id(), None, kind_of(entry), &reason);
            for off in self.hanging_under(entry.id()) {
                rows.lose(off.id(), None, kind_of(off), &reason);
            }
        }
    }

    /// The items and rows of what hangs under one context-path entry, in
    /// file order: each carried sidechain as marked text, every other entry
    /// as a lost row.
    pub fn under(
        &self,
        anchor: &str,
        items: &mut Vec<Item>,
        rows: &mut Rows,
    ) -> Result<(), HomeError> {
        for entry in self.hanging_under(anchor) {
            if let Some(side) = self.sidechains.get(entry.id()) {
                items.push(self.carry(entry, side, rows)?);
                continue;
            }
            if self.carried.contains(entry.id()) {
                continue;
            }
            match &entry.body {
                EntryBody::Message { .. } => {
                    rows.lose(entry.id(), None, UNLABELLED, UNLABELLED_REASON);
                }
                EntryBody::Compaction { .. }
                | EntryBody::BranchSummary { .. }
                | EntryBody::CustomMessage { .. } => {
                    rows.lose(entry.id(), None, kind_of(entry), OFF_PATH);
                }
                _ => rows.lose(entry.id(), None, kind_of(entry), NOT_CONVERSATION),
            }
        }
        Ok(())
    }

    /// One carried sidechain as a developer message under its label's
    /// parent, stamped with the label's stamp.
    fn carry(&self, label: &Entry, side: &Sidechain, rows: &mut Rows) -> Result<Item, HomeError> {
        let (target, agent, parent) = (&side.target, &side.agent, &side.parent);
        let heading = format!("<SIDECHAIN {target} AGENT {agent} UNDER ENTRY {parent}>");
        let mut lines = vec![heading];
        rows.change(
            label.id(),
            None,
            "label",
            "marker line",
            "agent id carried in the marker line",
        );
        let how = format!("carried as marked text under entry {parent}");
        for &i in &side.members {
            let entry = &self.entries[i];
            if let EntryBody::Message { message } = &entry.body {
                member_lines(entry.id(), message, &how, &mut lines, rows)?;
            }
        }
        Ok(Item {
            stamp: label.base.timestamp.clone(),
            payload: developer(&lines.join("\n")),
        })
    }
}

/// One sidechain message as marked lines, with its rows.
fn member_lines(
    id: &str,
    message: &Value,
    how: &str,
    lines: &mut Vec<String>,
    rows: &mut Rows,
) -> Result<(), HomeError> {
    let role = match message.get("role").and_then(Value::as_str) {
        Some(role @ ("user" | "assistant")) => role,
        Some("toolResult") => {
            let Some(call) = named(message, "toolCallId") else {
                rows.lose(id, Some(part_hash(message)?), "toolResult", NO_RESULT_ID);
                return Ok(());
            };
            rows.change(id, None, "sidechain", "marked text", how);
            lines.push(format!("[toolResult {id}]"));
            lines.push(format!("[tool result {call}]"));
            let parts = message.get("content").and_then(Value::as_array);
            for (index, part) in parts.into_iter().flatten().enumerate() {
                let text = match part.get("type").and_then(Value::as_str) {
                    Some("text") => part.get("text").and_then(Value::as_str),
                    _ => None,
                };
                match text {
                    Some(text) => lines.push(text.to_owned()),
                    None => marked_loss(id, part, index, MARKED_HOLDS, rows)?,
                }
            }
            if let Some(Value::String(text)) = message.get("content") {
                lines.push(text.clone());
            }
            return Ok(());
        }
        Some(role) => {
            let kind = format!("{role} message");
            rows.lose(id, Some(part_hash(message)?), &kind, OTHER_ROLE);
            return Ok(());
        }
        None => {
            rows.lose(id, Some(part_hash(message)?), NO_ROLE, OTHER_ROLE);
            return Ok(());
        }
    };
    rows.change(id, None, "sidechain", "marked text", how);
    lines.push(format!("[{role} {id}]"));
    let parts = match message.get("content") {
        Some(Value::String(text)) => {
            lines.push(text.clone());
            return Ok(());
        }
        Some(Value::Array(parts)) => parts,
        _ => return Ok(()),
    };
    for (index, part) in parts.iter().enumerate() {
        match part.get("type").and_then(Value::as_str) {
            Some("text") => match part.get("text").and_then(Value::as_str) {
                Some(text) => lines.push(text.to_owned()),
                None => marked_loss(id, part, index, MARKED_HOLDS, rows)?,
            },
            Some("thinking") => match thinking_of(part) {
                Thinking::Readable(text) => lines.push(text.to_owned()),
                Thinking::Redacted => rows.lose(id, Some(part_hash(part)?), "thinking", REDACTED),
                Thinking::Empty => {
                    let hash = part_hash(part)?;
                    rows.lose(id, Some(hash), "thinking", EMPTY_THINKING);
                }
            },
            Some("toolCall") => match check_call(part) {
                Ok((call, name, arguments)) => {
                    let arguments = arguments_json(arguments)?;
                    lines.push(format!("[tool call {call} {name}] {arguments}"));
                }
                Err(reason) => rows.lose(id, Some(part_hash(part)?), "toolCall", reason),
            },
            _ => marked_loss(id, part, index, MARKED_HOLDS, rows)?,
        }
    }
    Ok(())
}

/// A forked child's carried prompt: the item after the walked history, and
/// its rows.
#[derive(Clone, Debug)]
pub struct Prompt {
    /// The line, stamped with the child's `lys.forked_from` entry's stamp.
    pub item: Item,
    /// The changed row and one lost row per part that is not text.
    pub rows: Rows,
}

/// The prompt of a child forked at a user message, or `None` for any other
/// session.
pub fn forked_prompt(session: &Session, context: &[Entry]) -> Result<Option<Prompt>, HomeError> {
    let Some(seed) = seed_of(session, context)? else {
        return Ok(None);
    };
    let mut forked_entry = None;
    for entry in context {
        if entry.is_custom(CUSTOM_FORKED_FROM) {
            forked_entry = Some(entry);
        }
    }
    let Some(forked_entry) = forked_entry else {
        return Ok(None);
    };
    let forked: ForkedFrom = data_of(&session.header().id, forked_entry, CUSTOM_FORKED_FROM)?;
    let Some(carried) = forked.carried else {
        return Ok(None);
    };
    let file = format!("{}.jsonl", forked.parent_session);
    let parent = SessionReader::open(sessions_dir_of(session.file())?.join(file))?;
    let entry = parent.entry(&carried)?;
    let mut rows = Rows::default();
    let EntryBody::Message { message } = &entry.body else {
        return Err(HomeError::BodyShape {
            api: "translate-codex",
            reason: "the carried entry is not a message",
        });
    };
    let mut how = vec![PROMPT_HOW.to_owned()];
    how.extend(not_carried(message, &["role", "content"]));
    rows.change(&carried, None, "point", "first prompt", &how.join("; "));
    let parts = message.get("content").and_then(Value::as_array);
    for (index, part) in parts.into_iter().flatten().enumerate() {
        if part.get("type").and_then(Value::as_str) != Some("text") {
            marked_loss(&carried, part, index, PROMPT_HOLDS, &mut rows)?;
        }
    }
    let payload = json!({"type": "message", "role": "user",
        "content": [{"type": "input_text", "text": seed.text}]});
    let stamp = forked_entry.base.timestamp.clone();
    Ok(Some(Prompt {
        item: Item { stamp, payload },
        rows,
    }))
}
