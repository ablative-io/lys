//! What hangs beside the path (HOME-009 R5): labelled sidechains carried as
//! marked text, every other entry off the path listed lost, and a forked
//! child's carried message written as the thread's next user prompt.
//!
//! An `agent ` label off the path whose parent is on the context path names
//! a sidechain: the label's target and the message entries descending from
//! it. That sidechain is carried as one developer message, placed directly
//! after the items of the label's parent, holding the marker line and each
//! message's text, tool calls and tool results' text; the label is carried by
//! the marker line. Every other entry off the root-to-head path that
//! descends from it is a row in the account, whatever its type: under an
//! entry a compaction left off the context path, lost with that compaction's
//! reason; otherwise an off-path compaction, branch summary or custom
//! message is lost as not carried, any other entry that is not a message is
//! lost as no conversation, and a message outside a carried sidechain is an
//! unlabelled branch, listed and never carried.
//!
//! A child forked at a user message ends with the carried message's text,
//! read through the same `seed_of` the Claude Code render uses, as one user
//! message after the whole walked history. The translation opens the parent
//! session's file once more to list each part of the carried message that
//! is not text. That parent file is the one file read beyond the session's.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::error::HomeError;
use crate::harness::claude_code::seed::{seed_of, sessions_dir_of};
use crate::harness::codex::account::{Rows, part_hash};
use crate::harness::codex::parts::{
    EMPTY_THINKING, NO_ARGUMENTS, NO_RESULT_ID, NO_ROLE_ITEM, NO_TEXT, REDACTED, Thinking, call_of,
    kind_of, named, not_carried, role_of, thinking_of,
};
use crate::harness::codex::rollout::{Lines, NOT_CONVERSATION, entry_kind, text_only};
use crate::record::Session;
use crate::record::entries::{CUSTOM_FORKED_FROM, Entry, EntryBody};
use crate::record::fork::ForkedFrom;
use crate::record::lantern::data_of;
use crate::record::reader::SessionReader;

/// Why an off-path compaction, branch summary or custom message is lost.
pub const OFF_PATH: &str = "off the context path: not carried";
/// Why a message off the path outside a carried sidechain is lost.
pub const UNLABELLED: &str = "off the context path with no agent label: not carried";

/// Every entry of the session file, and which of them are on the paths.
pub(crate) struct Beside {
    entries: Vec<Entry>,
    children: HashMap<String, Vec<usize>>,
    path: HashSet<String>,
    context: HashSet<String>,
}

impl Beside {
    /// Read every entry of the session's own file, in file order.
    pub(crate) fn read(
        session: &Session,
        path: &[Entry],
        context: &[Entry],
    ) -> Result<Self, HomeError> {
        let reader = SessionReader::open(session.file())?;
        let mut entries = Vec::with_capacity(reader.len());
        for id in reader.ids() {
            entries.push(reader.entry(id)?);
        }
        let mut children: HashMap<String, Vec<usize>> = HashMap::new();
        for (n, entry) in entries.iter().enumerate() {
            if let Some(parent) = entry.parent_id() {
                children.entry(parent.to_owned()).or_default().push(n);
            }
        }
        Ok(Self {
            entries,
            children,
            path: path.iter().map(|e| e.id().to_owned()).collect(),
            context: context.iter().map(|e| e.id().to_owned()).collect(),
        })
    }

    /// The entries off the root-to-head path descending from `id`, in file
    /// order, never passing through an entry of the path.
    fn off_path_under(&self, id: &str) -> Vec<usize> {
        let mut found = BTreeSet::new();
        let mut stack = vec![id.to_owned()];
        while let Some(at) = stack.pop() {
            for &n in self.children.get(&at).into_iter().flatten() {
                let child = &self.entries[n];
                if !self.path.contains(child.id()) && found.insert(n) {
                    stack.push(child.id().to_owned());
                }
            }
        }
        found.into_iter().collect()
    }

    /// Rows for everything off the path under an entry a compaction left.
    pub(crate) fn left_behind(&self, id: &str, reason: &str, rows: &mut Rows) {
        for n in self.off_path_under(id) {
            let entry = &self.entries[n];
            rows.lost(entry.id(), None, entry_kind(entry), reason);
        }
    }

    /// After a context-path entry's items: its carried sidechains' items and
    /// a row for every entry off the path beneath it.
    pub(crate) fn after_entry(
        &self,
        anchor: &Entry,
        lines: &mut Lines,
        rows: &mut Rows,
    ) -> Result<(), HomeError> {
        let under = self.off_path_under(anchor.id());
        let mut carried: HashSet<usize> = HashSet::new();
        for &n in &under {
            if carried.contains(&n) {
                continue;
            }
            let entry = &self.entries[n];
            if let Some((agent, target)) = self.agent_label(entry, anchor) {
                rows.changed(
                    entry.id(),
                    None,
                    "label",
                    "marker line",
                    "agent id carried in the marker line",
                );
                let messages = self.sidechain(target);
                let mut text = format!(
                    "<SIDECHAIN {} AGENT {agent} UNDER ENTRY {}>",
                    self.entries[target].id(),
                    anchor.id()
                );
                for &m in &messages {
                    marked_message(&self.entries[m], anchor.id(), &mut text, rows);
                }
                carried.extend(messages);
                lines.message(&entry.base.timestamp, "developer", &text)?;
                continue;
            }
            match &entry.body {
                EntryBody::Compaction { .. }
                | EntryBody::BranchSummary { .. }
                | EntryBody::CustomMessage { .. } => {
                    rows.lost(entry.id(), None, entry_kind(entry), OFF_PATH);
                }
                EntryBody::Message { .. } => {
                    rows.lost(entry.id(), None, "unlabelled branch", UNLABELLED);
                }
                _ => rows.lost(entry.id(), None, entry_kind(entry), NOT_CONVERSATION),
            }
        }
        Ok(())
    }

    /// An `agent ` label hanging from the anchor on the context path whose
    /// target is off the path: the agent id and the target's position.
    fn agent_label<'a>(&self, entry: &'a Entry, anchor: &Entry) -> Option<(&'a str, usize)> {
        let EntryBody::Label {
            target_id,
            label: Some(label),
        } = &entry.body
        else {
            return None;
        };
        let agent = label.strip_prefix("agent ")?;
        if entry.parent_id() != Some(anchor.id())
            || !self.context.contains(anchor.id())
            || self.path.contains(target_id)
        {
            return None;
        }
        let target = self.entries.iter().position(|e| e.id() == target_id)?;
        Some((agent, target))
    }

    /// The sidechain a label names: its target and the message entries
    /// descending from it, through any entry between them, in file order.
    fn sidechain(&self, target: usize) -> Vec<usize> {
        let mut all = self.off_path_under(self.entries[target].id());
        all.push(target);
        all.sort_unstable();
        all.into_iter()
            .filter(|&n| matches!(self.entries[n].body, EntryBody::Message { .. }))
            .collect()
    }
}

/// One sidechain message's lines, appended to the marked text, and its rows.
fn marked_message(entry: &Entry, parent: &str, text: &mut String, rows: &mut Rows) {
    let EntryBody::Message { message } = &entry.body else {
        return;
    };
    let id = entry.id();
    let role = match role_of(message) {
        Ok(role) => role,
        Err(kind) => {
            rows.lost(id, Some(part_hash(message)), &kind, NO_ROLE_ITEM);
            return;
        }
    };
    let call_id = if role == "toolResult" {
        let Some(call_id) = named(message, "toolCallId") else {
            rows.lost(id, Some(part_hash(message)), "toolResult", NO_RESULT_ID);
            return;
        };
        Some(call_id)
    } else {
        None
    };
    let how = format!("carried as marked text under entry {parent}");
    rows.changed(id, None, "sidechain", "marked text", &how);
    let mut said = vec![format!("[{role} {id}]")];
    if let Some(call_id) = call_id {
        said.push(format!("[tool result {call_id}]"));
    }
    match message.get("content") {
        Some(Value::String(content)) => said.push(content.clone()),
        Some(Value::Array(parts)) => {
            for (index, part) in parts.iter().enumerate() {
                said.extend(part_line(id, role, index, part, rows));
            }
        }
        _ => {}
    }
    for line in said {
        text.push('\n');
        text.push_str(&line);
    }
}

/// One part of a sidechain message as a line of marked text, or its row.
fn part_line(id: &str, role: &str, index: usize, part: &Value, rows: &mut Rows) -> Option<String> {
    let hash = || Some(part_hash(part));
    match (kind_of(part), role) {
        ("text", _) => {
            let text = part.get("text").and_then(Value::as_str).map(str::to_owned);
            if text.is_none() {
                rows.lost(id, hash(), "text", NO_TEXT);
            }
            text
        }
        ("thinking", "assistant") => match thinking_of(part) {
            Thinking::Readable(text) => Some(text.to_owned()),
            Thinking::Redacted => {
                rows.lost(id, hash(), "thinking", REDACTED);
                None
            }
            Thinking::Empty => {
                rows.lost(id, hash(), "thinking", EMPTY_THINKING);
                None
            }
        },
        ("toolCall", "assistant") => {
            let call = call_of(part).and_then(|(call, name)| {
                part.get("arguments")
                    .map(|arguments| format!("[tool call {call} {name}] {arguments}"))
                    .ok_or(NO_ARGUMENTS)
            });
            match call {
                Ok(line) => Some(line),
                Err(reason) => {
                    rows.lost(id, hash(), "toolCall", reason);
                    None
                }
            }
        }
        (kind, _) => {
            rows.lost(id, hash(), kind, &text_only(kind, index));
            None
        }
    }
}

/// A forked child's carried message as the thread's last user prompt, with
/// its changed row and a lost row for each part of it that is not text.
pub(crate) fn carried_prompt(
    session: &Session,
    context: &[Entry],
    lines: &mut Lines,
    rows: &mut Rows,
) -> Result<(), HomeError> {
    let Some(seed) = seed_of(session, context)? else {
        return Ok(());
    };
    let Some(forked) = context
        .iter()
        .rev()
        .find(|entry| entry.is_custom(CUSTOM_FORKED_FROM))
    else {
        return Ok(());
    };
    let data: ForkedFrom = data_of(&session.header().id, forked, CUSTOM_FORKED_FROM)?;
    let Some(carried) = data.carried else {
        return Ok(());
    };
    lines.message(&forked.base.timestamp, "user", &seed.text)?;
    let sessions = sessions_dir_of(session.file())?;
    let parent = SessionReader::open(sessions.join(format!("{}.jsonl", data.parent_session)))?;
    let entry = parent.entry(&carried)?;
    let EntryBody::Message { message } = &entry.body else {
        return Ok(());
    };
    let mut how = vec![
        "text parts joined by newlines into one input_text part, read from the parent session's file"
            .to_owned(),
    ];
    how.extend(not_carried(message, &["role", "content"]));
    rows.changed(&carried, None, "point", "first prompt", &how.join("; "));
    if let Some(Value::Array(parts)) = message.get("content") {
        for (index, part) in parts.iter().enumerate() {
            let kind = kind_of(part);
            if kind != "text" {
                let reason = format!(
                    "{kind} part {index} not carried: a carried prompt holds text parts only"
                );
                rows.lost(&carried, Some(part_hash(part)), kind, &reason);
            }
        }
    }
    Ok(())
}
