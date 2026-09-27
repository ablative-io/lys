//! The handover (HOME-015, building HOME-001 R12): an outgoing session's
//! letter carried into a new successor home as inherited memory (ADR-058).
//!
//! The letter is one or more entry ids of the outgoing session: a run of
//! assistant message entries standing next to each other on its
//! root-to-head path, in path order. It is copied whole: each entry keeps
//! its id, its timestamp and its message, every thinking block and
//! signature byte for byte, and only its parent link is rewritten to chain
//! onto the successor. Nothing is composed, merged or re-ordered.
//!
//! The outgoing session is read without owning it: no lock is taken and
//! nothing is written, created or renamed under the outgoing home. Every
//! refusal is checked before any file or directory is created, and names
//! ids and paths only, never the letter's text, thinking or signature.
//!
//! The successor is a new home at a path that is absent or an empty
//! directory, holding one session with a fresh id, the outgoing header's
//! cwd and no parentSession. Its first entry is a `lys.inherited` entry
//! with no rule, which is what says the successor's first memory is
//! inherited; then the letter's entries; then a `session_info` named
//! `inherited from <outgoing session id>`. No field is added to Pi's
//! grammar and no block is written to either home.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::HomeError;
use crate::record::canon::{AUTHORED, Inherited};
use crate::record::entries::{CUSTOM_INHERITED, Entry, EntryBody, INHERITED_FROM};
use crate::record::index::read_head;
use crate::record::lantern::session_file;
use crate::record::reader::SessionReader;
use crate::record::{Home, fresh_id};

/// What one handover wrote: paths and ids, never transcript content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoverReport {
    /// The successor home.
    pub successor_home: PathBuf,
    /// The successor session's id.
    pub session: String,
    /// The successor session's file.
    pub file: PathBuf,
    /// Whether the successor's first entry is a `lys.inherited` entry, read
    /// back from the file.
    pub inherited: bool,
}

/// Hand the letter `letter` of session `from` in the home at `home` to a new
/// successor home at `successor`. Every check runs, in order, before any
/// file or directory is created: the successor path, the session, each
/// letter entry being an assistant message, the run standing next to each
/// other on the root-to-head path in the order given, no entry authored,
/// and at least one thinking block.
pub fn handover(
    home: &Path,
    from: &str,
    letter: &[String],
    successor: &Path,
) -> Result<HandoverReport, HomeError> {
    check_successor(successor)?;
    let outgoing = Home::read(home)?;
    let reader = SessionReader::open(session_file(&outgoing, from)?)?;
    let session = || reader.header().id.clone();
    let entries = reader.entries(letter)?;
    if let Some(entry) = entries.iter().find(|e| {
        message_of(e)
            .and_then(|m| m.get("role"))
            .and_then(Value::as_str)
            != Some("assistant")
    }) {
        return Err(HomeError::LetterNotAssistant {
            session: session(),
            id: entry.id().to_owned(),
        });
    }
    check_contiguous(&reader, &entries)?;
    if let Some(entry) = entries.iter().find(|e| field(e, "provider") == AUTHORED) {
        return Err(HomeError::LetterAuthored {
            session: session(),
            id: entry.id().to_owned(),
        });
    }
    let thinking = entries.iter().any(holds_thinking);
    let (true, Some(first), Some(last)) = (thinking, entries.first(), entries.last()) else {
        return Err(HomeError::LetterWithoutThinking {
            session: session(),
            ids: letter.to_vec(),
        });
    };
    let inherited = Inherited {
        authored: false,
        from_session: Some(session()),
        from_entries: letter.to_vec(),
        provider: field(first, "provider").to_owned(),
        api: field(first, "api").to_owned(),
        model: field(first, "model").to_owned(),
        curated_at: last.base.timestamp.clone(),
        curated_by: session(),
        rule: None,
    };
    write_successor(&reader, &inherited, entries, successor)
}

/// Refuse a successor path that exists and is not an empty directory.
fn check_successor(successor: &Path) -> Result<(), HomeError> {
    let refused = || HomeError::SuccessorNotEmpty {
        path: successor.to_path_buf(),
    };
    match std::fs::symlink_metadata(successor) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(HomeError::io("reading the successor path", successor, e)),
        Ok(meta) if !meta.is_dir() => Err(refused()),
        Ok(_) => {
            let mut listing = std::fs::read_dir(successor)
                .map_err(|e| HomeError::io("listing the successor path", successor, e))?;
            if listing.next().is_some() {
                Err(refused())
            } else {
                Ok(())
            }
        }
    }
}

/// Refuse a letter whose entries do not stand next to each other on the
/// root-to-head path in the order given. The path is the head's ancestry
/// in the reader's own index, walked in memory without reading an entry.
fn check_contiguous(reader: &SessionReader, entries: &[Entry]) -> Result<(), HomeError> {
    let index = reader.index();
    let on_path: HashSet<&str> = match read_head(reader.file(), index)? {
        Some(head) => index
            .ancestry(&head)?
            .into_iter()
            .map(|row| row.id.as_str())
            .collect(),
        None => HashSet::new(),
    };
    let refused = |entry: &Entry| HomeError::LetterNotContiguous {
        session: reader.header().id.clone(),
        id: entry.id().to_owned(),
    };
    if let Some(entry) = entries.iter().find(|e| !on_path.contains(e.id())) {
        return Err(refused(entry));
    }
    for pair in entries.windows(2) {
        if pair[1].parent_id() != Some(pair[0].id()) {
            return Err(refused(&pair[1]));
        }
    }
    Ok(())
}

/// Write the successor home: the rule-less `lys.inherited` entry, the
/// letter's entries with only their parent links rewritten, then the
/// `session_info` name, and read the first entry back.
fn write_successor(
    reader: &SessionReader,
    inherited: &Inherited,
    entries: Vec<Entry>,
    successor: &Path,
) -> Result<HandoverReport, HomeError> {
    let from = &reader.header().id;
    let data = serde_json::to_value(inherited).map_err(|source| HomeError::Json {
        context: "the inherited entry could not be serialised",
        source,
    })?;
    let home = Home::open(successor)?;
    let id = fresh_id();
    let mut session = home.create_session(&id, &reader.header().cwd, None)?;
    let mut prev = session.append(EntryBody::Custom {
        custom_type: CUSTOM_INHERITED.to_owned(),
        data: Some(data),
    })?;
    for mut entry in entries {
        entry.base.parent_id = Some(prev);
        session.append_entry(&entry)?;
        prev = entry.base.id;
    }
    session.append(EntryBody::SessionInfo {
        name: Some(format!("{INHERITED_FROM}{from}")),
        rest: Map::new(),
    })?;
    let file = session.file().to_path_buf();
    drop(session);
    let written = SessionReader::open(&file)?;
    let inherited = match written.ids().next() {
        Some(first) => written.entry(first)?.is_custom(CUSTOM_INHERITED),
        None => false,
    };
    Ok(HandoverReport {
        successor_home: successor.to_path_buf(),
        session: id,
        file,
        inherited,
    })
}

fn message_of(entry: &Entry) -> Option<&Value> {
    match &entry.body {
        EntryBody::Message { message } => Some(message),
        _ => None,
    }
}

/// A string field of the entry's message, or empty when it has none.
fn field<'a>(entry: &'a Entry, key: &str) -> &'a str {
    message_of(entry)
        .and_then(|m| m.get(key))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn holds_thinking(entry: &Entry) -> bool {
    message_of(entry)
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
        .is_some_and(|parts| {
            parts
                .iter()
                .any(|p| p.get("type").and_then(Value::as_str) == Some("thinking"))
        })
}
