//! Writing a fork: the child from the parent's own lines, with the ancestry
//! on both sides (HOME-006 R3).
//!
//! The parent is opened as its one owner first, so a session another owner
//! holds refuses by name before any file is created. The child is a new
//! session with a fresh id, the parent header's `cwd`, and `parentSession`
//! the parent file's path relative to the home root (`sessions/<id>.jsonl`),
//! the field Pi reads as the parent session's file path. Each entry of the
//! cut is appended to the child as the exact bytes of its line in the parent
//! file, read by its index row's offset and length and never re-serialised,
//! through the same durable order as any append: the line, then its index
//! row, then the head. Then one `lys.forked_from` custom entry is appended as
//! the child of the cut entry and becomes the child's head, and one
//! `lys.fork` custom entry naming the child is appended at the parent's head,
//! which advances to it. A carried user message is counted for its parts
//! that are not text and never copied. Nothing else of the parent enters the
//! child: no header field but `cwd`, no credential, no handle, no launch
//! setting; no block is written and no earlier byte of the parent changes.
//! A failure after the child was created removes the child's files by name
//! and returns the refusal that stopped the fork; a cleanup that fails too
//! is refused naming both, so a half-written child is never silent.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::entries::{CUSTOM_FORK, CUSTOM_FORKED_FROM, Entry, EntryBody};
use crate::record::fork_cut::{Cut, resolve_and_cut};
use crate::record::fork_report::{ForkReport, candidate_hashes, count_held};
use crate::record::index::{Index, IndexRow, write_head};
use crate::record::lantern::own;
use crate::record::{Home, Session, custom_type_of, fresh_id, write_durable};

/// What a `lys.forked_from` entry carries in `custom.data`: the ancestry
/// as the child records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForkedFrom {
    /// The parent's session id.
    pub parent_session: String,
    /// The lantern the fork was taken through.
    pub lantern: String,
    /// The lantern's point.
    pub point: String,
    /// The cut entry: the last assistant message at or before the point.
    pub cut_at: String,
    /// Whether the point is a user message carried as the child's first
    /// prompt rather than copied.
    pub coordinate_carried: bool,
    /// The carried entry's id, when one is carried.
    pub carried: Option<String>,
    /// The parts of the carried message that are not text, counted by type.
    pub seed_left_out: BTreeMap<String, u64>,
}

/// The parts of a carried message that are not text, counted by type; empty
/// when nothing is carried, when the content is a string, or when every
/// part is text. A part with no `type` string is refused by name, never
/// counted under a made-up kind.
fn seed_left_out(carried: Option<&Entry>) -> Result<BTreeMap<String, u64>, HomeError> {
    let mut out = BTreeMap::new();
    let Some(EntryBody::Message { message }) = carried.map(|entry| &entry.body) else {
        return Ok(out);
    };
    let Some(parts) = message.get("content").and_then(Value::as_array) else {
        return Ok(out);
    };
    for part in parts {
        let kind = part
            .get("type")
            .and_then(Value::as_str)
            .ok_or(HomeError::BodyShape {
                api: "fork",
                reason: "a part of the carried message has no type",
            })?;
        if kind != "text" {
            *out.entry(kind.to_owned()).or_insert(0) += 1;
        }
    }
    Ok(out)
}

/// Remove a child session's files by name after a fork failed part way:
/// the session file, its index, its head, its lock and their temporaries.
/// A file already gone is fine; any other failure is reported.
fn remove_child(home: &Home, child: &str) -> Result<(), HomeError> {
    let file = home.session_path(child)?;
    let mut paths = vec![
        file.clone(),
        Index::index_path(&file),
        Index::head_path(&file),
        Index::lock_path(&file),
    ];
    paths.push(Index::index_path(&file).with_extension("jsonl.tmp"));
    paths.push(Index::head_path(&file).with_extension("head.tmp"));
    for path in paths {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(HomeError::io("removing a half-written child", &path, e)),
        }
    }
    Ok(())
}

/// Copy the cut into the child, append `lys.forked_from` as its head, and
/// mark the parent; everything after the child was created.
fn write_fork(
    parent: &mut Session,
    child: &mut Session,
    child_id: &str,
    cut: &Cut,
    forked_from: &ForkedFrom,
) -> Result<(), HomeError> {
    for (row, entry) in cut.rows.iter().zip(&cut.entries) {
        let line = read_line(&cut.file, row)?;
        child.append_copied(entry, &line)?;
    }
    let data = serde_json::to_value(forked_from).map_err(|source| HomeError::Json {
        context: "the forked_from data could not be serialised",
        source,
    })?;
    child.append(EntryBody::Custom {
        custom_type: CUSTOM_FORKED_FROM.to_owned(),
        data: Some(data),
    })?;
    parent.append(EntryBody::Custom {
        custom_type: CUSTOM_FORK.to_owned(),
        data: Some(json!({"child": child_id})),
    })?;
    Ok(())
}

/// The exact bytes of one entry's line, read by its row's offset and length.
pub(crate) fn read_line(file: &Path, row: &IndexRow) -> Result<Vec<u8>, HomeError> {
    let mut session = std::fs::File::open(file)
        .map_err(|e| HomeError::io("opening the session file", file, e))?;
    session
        .seek(SeekFrom::Start(row.offset))
        .map_err(|e| HomeError::io("seeking the session file", file, e))?;
    let mut line = vec![
        0u8;
        usize::try_from(row.len).map_err(|source| HomeError::RowTooLong {
            path: file.to_path_buf(),
            len: row.len,
            source,
        })?
    ];
    session
        .read_exact(&mut line)
        .map_err(|e| HomeError::io("reading an entry's line", file, e))?;
    if line.last() != Some(&b'\n') {
        return Err(HomeError::StaleIndex {
            path: file.to_path_buf(),
            reason: "a row does not end on a newline",
        });
    }
    Ok(line)
}

impl Session {
    /// Append a line copied from another session file exactly as its bytes
    /// stand, and advance the head to it. `entry` is that line parsed, for
    /// its id, parent and custom type; the bytes are what is written. The
    /// line is durable before its index row, which is durable before the
    /// head, and a step that fails after the line reconciles from the file
    /// as any append does.
    pub(crate) fn append_copied(&mut self, entry: &Entry, line: &[u8]) -> Result<(), HomeError> {
        self.reconcile()?;
        if self.index.row(entry.id()).is_some() {
            return Err(HomeError::DuplicateEntry {
                session: self.header.id.clone(),
                id: entry.id().to_owned(),
            });
        }
        if let Some(parent) = entry.parent_id()
            && self.index.row(parent).is_none()
        {
            return Err(HomeError::UnknownParent {
                id: entry.id().to_owned(),
                parent: parent.to_owned(),
            });
        }
        let text = std::str::from_utf8(line).map_err(|e| HomeError::Malformed {
            path: self.file.clone(),
            line: 0,
            what: "copied session line",
            reason: e.to_string(),
        })?;
        let offset = self.index.end();
        if let Err(e) = write_durable(&self.file, text) {
            self.stale = true;
            self.reconcile()?;
            return Err(e);
        }
        if let Err(e) = self.index.append(IndexRow {
            id: entry.id().to_owned(),
            parent: entry.parent_id().map(str::to_owned),
            offset,
            len: line.len() as u64,
            custom: custom_type_of(entry),
        }) {
            self.stale = true;
            self.reconcile()?;
            drop(e);
        }
        if let Err(e) = write_head(&self.file, Some(entry.id())) {
            self.stale = true;
            self.reconcile()?;
            return Err(e);
        }
        self.head = Some(entry.id().to_owned());
        Ok(())
    }
}

/// Fork a child session from `lantern`'s point, cutting from `session` when
/// one is named, and return the report. The candidate hashes are taken
/// from the cut before anything is written, and counted against the store
/// after the child stands.
pub fn fork(home: &Home, lantern: &str, session: Option<&str>) -> Result<ForkReport, HomeError> {
    let cut = resolve_and_cut(home, lantern, session)?;
    let candidates = candidate_hashes(&cut.session, &cut.entries)?;
    let carried = cut.carried.as_ref().map(|entry| entry.id().to_owned());
    let forked_from = ForkedFrom {
        parent_session: cut.session.clone(),
        lantern: cut.lantern.clone(),
        point: cut.point.clone(),
        cut_at: cut.cut_at.clone(),
        coordinate_carried: cut.coordinate_carried(),
        carried: carried.clone(),
        seed_left_out: seed_left_out(cut.carried.as_ref())?,
    };
    let mut parent = own(home, &cut.session)?;
    let child_id = fresh_id();
    let parent_path = format!("sessions/{}.jsonl", cut.session);
    let mut child = home.create_session(&child_id, &parent.header().cwd, Some(&parent_path))?;
    if let Err(reason) = write_fork(&mut parent, &mut child, &child_id, &cut, &forked_from) {
        // The child is released before its files go, so its lock is not held
        // while they are removed; the parent stays owned until the end.
        drop(child);
        return Err(match remove_child(home, &child_id) {
            Ok(()) => reason,
            Err(cleanup) => HomeError::ForkHalfWritten {
                child: child_id,
                reason: reason.to_string(),
                cleanup: cleanup.to_string(),
            },
        });
    }
    let (blocks, unstored) = count_held(&home.blocks()?, &candidates);
    Ok(ForkReport {
        child: child_id,
        parent: cut.session,
        lantern: cut.lantern,
        point: cut.point,
        cut_at: cut.cut_at,
        entries: cut.entries.len() as u64,
        blocks,
        unstored,
        coordinate_carried: forked_from.coordinate_carried,
        carried,
    })
}
