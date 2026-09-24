//! The offset index and the persisted head that sit beside a session file.
//!
//! Pi reads a whole session file to build its context. A home file may be
//! large, so beside it `<stem>.index.jsonl` records, per entry, its id, its
//! parent and where its line begins and how long it is; reading the path from
//! the head to the root then seeks to those lines only. `<stem>.head` holds
//! the head entry's id, written whole and renamed into place, so reopening
//! restores where the session stood and never assumes the last line appended.
//!
//! An index that disagrees with its file (a shorter file, a row past the end)
//! is refused by name and rebuilt from the file, which is the one full read.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::blocks::sync_dir;
use crate::record::entries::{Entry, SessionHeader};

/// One row of the index: where an entry's line lies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexRow {
    /// The entry id.
    pub id: String,
    /// Its parent's id.
    pub parent: Option<String>,
    /// The byte offset of the line's first byte.
    pub offset: u64,
    /// The line's length in bytes, newline included.
    pub len: u64,
    /// The custom type, for a custom entry; lets a lookup by custom type seek
    /// to those entries only, on every branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<String>,
}

/// The index of one session file, held in memory and mirrored to disk.
#[derive(Clone, Debug)]
pub struct Index {
    file: PathBuf,
    rows: Vec<IndexRow>,
    by_id: HashMap<String, usize>,
    /// Where the file ends as far as the index knows.
    end: u64,
}

impl Index {
    /// The index file beside a session file.
    #[must_use]
    pub fn index_path(session_file: &Path) -> PathBuf {
        sibling(session_file, "index.jsonl")
    }

    /// The head file beside a session file.
    #[must_use]
    pub fn head_path(session_file: &Path) -> PathBuf {
        sibling(session_file, "head")
    }

    /// The lock file beside a session file: held open by the session's one owner.
    #[must_use]
    pub fn lock_path(session_file: &Path) -> PathBuf {
        sibling(session_file, ".lock")
    }

    /// An empty index for a session file that holds only its header line.
    pub(crate) fn new(session_file: &Path, header_len: u64) -> Self {
        Self {
            file: Self::index_path(session_file),
            rows: Vec::new(),
            by_id: HashMap::new(),
            end: header_len,
        }
    }

    /// Load the index beside a session file, verifying it against the file's
    /// length; rebuild it from the file when it is missing or stale. Returns
    /// the header, the index, and whether a rebuild happened.
    pub fn load(session_file: &Path) -> Result<(SessionHeader, Self, bool), HomeError> {
        let (header, header_len) = read_header(session_file)?;
        let file_len = fs::metadata(session_file)
            .map_err(|e| HomeError::io("measuring the session file", session_file, e))?
            .len();
        let index_file = Self::index_path(session_file);
        if index_file.is_file() {
            match Self::read_rows(&index_file) {
                Ok(rows) => {
                    let end = rows.last().map_or(header_len, |r| r.offset + r.len);
                    if end == file_len {
                        let mut index = Self {
                            file: index_file,
                            rows: Vec::new(),
                            by_id: HashMap::new(),
                            end: header_len,
                        };
                        for row in rows {
                            index.push_row(row);
                        }
                        return Ok((header, index, false));
                    }
                }
                Err(HomeError::Malformed { .. }) => {}
                Err(e) => return Err(e),
            }
        }
        let index = Self::rebuild(session_file, header_len)?;
        Ok((header, index, true))
    }

    fn read_rows(index_file: &Path) -> Result<Vec<IndexRow>, HomeError> {
        let reader = BufReader::new(
            fs::File::open(index_file)
                .map_err(|e| HomeError::io("opening the index", index_file, e))?,
        );
        let mut rows = Vec::new();
        for (n, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| HomeError::io("reading the index", index_file, e))?;
            if line.trim().is_empty() {
                continue;
            }
            let row: IndexRow = serde_json::from_str(&line).map_err(|e| HomeError::Malformed {
                path: index_file.to_path_buf(),
                line: n + 1,
                what: "index row",
                reason: e.to_string(),
            })?;
            rows.push(row);
        }
        Ok(rows)
    }

    /// Scan the session file once and write a fresh index beside it.
    fn rebuild(session_file: &Path, header_len: u64) -> Result<Self, HomeError> {
        let mut reader = BufReader::new(
            fs::File::open(session_file)
                .map_err(|e| HomeError::io("opening the session file", session_file, e))?,
        );
        let mut index = Self::new(session_file, header_len);
        let mut offset = 0u64;
        let mut buf = Vec::new();
        let mut n = 0usize;
        loop {
            buf.clear();
            let read = reader
                .read_until(b'\n', &mut buf)
                .map_err(|e| HomeError::io("reading the session file", session_file, e))?;
            if read == 0 {
                break;
            }
            n += 1;
            let len = read as u64;
            if n > 1 && !buf.iter().all(u8::is_ascii_whitespace) {
                let entry: Entry =
                    serde_json::from_slice(&buf).map_err(|e| HomeError::Malformed {
                        path: session_file.to_path_buf(),
                        line: n,
                        what: "session entry",
                        reason: e.to_string(),
                    })?;
                let custom = match &entry.body {
                    crate::record::entries::EntryBody::Custom { custom_type, .. } => {
                        Some(custom_type.clone())
                    }
                    _ => None,
                };
                if index.by_id.contains_key(&entry.base.id) {
                    return Err(HomeError::Malformed {
                        path: session_file.to_path_buf(),
                        line: n,
                        what: "session entry",
                        reason: "its id is already on record".to_owned(),
                    });
                }
                if let Some(parent) = &entry.base.parent_id
                    && !index.by_id.contains_key(parent)
                {
                    return Err(HomeError::Malformed {
                        path: session_file.to_path_buf(),
                        line: n,
                        what: "session entry",
                        reason: "its parent is not on record before it".to_owned(),
                    });
                }
                index.push_row(IndexRow {
                    id: entry.base.id,
                    parent: entry.base.parent_id,
                    offset,
                    len,
                    custom,
                });
            }
            offset += len;
        }
        index.end = offset;
        index.write_all()?;
        Ok(index)
    }

    fn push_row(&mut self, row: IndexRow) {
        self.end = row.offset + row.len;
        self.by_id.insert(row.id.clone(), self.rows.len());
        self.rows.push(row);
    }

    fn write_all(&self) -> Result<(), HomeError> {
        let dir = parent_dir(&self.file);
        let tmp = self.file.with_extension("jsonl.tmp");
        {
            let mut file =
                fs::File::create(&tmp).map_err(|e| HomeError::io("creating the index", &tmp, e))?;
            for row in &self.rows {
                let line = serde_json::to_string(row).map_err(|e| HomeError::Malformed {
                    path: tmp.clone(),
                    line: 0,
                    what: "index row",
                    reason: e.to_string(),
                })?;
                file.write_all(line.as_bytes())
                    .map_err(|e| HomeError::io("writing the index", &tmp, e))?;
                file.write_all(b"\n")
                    .map_err(|e| HomeError::io("writing the index", &tmp, e))?;
            }
            file.sync_all()
                .map_err(|e| HomeError::io("syncing the index", &tmp, e))?;
        }
        fs::rename(&tmp, &self.file)
            .map_err(|e| HomeError::io("placing the index", &self.file, e))?;
        sync_dir(dir)
    }

    /// Record one appended entry: append its row to the index file and sync.
    pub(crate) fn append(&mut self, row: IndexRow) -> Result<(), HomeError> {
        let line = serde_json::to_string(&row).map_err(|e| HomeError::Malformed {
            path: self.file.clone(),
            line: 0,
            what: "index row",
            reason: e.to_string(),
        })?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
            .map_err(|e| HomeError::io("opening the index", &self.file, e))?;
        file.write_all(line.as_bytes())
            .map_err(|e| HomeError::io("writing the index", &self.file, e))?;
        file.write_all(b"\n")
            .map_err(|e| HomeError::io("writing the index", &self.file, e))?;
        file.sync_all()
            .map_err(|e| HomeError::io("syncing the index", &self.file, e))?;
        self.push_row(row);
        Ok(())
    }

    /// Where the session file ends, as the index knows it.
    #[must_use]
    pub fn end(&self) -> u64 {
        self.end
    }

    /// How many entries are indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether no entries are indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row of an entry.
    #[must_use]
    pub fn row(&self, id: &str) -> Option<&IndexRow> {
        self.by_id.get(id).map(|&i| &self.rows[i])
    }

    /// The last row appended.
    #[must_use]
    pub fn last(&self) -> Option<&IndexRow> {
        self.rows.last()
    }

    /// The rows of custom entries of this custom type, in file order.
    #[must_use]
    pub fn rows_of_custom(&self, custom_type: &str) -> Vec<&IndexRow> {
        self.rows
            .iter()
            .filter(|r| r.custom.as_deref() == Some(custom_type))
            .collect()
    }

    /// The ids from `id` back to the root, root first.
    pub fn ancestry(&self, id: &str) -> Result<Vec<&IndexRow>, HomeError> {
        let mut path: Vec<&IndexRow> = Vec::new();
        let mut cur: &str = id;
        loop {
            let row = self.row(cur).ok_or_else(|| HomeError::UnknownParent {
                id: path.last().map_or_else(|| cur.to_owned(), |r| r.id.clone()),
                parent: cur.to_owned(),
            })?;
            path.push(row);
            match &row.parent {
                Some(parent) => cur = parent,
                None => break,
            }
        }
        path.reverse();
        Ok(path)
    }

    /// Read the entries at these rows by seeking to each, in the order given.
    /// Returns the entries and how many bytes were read from the file.
    pub fn read_rows_from(
        &self,
        session_file: &Path,
        rows: &[&IndexRow],
    ) -> Result<(Vec<Entry>, u64), HomeError> {
        let mut file = fs::File::open(session_file)
            .map_err(|e| HomeError::io("opening the session file", session_file, e))?;
        let mut out = Vec::with_capacity(rows.len());
        let mut read = 0u64;
        for row in rows {
            file.seek(SeekFrom::Start(row.offset))
                .map_err(|e| HomeError::io("seeking the session file", session_file, e))?;
            let mut buf = vec![
                0u8;
                usize::try_from(row.len).map_err(|source| HomeError::RowTooLong {
                    path: session_file.to_path_buf(),
                    len: row.len,
                    source,
                })?
            ];
            file.read_exact(&mut buf)
                .map_err(|e| HomeError::io("reading an entry", session_file, e))?;
            read += row.len;
            let entry: Entry = serde_json::from_slice(&buf).map_err(|e| HomeError::Malformed {
                path: session_file.to_path_buf(),
                line: 0,
                what: "session entry",
                reason: format!("at offset {}: {e}", row.offset),
            })?;
            if entry.base.id != row.id {
                return Err(HomeError::StaleIndex {
                    path: session_file.to_path_buf(),
                    reason: "an entry's id differs from its row",
                });
            }
            out.push(entry);
        }
        Ok((out, read))
    }
}

/// Read the persisted head: `Some(id)`, `None` when the head is the header
/// (no entries or moved before the first), or the last indexed entry when no
/// head file exists.
pub fn read_head(session_file: &Path, index: &Index) -> Result<Option<String>, HomeError> {
    let path = Index::head_path(session_file);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let id = text.trim();
            Ok(if id.is_empty() {
                None
            } else {
                Some(id.to_owned())
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(index.last().map(|r| r.id.clone()))
        }
        Err(e) => Err(HomeError::io("reading the head", &path, e)),
    }
}

/// Persist the head: written whole to a temporary file and renamed into place.
pub fn write_head(session_file: &Path, head: Option<&str>) -> Result<(), HomeError> {
    let path = Index::head_path(session_file);
    let tmp = path.with_extension("head.tmp");
    {
        let mut file =
            fs::File::create(&tmp).map_err(|e| HomeError::io("creating the head", &tmp, e))?;
        file.write_all(head.unwrap_or("").as_bytes())
            .map_err(|e| HomeError::io("writing the head", &tmp, e))?;
        file.write_all(b"\n")
            .map_err(|e| HomeError::io("writing the head", &tmp, e))?;
        file.sync_all()
            .map_err(|e| HomeError::io("syncing the head", &tmp, e))?;
    }
    fs::rename(&tmp, &path).map_err(|e| HomeError::io("placing the head", &path, e))?;
    sync_dir(parent_dir(&path))
}

/// The header line of a session file and its length in bytes.
pub fn read_header(session_file: &Path) -> Result<(SessionHeader, u64), HomeError> {
    let mut reader = BufReader::new(
        fs::File::open(session_file)
            .map_err(|e| HomeError::io("opening the session file", session_file, e))?,
    );
    let mut line = Vec::new();
    let read = reader
        .read_until(b'\n', &mut line)
        .map_err(|e| HomeError::io("reading the session file", session_file, e))?;
    if read == 0 {
        return Err(HomeError::NoHeader {
            path: session_file.to_path_buf(),
            reason: "the file is empty".to_owned(),
        });
    }
    let header: SessionHeader = serde_json::from_slice(&line).map_err(|e| HomeError::NoHeader {
        path: session_file.to_path_buf(),
        reason: e.to_string(),
    })?;
    Ok((header, read as u64))
}

fn sibling(session_file: &Path, suffix: &str) -> PathBuf {
    let stem = session_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    parent_dir(session_file).join(format!("{stem}.{suffix}"))
}

fn parent_dir(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}
