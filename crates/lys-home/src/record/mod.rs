//! The home and its sessions.
//!
//! A [`Home`] is a directory: `sessions/<id>.jsonl` files in Pi's grammar,
//! each with its index and head beside it, and `blocks/` for content by hash.
//! A [`Session`] is one open session file: append a child of the head, move
//! the head, read the path from the head to the root by seeking, and build the
//! context path the way Pi's `buildSessionContext` does.

pub mod blocks;
#[cfg(test)]
mod blocks_tests;
pub mod call;
#[cfg(test)]
mod call_tests;
pub mod entries;
pub mod index;
#[cfg(test)]
mod record_tests;

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::HomeError;
use crate::record::blocks::BlockStore;
use crate::record::entries::{Entry, EntryBase, EntryBody, SessionHeader};
use crate::record::index::{Index, IndexRow, read_head, write_head};

/// The version of Pi's file format this crate writes: the tree format.
pub const PI_FORMAT_VERSION: u32 = 2;

/// A home directory.
#[derive(Clone, Debug)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    /// Open (creating if needed) a home at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let root = root.into();
        for sub in ["sessions", "blocks"] {
            let dir = root.join(sub);
            std::fs::create_dir_all(&dir)
                .map_err(|e| HomeError::io("creating the home", &dir, e))?;
        }
        Ok(Self { root })
    }

    /// Where the home lives.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The block store of this home.
    pub fn blocks(&self) -> Result<BlockStore, HomeError> {
        BlockStore::open(self.root.join("blocks"))
    }

    /// The path of a session file by id.
    #[must_use]
    pub fn session_path(&self, id: &str) -> PathBuf {
        self.root.join("sessions").join(format!("{id}.jsonl"))
    }

    /// Create a session: writes the header line, an empty index and a head.
    /// Refuses an existing file by name.
    pub fn create_session(
        &self,
        id: &str,
        cwd: &str,
        parent_session: Option<&str>,
    ) -> Result<Session, HomeError> {
        let header = SessionHeader {
            version: Some(PI_FORMAT_VERSION),
            id: id.to_owned(),
            timestamp: now(),
            cwd: cwd.to_owned(),
            parent_session: parent_session.map(str::to_owned),
        };
        Session::create(self.session_path(id), header)
    }

    /// Open a session by id.
    pub fn open_session(&self, id: &str) -> Result<Session, HomeError> {
        Session::open(self.session_path(id))
    }
}

/// One open session file.
#[derive(Debug)]
pub struct Session {
    file: PathBuf,
    header: SessionHeader,
    index: Index,
    head: Option<String>,
    /// Whether opening had to rebuild the index from the file.
    rebuilt: bool,
}

impl Session {
    /// Create a session file at `file` with this header.
    pub fn create(file: impl Into<PathBuf>, header: SessionHeader) -> Result<Self, HomeError> {
        let file = file.into();
        if file.exists() {
            return Err(HomeError::Exists { path: file });
        }
        let line = to_line(&header, &file)?;
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&file)
            .map_err(|e| HomeError::io("creating the session file", &file, e))?;
        f.write_all(line.as_bytes())
            .map_err(|e| HomeError::io("writing the header", &file, e))?;
        f.sync_all()
            .map_err(|e| HomeError::io("syncing the session file", &file, e))?;
        blocks::sync_dir(file.parent().unwrap_or_else(|| Path::new(".")))?;
        let index = Index::new(&file, line.len() as u64);
        write_head(&file, None)?;
        Ok(Self {
            file,
            header,
            index,
            head: None,
            rebuilt: false,
        })
    }

    /// Open an existing session file, loading or rebuilding its index and
    /// reading its persisted head.
    pub fn open(file: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let file = file.into();
        let (header, index, rebuilt) = Index::load(&file)?;
        let head = read_head(&file, &index)?;
        if let Some(id) = &head {
            if index.row(id).is_none() {
                return Err(HomeError::UnknownEntry {
                    session: header.id,
                    id: id.clone(),
                });
            }
        }
        Ok(Self {
            file,
            header,
            index,
            head,
            rebuilt,
        })
    }

    /// The session file.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The header.
    #[must_use]
    pub fn header(&self) -> &SessionHeader {
        &self.header
    }

    /// The head entry's id; `None` when the head is the header.
    #[must_use]
    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    /// How many entries the file holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the file holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Whether opening rebuilt the index from the whole file.
    #[must_use]
    pub fn index_was_rebuilt(&self) -> bool {
        self.rebuilt
    }

    /// Whether an entry of this id is on record.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.index.row(id).is_some()
    }

    /// Append an entry as a child of the head, with a fresh id and the current
    /// time, and advance the head to it. The entry line is durable before the
    /// index row, which is durable before the head.
    pub fn append(&mut self, body: EntryBody) -> Result<String, HomeError> {
        let id = fresh_id();
        let entry = Entry {
            base: EntryBase {
                id: id.clone(),
                parent_id: self.head.clone(),
                timestamp: now(),
            },
            body,
        };
        self.append_entry(&entry)?;
        Ok(id)
    }

    /// Append an entry exactly as given (id, parent and timestamp included), as
    /// an importer does, and advance the head to it. The parent must be on
    /// record or `None`.
    pub fn append_entry(&mut self, entry: &Entry) -> Result<(), HomeError> {
        if let Some(parent) = entry.parent_id() {
            if self.index.row(parent).is_none() {
                return Err(HomeError::UnknownParent {
                    id: entry.id().to_owned(),
                    parent: parent.to_owned(),
                });
            }
        }
        let line = to_line(entry, &self.file)?;
        let offset = self.index.end();
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&self.file)
                .map_err(|e| HomeError::io("opening the session file", &self.file, e))?;
            f.write_all(line.as_bytes())
                .map_err(|e| HomeError::io("appending an entry", &self.file, e))?;
            f.sync_all()
                .map_err(|e| HomeError::io("syncing the session file", &self.file, e))?;
        }
        self.index.append(IndexRow {
            id: entry.id().to_owned(),
            parent: entry.parent_id().map(str::to_owned),
            offset,
            len: line.len() as u64,
            custom: custom_type_of(entry),
        })?;
        write_head(&self.file, Some(entry.id()))?;
        self.head = Some(entry.id().to_owned());
        Ok(())
    }

    /// Move the head to an entry on record (or to the header with `None`).
    /// Nothing in the file changes.
    pub fn move_head(&mut self, id: Option<&str>) -> Result<(), HomeError> {
        if let Some(id) = id {
            if self.index.row(id).is_none() {
                return Err(HomeError::UnknownEntry {
                    session: self.header.id.clone(),
                    id: id.to_owned(),
                });
            }
        }
        write_head(&self.file, id)?;
        self.head = id.map(str::to_owned);
        Ok(())
    }

    /// One entry by id, read by seeking to it.
    pub fn entry(&self, id: &str) -> Result<Entry, HomeError> {
        let row = self.index.row(id).ok_or_else(|| HomeError::UnknownEntry {
            session: self.header.id.clone(),
            id: id.to_owned(),
        })?;
        let (mut entries, _) = self.index.read_rows_from(&self.file, &[row])?;
        entries.pop().ok_or(HomeError::StaleIndex {
            path: self.file.clone(),
            reason: "a row read no entry",
        })
    }

    /// The entries from the root to the head, root first, read by seeking to
    /// each; the second value is how many bytes of the file were read.
    pub fn path(&self) -> Result<(Vec<Entry>, u64), HomeError> {
        let Some(head) = &self.head else {
            return Ok((Vec::new(), 0));
        };
        let rows = self.index.ancestry(head)?;
        self.index.read_rows_from(&self.file, &rows)
    }

    /// The context path: what Pi's `buildSessionContext` feeds the model. With
    /// a compaction on the path, the compaction entry first, then the kept
    /// entries from `first_kept_entry_id` up to the compaction, then everything
    /// after it; without one, the whole path.
    pub fn context_path(&self) -> Result<Vec<Entry>, HomeError> {
        let (path, _) = self.path()?;
        let compaction = path
            .iter()
            .rposition(|e| matches!(e.body, EntryBody::Compaction { .. }));
        let Some(ci) = compaction else {
            return Ok(path);
        };
        let EntryBody::Compaction {
            first_kept_entry_id,
            ..
        } = &path[ci].body
        else {
            return Ok(path);
        };
        let mut out = Vec::with_capacity(path.len());
        out.push(path[ci].clone());
        let mut keeping = false;
        for entry in &path[..ci] {
            if entry.id() == first_kept_entry_id {
                keeping = true;
            }
            if keeping {
                out.push(entry.clone());
            }
        }
        out.extend(path[ci + 1..].iter().cloned());
        Ok(out)
    }

    /// The custom entries of a given custom type on the path, root first.
    pub fn customs(&self, custom_type: &str) -> Result<Vec<Entry>, HomeError> {
        let (path, _) = self.path()?;
        Ok(path
            .into_iter()
            .filter(|e| e.is_custom(custom_type))
            .collect())
    }

    /// Every durable custom entry of a given custom type in the file, on any
    /// branch and whatever the head, in file order, read by seeking to each.
    pub fn customs_everywhere(&self, custom_type: &str) -> Result<Vec<Entry>, HomeError> {
        let rows = self.index.rows_of_custom(custom_type);
        let (entries, _) = self.index.read_rows_from(&self.file, &rows)?;
        Ok(entries)
    }
}

fn custom_type_of(entry: &Entry) -> Option<String> {
    match &entry.body {
        EntryBody::Custom { custom_type, .. } => Some(custom_type.clone()),
        _ => None,
    }
}

fn to_line<T: serde::Serialize>(value: &T, file: &Path) -> Result<String, HomeError> {
    let mut line = serde_json::to_string(value).map_err(|e| HomeError::Malformed {
        path: file.to_path_buf(),
        line: 0,
        what: "session line",
        reason: e.to_string(),
    })?;
    line.push('\n');
    Ok(line)
}

/// The current time, RFC 3339 with millisecond precision.
#[must_use]
pub fn now() -> String {
    let t = time::OffsetDateTime::now_utc();
    let t = t
        .replace_nanosecond(u32::from(t.millisecond()) * 1_000_000)
        .unwrap_or(t);
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// A fresh entry id: 16 random bytes as hex, which Pi accepts as an id.
#[must_use]
pub fn fresh_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    blocks::hex_of(&bytes)
}

/// A JSON value's serialised size in bytes.
#[must_use]
pub fn json_len(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(0, |v| v.len())
}
