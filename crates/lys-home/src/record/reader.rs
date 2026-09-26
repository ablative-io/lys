//! A session read without owning it, and the sessions a home holds
//! (HOME-004 R2).
//!
//! [`SessionReader`] opens a session file for reading only: it takes no lock,
//! so a session another owner holds is read as it stands, and it creates,
//! writes, renames and truncates nothing, not the index, the head, the lock
//! or a temporary file. The index beside the file is used when it is this
//! file's; otherwise one is built in memory by scanning the file and left
//! unwritten ([`Index::read`]). Entries are read by seeking to their rows,
//! and custom entries of a type by seeking to the rows the index's custom
//! column names, on every branch (CN7). The owner's lock is untouched, so an
//! owner opening afterwards finds the session exactly as it left it.
//!
//! A reader sees the file as it was when it opened: an entry appended by the
//! owner afterwards is not on its index. The file is append-only, so every
//! row it holds stays true.

use std::path::{Path, PathBuf};

use crate::error::HomeError;
use crate::record::entries::{Entry, SessionHeader};
use crate::record::index::{Index, IndexRow, read_header};

/// The suffix an index file carries beside its session file.
const INDEX_SUFFIX: &str = ".index.jsonl";
/// The suffix every session file carries.
const SESSION_SUFFIX: &str = ".jsonl";

/// One session file, open for reading only.
#[derive(Clone, Debug)]
pub struct SessionReader {
    file: PathBuf,
    header: SessionHeader,
    index: Index,
    scanned: bool,
}

impl SessionReader {
    /// Open a session file for reading. Nothing is locked or written.
    pub fn open(file: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let file = file.into();
        let (header, index, scanned) = Index::read(&file)?;
        Ok(Self {
            file,
            header,
            index,
            scanned,
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

    /// Whether the index was built by scanning the file, because none beside
    /// it was this file's.
    #[must_use]
    pub fn index_was_scanned(&self) -> bool {
        self.scanned
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

    /// Whether an entry of this id is on record.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.index.row(id).is_some()
    }

    /// Every entry id, in file order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.index.rows().iter().map(|row| row.id.as_str())
    }

    /// One entry by id, read by seeking to it.
    pub fn entry(&self, id: &str) -> Result<Entry, HomeError> {
        let row = self.index.row(id).ok_or_else(|| HomeError::UnknownEntry {
            session: self.header.id.clone(),
            id: id.to_owned(),
        })?;
        let (mut entries, _) = self.read_rows(&[row])?;
        entries.pop().ok_or(HomeError::StaleIndex {
            path: self.file.clone(),
            reason: "a row read no entry",
        })
    }

    /// Every custom entry of a given custom type in the file, on any branch,
    /// in file order, read by seeking to the rows the index names.
    pub fn customs_everywhere(&self, custom_type: &str) -> Result<Vec<Entry>, HomeError> {
        let rows = self.index.rows_of_custom(custom_type);
        let (entries, _) = self.read_rows(&rows)?;
        Ok(entries)
    }

    fn read_rows(&self, rows: &[&IndexRow]) -> Result<(Vec<Entry>, u64), HomeError> {
        self.index.read_rows_from(&self.file, rows)
    }
}

/// The session ids under a `sessions/` directory, in ascending byte order.
/// A regular file named `<id>.jsonl` is a session, except one named
/// `<stem>.index.jsonl` whose first line is not a session header, which is
/// an index. A file that cannot be read while listing refuses by path; a
/// missing directory lists nothing.
pub fn session_ids(sessions: &Path) -> Result<Vec<String>, HomeError> {
    let entries = match std::fs::read_dir(sessions) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(HomeError::io("listing the sessions", sessions, e)),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| HomeError::io("listing the sessions", sessions, e))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| HomeError::io("reading a session file's type", &path, e))?;
        if !kind.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(id) = name.strip_suffix(SESSION_SUFFIX) else {
            continue;
        };
        if name.ends_with(INDEX_SUFFIX) {
            match read_header(&path) {
                Ok(_) => {}
                Err(HomeError::NoHeader { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        ids.push(id.to_owned());
    }
    ids.sort_unstable();
    Ok(ids)
}
