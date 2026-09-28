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
//! message after the whole walked history. `seed_of` keeps only the text
//! parts, so the parent session's file, the one file read beyond the
//! session's, is opened once more through its reader to list each part of
//! the carried message that is not text.

use std::borrow::Cow;
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
use crate::record::entries::{CUSTOM_FORKED_FROM, Entry, EntryBody};
use crate::record::fork::ForkedFrom;
use crate::record::lantern::data_of;
use crate::record::reader::SessionReader;
use crate::record::{Session, safe_component};

/// Why an off-path compaction, branch summary or custom message is lost.
pub const OFF_PATH: &str = "off the context path: not carried";
/// Why a message off the path outside a carried sidechain is lost.
pub const UNLABELLED: &str = "off the context path with no agent label: not carried";

/// Every entry of the session file, read once in file order, with the
/// root-to-head path and the context path as positions among them.
pub(crate) struct Beside {
    entries: Vec<Entry>,
    by_id: HashMap<String, usize>,
    children: Vec<Vec<usize>>,
    path: Vec<usize>,
    context: Vec<usize>,
    on_path: Vec<bool>,
    on_context: Vec<bool>,
}

impl Beside {
    /// Read every entry of the session's own file through its index, and
    /// derive the path from the head and the context path from the path.
    pub(crate) fn read(session: &Session) -> Result<Self, HomeError> {
        let entries = session.entries()?;
        let by_id: HashMap<String, usize> = entries
            .iter()
            .enumerate()
            .map(|(n, entry)| (entry.id().to_owned(), n))
            .collect();
        let mut children = vec![Vec::new(); entries.len()];
        for (n, entry) in entries.iter().enumerate() {
            if let Some(&parent) = entry.parent_id().and_then(|id| by_id.get(id)) {
                children[parent].push(n);
            }
        }
        let path = match session.head()? {
            Some(head) => ancestry(&entries, &by_id, head)?,
            None => Vec::new(),
        };
        let on: Vec<&Entry> = path.iter().map(|&n| &entries[n]).collect();
        let context = match Session::context_positions(&on) {
            Some(positions) => positions.into_iter().map(|at| path[at]).collect(),
            None => path.clone(),
        };
        let mut on_path = vec![false; entries.len()];
        for &n in &path {
            on_path[n] = true;
        }
        let mut on_context = vec![false; entries.len()];
        for &n in &context {
            on_context[n] = true;
        }
        Ok(Self {
            entries,
            by_id,
            children,
            path,
            context,
            on_path,
            on_context,
        })
    }

    /// The entry at a position.
    pub(crate) fn entry(&self, n: usize) -> &Entry {
        &self.entries[n]
    }

    /// The root-to-head path, root first.
    pub(crate) fn path(&self) -> impl DoubleEndedIterator<Item = &Entry> {
        self.path.iter().map(|&n| &self.entries[n])
    }

    /// The context path's positions, in context order.
    pub(crate) fn context_positions(&self) -> &[usize] {
        &self.context
    }

    /// The context path, in context order.
    pub(crate) fn context(&self) -> impl DoubleEndedIterator<Item = &Entry> {
        self.context.iter().map(|&n| &self.entries[n])
    }

    /// Rows for each entry of the path a compaction left off the context
    /// path, and for everything off the path under it, with `reason`.
    pub(crate) fn compacted_away(&self, reason: &str, rows: &mut Rows) {
        for &n in self.path.iter().filter(|&&n| !self.on_context[n]) {
            let entry = &self.entries[n];
            rows.lost(entry.id(), None, entry_kind(entry), reason);
            for m in self.off_path_under(n) {
                let under = &self.entries[m];
                rows.lost(under.id(), None, entry_kind(under), reason);
            }
        }
    }

    /// The entries off the root-to-head path descending from the entry at
    /// `n`, in file order, never passing through an entry of the path.
    fn off_path_under(&self, n: usize) -> Vec<usize> {
        let mut found = BTreeSet::new();
        let mut stack = vec![n];
        while let Some(at) = stack.pop() {
            for &child in &self.children[at] {
                if !self.on_path[child] && found.insert(child) {
                    stack.push(child);
                }
            }
        }
        found.into_iter().collect()
    }

    /// After the items of the context-path entry at `anchor`: its carried
    /// sidechains' items and a row for every entry off the path beneath it.
    pub(crate) fn after_entry(
        &self,
        anchor: usize,
        lines: &mut Lines,
        rows: &mut Rows,
    ) -> Result<(), HomeError> {
        let anchor_id = self.entries[anchor].id();
        let under = self.off_path_under(anchor);
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
                    "<SIDECHAIN {} AGENT {agent} UNDER ENTRY {anchor_id}>",
                    self.entries[target].id(),
                );
                for &m in &messages {
                    marked_message(&self.entries[m], anchor_id, &mut text, rows);
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
    fn agent_label<'a>(&self, entry: &'a Entry, anchor: usize) -> Option<(&'a str, usize)> {
        let EntryBody::Label {
            target_id,
            label: Some(label),
        } = &entry.body
        else {
            return None;
        };
        let agent = label.strip_prefix("agent ")?;
        if entry.parent_id() != Some(self.entries[anchor].id()) || !self.on_context[anchor] {
            return None;
        }
        let &target = self.by_id.get(target_id)?;
        (!self.on_path[target]).then_some((agent, target))
    }

    /// The sidechain a label names: its target and the message entries
    /// descending from it, through any entry between them, in file order.
    fn sidechain(&self, target: usize) -> Vec<usize> {
        let mut all = self.off_path_under(target);
        all.push(target);
        all.sort_unstable();
        all.into_iter()
            .filter(|&n| matches!(self.entries[n].body, EntryBody::Message { .. }))
            .collect()
    }

    /// A forked child's carried message as the thread's last user prompt,
    /// with its changed row and a lost row for each part of it that is not
    /// text. The text is read through [`seed_of`]; the parent session's file
    /// is then opened once more for the carried entry's other parts.
    pub(crate) fn carried_prompt(
        &self,
        session: &Session,
        lines: &mut Lines,
        rows: &mut Rows,
    ) -> Result<(), HomeError> {
        let Some(forked) = self
            .context()
            .rev()
            .find(|entry| entry.is_custom(CUSTOM_FORKED_FROM))
        else {
            return Ok(());
        };
        let Some(seed) = seed_of(session, std::slice::from_ref(forked))? else {
            return Ok(());
        };
        let carried = carried_entry(session, forked)?;
        lines.message(&forked.base.timestamp, "user", &seed.text)?;
        let EntryBody::Message { message } = &carried.body else {
            return Ok(());
        };
        let mut how = vec![
            "text parts joined by newlines into one input_text part, read from the parent session's file"
                .to_owned(),
        ];
        how.extend(not_carried(message, &["role", "content"]));
        rows.changed(carried.id(), None, "point", "first prompt", &how.join("; "));
        if let Some(Value::Array(parts)) = message.get("content") {
            for (index, part) in parts.iter().enumerate() {
                let kind = kind_of(part);
                if kind != "text" {
                    let reason = format!(
                        "{kind} part {index} not carried: a carried prompt holds text parts only"
                    );
                    rows.lost(carried.id(), Some(part_hash(part)), kind, &reason);
                }
            }
        }
        Ok(())
    }
}

/// The carried entry a `lys.forked_from` entry names, as the parent session's
/// file holds it, read once more through the parent's reader after
/// [`seed_of`] has read its text; `seed_of` has already refused a record
/// whose `coordinate_carried` and `carried` disagree, so a missing `carried`
/// here is refused by the same name.
fn carried_entry(session: &Session, forked: &Entry) -> Result<Entry, HomeError> {
    let data: ForkedFrom = data_of(&session.header().id, forked, CUSTOM_FORKED_FROM)?;
    let Some(carried) = data.carried else {
        return Err(HomeError::EntryShape {
            session: session.header().id.clone(),
            id: forked.id().to_owned(),
            custom_type: CUSTOM_FORKED_FROM.to_owned(),
            source: None,
        });
    };
    safe_component("session id", &data.parent_session)?;
    let sessions = sessions_dir_of(session.file())?;
    SessionReader::open(sessions.join(format!("{}.jsonl", data.parent_session)))?
        .entry(&carried)
}

/// The positions from the head back to the root, root first, as the index's
/// ancestry gives them; a parent not on record is refused by name.
fn ancestry(
    entries: &[Entry],
    by_id: &HashMap<String, usize>,
    head: &str,
) -> Result<Vec<usize>, HomeError> {
    let mut path: Vec<usize> = Vec::new();
    let mut at = head;
    loop {
        let &n = by_id.get(at).ok_or_else(|| HomeError::UnknownParent {
            id: path
                .last()
                .map_or_else(|| at.to_owned(), |&last| entries[last].id().to_owned()),
            parent: at.to_owned(),
        })?;
        path.push(n);
        match entries[n].parent_id() {
            Some(parent) => at = parent,
            None => break,
        }
    }
    path.reverse();
    Ok(path)
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
    text.extend(["\n[", role, " ", id, "]"]);
    if let Some(call_id) = call_id {
        text.extend(["\n[tool result ", call_id, "]"]);
    }
    match message.get("content") {
        Some(Value::String(content)) => {
            text.push('\n');
            text.push_str(content);
        }
        Some(Value::Array(parts)) => {
            for (index, part) in parts.iter().enumerate() {
                if let Some(line) = part_line(id, role, index, part, rows) {
                    text.push('\n');
                    text.push_str(&line);
                }
            }
        }
        _ => {}
    }
}

/// One part of a sidechain message as a line of marked text, or its row.
fn part_line<'a>(
    id: &str,
    role: &str,
    index: usize,
    part: &'a Value,
    rows: &mut Rows,
) -> Option<Cow<'a, str>> {
    let hash = || Some(part_hash(part));
    match (kind_of(part), role) {
        ("text", _) => {
            let text = part.get("text").and_then(Value::as_str);
            if text.is_none() {
                rows.lost(id, hash(), "text", NO_TEXT);
            }
            text.map(Cow::Borrowed)
        }
        ("thinking", "assistant") => match thinking_of(part) {
            Thinking::Readable(text) => Some(Cow::Borrowed(text)),
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
                Ok(line) => Some(Cow::Owned(line)),
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
