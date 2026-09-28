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
//! Opening from the cached index checks every row in memory and reads the
//! final byte of at most three rows (the first, the middle and the last), so
//! an open costs the same however long the file is; a row between them whose
//! boundaries sit inside the lines is refused when it is read
//! ([`Index::read_rows_from`]), never rebuilt there. A failure to open, seek
//! or read the session file while checking is that error, never a stale
//! cache.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::blocks::sync_dir;
use crate::record::entries::{Entry, SessionHeader};
use crate::record::io_counts::IoCounter;

mod head;

pub(crate) use head::{place_head, write_head_counted};
pub use head::{read_head, read_header, write_head};

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
    /// length; rebuild it from the file when it is missing or stale, and write
    /// the rebuilt index beside the file. Returns the header, the index, and
    /// whether a rebuild happened. This is the owner's load; a reader that
    /// must write nothing uses [`Index::read`].
    pub fn load(session_file: &Path) -> Result<(SessionHeader, Self, bool), HomeError> {
        Self::load_counted(session_file, &IoCounter::default())
    }

    /// [`Index::load`], counting the syncs a rewritten index makes on the
    /// owner's counter.
    pub(crate) fn load_counted(
        session_file: &Path,
        counts: &IoCounter,
    ) -> Result<(SessionHeader, Self, bool), HomeError> {
        let (header, index, rebuilt) = Self::read(session_file)?;
        if rebuilt {
            index.write_all(counts)?;
        }
        Ok((header, index, rebuilt))
    }

    /// The index of a session file without writing anything: the cached
    /// index beside the file when it is this file's, otherwise one built in
    /// memory by scanning the file, left unwritten. Returns the header, the
    /// index, and whether it was built by scanning. A failure to open, seek
    /// or read the session file while checking the cache is returned as
    /// that error, and nothing is scanned.
    pub fn read(session_file: &Path) -> Result<(SessionHeader, Self, bool), HomeError> {
        let (header, header_len) = read_header(session_file)?;
        let file_len = fs::metadata(session_file)
            .map_err(|e| HomeError::io("measuring the session file", session_file, e))?
            .len();
        let index_file = Self::index_path(session_file);
        if index_file.is_file() {
            match Self::read_rows(&index_file) {
                Ok(rows) => {
                    let mut reader = BufReader::new(open_session(session_file)?);
                    if let Some(index) = Self::from_cached(
                        session_file,
                        &mut reader,
                        index_file,
                        rows,
                        header_len,
                        file_len,
                    )? {
                        return Ok((header, index, false));
                    }
                }
                Err(HomeError::Malformed { .. }) => {}
                Err(e) => return Err(e),
            }
        }
        let index = Self::scan(session_file, header_len)?;
        Ok((header, index, true))
    }

    /// The cached index beside a session file when it is this file's, and
    /// `None` when it is not: a row that does not parse, or rows the cached
    /// read refuses (`from_cached`). Never scans the session file and
    /// writes nothing, so a stale index is reported as stale rather than
    /// rebuilt in memory (HOME-019 R2). An index file that cannot be opened,
    /// or a session file that cannot be opened, sought or read, is refused
    /// by path.
    pub(crate) fn cached_only(session_file: &Path) -> Result<Option<Self>, HomeError> {
        let (_, header_len) = read_header(session_file)?;
        let file_len = fs::metadata(session_file)
            .map_err(|e| HomeError::io("measuring the session file", session_file, e))?
            .len();
        let index_file = Self::index_path(session_file);
        match Self::read_rows(&index_file) {
            Ok(rows) => {
                let mut reader = BufReader::new(open_session(session_file)?);
                Self::from_cached(
                    session_file,
                    &mut reader,
                    index_file,
                    rows,
                    header_len,
                    file_len,
                )
            }
            Err(HomeError::Malformed { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The cached rows as an index, when they are one. Every row is checked
    /// in memory: it starts where the one before ended (the first where the
    /// header ends), has a length whose end does not overflow, an id not yet
    /// seen and a parent already indexed (never itself), and the last ends
    /// where the file does. Then the final byte of the rows at positions 0,
    /// n/2 and n-1 of the n rows is read, each distinct position once, in
    /// ascending order, with one seek each, and must be a newline (every
    /// record is one line). Anything else is a cache that is not this
    /// file's, whatever its final offset says: `Ok(None)`, and the caller
    /// rebuilds from the session file, which is the record. A self-parent
    /// row taken on trust would make `ancestry` walk forever.
    ///
    /// A read that ends early at the end of the file is a stale cache; any
    /// other failure to seek or read is refused as that error, naming the
    /// session file, and nothing is rebuilt.
    pub(crate) fn from_cached<R: Read + Seek>(
        session_file: &Path,
        reader: &mut R,
        file: PathBuf,
        rows: Vec<IndexRow>,
        header_len: u64,
        file_len: u64,
    ) -> Result<Option<Self>, HomeError> {
        let mut index = Self {
            file,
            rows: Vec::new(),
            by_id: HashMap::new(),
            end: header_len,
        };
        for row in rows {
            if row.offset != index.end
                || row.len == 0
                || row.offset.checked_add(row.len).is_none()
                || index.by_id.contains_key(&row.id)
            {
                return Ok(None);
            }
            if let Some(parent) = &row.parent
                && !index.by_id.contains_key(parent)
            {
                return Ok(None);
            }
            index.push_row(row);
        }
        if index.end != file_len {
            return Ok(None);
        }
        let n = index.rows.len();
        let mut positions: Vec<usize> = Vec::with_capacity(3);
        for p in [0, n / 2, n.saturating_sub(1)] {
            if p < n && positions.last() != Some(&p) {
                positions.push(p);
            }
        }
        for p in positions {
            let Some(row) = index.rows.get(p) else {
                return Ok(None);
            };
            reader
                .seek(SeekFrom::Start(row.offset + row.len - 1))
                .map_err(|e| HomeError::io("seeking the session file", session_file, e))?;
            let mut byte = [0u8; 1];
            match reader.read_exact(&mut byte) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(HomeError::io("reading the session file", session_file, e)),
            }
            if byte != *b"\n" {
                return Ok(None);
            }
        }
        Ok(Some(index))
    }

    /// The rows of an index file, in file order.
    pub(crate) fn read_rows(index_file: &Path) -> Result<Vec<IndexRow>, HomeError> {
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

    /// Scan the session file once into an index held in memory; nothing is
    /// written. [`Index::load`] writes the result beside the file.
    fn scan(session_file: &Path, header_len: u64) -> Result<Self, HomeError> {
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
        Ok(index)
    }

    /// Hold one row in memory, and nothing more: the index file is not
    /// touched (a staged import writes the rows whole when it publishes).
    pub(crate) fn push_row(&mut self, row: IndexRow) {
        self.end = row.offset + row.len;
        self.by_id.insert(row.id.clone(), self.rows.len());
        self.rows.push(row);
    }

    /// Write every row to the index file, through a temporary file synced
    /// and renamed, then sync the directory.
    fn write_all(&self, counts: &IoCounter) -> Result<(), HomeError> {
        self.place_all(counts)?;
        sync_dir(parent_dir(&self.file))?;
        counts.synced();
        Ok(())
    }

    /// Write every row to the index file through a temporary file, synced
    /// and renamed into place, leaving the directory unsynced.
    pub(crate) fn place_all(&self, counts: &IoCounter) -> Result<(), HomeError> {
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
            counts.synced();
        }
        fs::rename(&tmp, &self.file).map_err(|e| HomeError::io("placing the index", &self.file, e))
    }

    /// Record one appended entry: append its row to the index file and sync.
    pub(crate) fn append(&mut self, row: IndexRow, counts: &IoCounter) -> Result<(), HomeError> {
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
        counts.synced();
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

    /// Every row, in file order.
    #[must_use]
    pub fn rows(&self) -> &[IndexRow] {
        &self.rows
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

fn open_session(session_file: &Path) -> Result<fs::File, HomeError> {
    fs::File::open(session_file)
        .map_err(|e| HomeError::io("opening the session file", session_file, e))
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
