//! The loss account's terms for `every_row_names_its_kinds_and_reason`,
//! counted from the fixture's Claude Code records and from the home's
//! entries by walking parent ids, never from the account.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;

use serde_json::Value;

use lys_home::{Entry, EntryBody, Home, SessionReader};

/// The home's entries by id, file order, and each entry's children.
pub(crate) struct Walk {
    entries: HashMap<String, Entry>,
    order: Vec<String>,
    path: Vec<String>,
    pub(crate) context: HashSet<String>,
}

impl Walk {
    pub(crate) fn read(home: &Path) -> Result<Self, Box<dyn Error>> {
        let owned = Home::read(home)?;
        let reader: SessionReader = owned.read_session("s1")?;
        let order: Vec<String> = reader.ids().map(str::to_owned).collect();
        let mut entries = HashMap::new();
        for id in &order {
            entries.insert(id.clone(), reader.entry(id)?);
        }
        let head = owned
            .open_session("s1")?
            .head()?
            .map(str::to_owned)
            .ok_or("no head")?;
        let mut path = vec![head];
        while let Some(parent) = path
            .last()
            .and_then(|id| entries.get(id))
            .and_then(Entry::parent_id)
        {
            path.push(parent.to_owned());
        }
        path.reverse();
        let last = path
            .iter()
            .rposition(|id| matches!(entries[id].body, EntryBody::Compaction { .. }));
        let context = match last {
            None => path.iter().cloned().collect(),
            Some(at) => {
                let EntryBody::Compaction {
                    first_kept_entry_id,
                    ..
                } = &entries[&path[at]].body
                else {
                    return Err("not a compaction".into());
                };
                let kept = path[..at]
                    .iter()
                    .position(|id| id == first_kept_entry_id)
                    .unwrap_or(at);
                path[kept..].iter().cloned().collect()
            }
        };
        Ok(Self {
            entries,
            order,
            path,
            context,
        })
    }

    /// The first ancestor on the root-to-head path of an entry off it.
    pub(crate) fn anchor(&self, id: &str) -> Option<String> {
        let mut at = self.entries.get(id)?.parent_id()?.to_owned();
        loop {
            if self.path.contains(&at) {
                return Some(at);
            }
            let parent = self.entries.get(&at)?.parent_id()?.to_owned();
            at = parent;
        }
    }

    pub(crate) fn off_path(&self) -> impl Iterator<Item = &Entry> {
        self.order
            .iter()
            .filter(|id| !self.path.contains(id))
            .map(|id| &self.entries[id])
    }
}

/// Terms (1) to (3): what the context path itself loses.
pub(crate) fn on_path_losses(walk: &Walk) -> usize {
    let mut lost = 0;
    for id in walk.path.iter().filter(|id| walk.context.contains(*id)) {
        match &walk.entries[id].body {
            EntryBody::Message { message } => {
                let role = message["role"].as_str().unwrap_or("none");
                if !["user", "assistant", "toolResult"].contains(&role) {
                    lost += 1;
                    continue;
                }
                if role == "toolResult" && message["toolCallId"].as_str().is_none_or(str::is_empty)
                {
                    lost += 1;
                    continue;
                }
                for part in message["content"].as_array().into_iter().flatten() {
                    let kind = part["type"].as_str().unwrap_or("none");
                    let thinking = part["thinking"].as_str().unwrap_or("");
                    lost += usize::from(match (role, kind) {
                        ("assistant", "thinking") => thinking.trim().is_empty(),
                        ("assistant", "toolCall") => {
                            part["id"].as_str().is_none_or(str::is_empty)
                                || part["name"].as_str().is_none_or(str::is_empty)
                        }
                        (_, "image") => part["source"]["type"] != "base64",
                        ("assistant" | "user" | "toolResult", "text") => false,
                        _ => true,
                    });
                }
            }
            EntryBody::Compaction { .. } | EntryBody::BranchSummary { .. } => {}
            EntryBody::CustomMessage { content, .. } => {
                lost += content
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|p| p["type"] != "text")
                    .count();
            }
            _ => lost += 1,
        }
    }
    lost
}

/// Terms (4) and (5): entries left by the compaction with everything under
/// them, and entries off the path under the context path that are not
/// messages, less each carried sidechain's agent label.
pub(crate) fn off_path_losses(walk: &Walk) -> (usize, usize) {
    let left = walk
        .path
        .iter()
        .filter(|id| !walk.context.contains(*id))
        .count();
    let (mut under_left, mut beside) = (0, 0);
    for entry in walk.off_path() {
        let Some(anchor) = walk.anchor(entry.id()) else {
            continue;
        };
        if !walk.context.contains(&anchor) {
            under_left += 1;
            continue;
        }
        let agent = matches!(&entry.body, EntryBody::Label { label: Some(l), target_id }
            if l.starts_with("agent ") && entry.parent_id() == Some(anchor.as_str()) && !walk.path.contains(target_id));
        if !matches!(entry.body, EntryBody::Message { .. }) && !agent {
            beside += 1;
        }
    }
    (left + under_left, beside)
}

/// Terms (6) and (7), and the harness events under the carried sidechain of
/// agent `carried_agent`, from the records.
pub(crate) fn sidechain_losses(records: &[Value], carried_agent: &str) -> (usize, usize, usize) {
    let (mut carried, mut unlabelled, mut events) = (0, 0, 0);
    let mut agent: Option<String> = None;
    for record in records.iter().filter(|r| r["isSidechain"] == true) {
        if record["parentUuid"].is_null() && record["message"].is_object() {
            agent = record["agentId"].as_str().map(str::to_owned);
        }
        let parts: Vec<&Value> = record["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        match (&agent, record["type"].as_str()) {
            (Some(name), _) if name != carried_agent => {}
            (Some(_), Some("attachment" | "system")) => events += 1,
            (Some(_), _) => {
                events += parts.iter().filter(|p| p["type"] == "tool_result").count();
                carried += parts
                    .iter()
                    .filter(|p| match p["type"].as_str() {
                        Some("thinking") => {
                            p["thinking"].as_str().is_none_or(|t| t.trim().is_empty())
                        }
                        Some("text" | "tool_use" | "tool_result") => false,
                        _ => true,
                    })
                    .count();
            }
            (None, Some("assistant")) => unlabelled += 1,
            (None, _) => {
                let results = parts.iter().filter(|p| p["type"] == "tool_result").count();
                unlabelled += results
                    + usize::from(
                        parts.len() > results || record["message"]["content"].is_string(),
                    );
            }
        }
    }
    (carried, unlabelled, events)
}
