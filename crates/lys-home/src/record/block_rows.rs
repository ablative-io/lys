//! The block rows beside a session, `<id>.blocks.jsonl` (HOME-030 R2).
//!
//! One JSON row per content part the importer stored through
//! [`BlockStore::put`](crate::record::blocks::BlockStore::put):
//! `{entry, part, hash}`, where `entry` is the id of the entry the part went
//! into, `part` is the part's 0-based index in its source record's content
//! (a string content is part 0) and `hash` is the SHA-256 hex `put`
//! returned, never one recomputed from the Pi-shaped part.
//!
//! Invariants: a row names an entry id, an index and a hash, never content,
//! text or a length of text; the file is appended only, and is never part of
//! Pi's grammar, so no field is added to the header or to any entry; it can
//! be rebuilt from the original file by importing its parts through the
//! store again. A session imported before the file existed has none, and a
//! reader reports that session's blocks as unverified rather than guess them
//! ([`read_block_rows`] answers `None`).

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::blocks::{Hash, sync_dir};
use crate::record::index::Index;

/// One row: the entry a part went into, the part's index in its source
/// record, and the block's hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockRow {
    /// The id of the entry the part went into.
    pub entry: String,
    /// The part's 0-based index in its source record's content.
    pub part: u64,
    /// The SHA-256 hex the block store returned for the part.
    pub hash: String,
}

/// A row as the importer holds it while it writes: the part's index, the
/// block's hash, and the block's length in bytes as it was stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldRow {
    /// The part's 0-based index in its source record's content.
    pub part: u64,
    /// The block's hash.
    pub hash: Hash,
    /// The block's length in bytes.
    pub len: u64,
}

/// The rows file of one session, open for appending, with the rows this
/// writer appended held by entry id.
#[derive(Debug)]
pub struct BlockRowWriter {
    path: PathBuf,
    file: fs::File,
    by_entry: HashMap<String, Vec<HeldRow>>,
}

impl BlockRowWriter {
    /// Open the rows file beside a session file for appending, creating it
    /// when it is absent.
    pub fn open(session_file: &Path) -> Result<Self, HomeError> {
        let path = Index::blocks_path(session_file);
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| HomeError::io("opening the block rows", &path, e))?;
        Ok(Self {
            path,
            file,
            by_entry: HashMap::new(),
        })
    }

    /// The rows file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one row: the part `part` of the entry `entry`, stored under
    /// `hash` as `len` bytes. Only the entry, the index and the hash are
    /// written; the length is held for the loss entry and never written.
    pub fn append(
        &mut self,
        entry: &str,
        part: u64,
        hash: &Hash,
        len: u64,
    ) -> Result<(), HomeError> {
        let row = BlockRow {
            entry: entry.to_owned(),
            part,
            hash: hash.as_str().to_owned(),
        };
        let mut line = serde_json::to_string(&row).map_err(|source| HomeError::Json {
            context: "a block row could not be serialised",
            source,
        })?;
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .map_err(|e| HomeError::io("appending a block row", &self.path, e))?;
        self.by_entry.entry(row.entry).or_default().push(HeldRow {
            part,
            hash: hash.clone(),
            len,
        });
        Ok(())
    }

    /// The rows this writer appended for one entry, in the order appended.
    #[must_use]
    pub fn rows_of(&self, entry: &str) -> &[HeldRow] {
        self.by_entry
            .get(entry)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Make every row appended so far durable: the file, then its directory.
    pub fn sync(&self) -> Result<(), HomeError> {
        self.file
            .sync_all()
            .map_err(|e| HomeError::io("syncing the block rows", &self.path, e))?;
        sync_dir(self.path.parent().unwrap_or_else(|| Path::new(".")))
    }
}

/// The rows beside a session file, in file order; `None` when the rows
/// file is absent, as it is for a session imported before it existed. A row
/// that does not parse is refused by line.
pub fn read_block_rows(session_file: &Path) -> Result<Option<Vec<BlockRow>>, HomeError> {
    let path = Index::blocks_path(session_file);
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(HomeError::io("opening the block rows", &path, e)),
    };
    let mut rows = Vec::new();
    for (n, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| HomeError::io("reading the block rows", &path, e))?;
        if line.trim().is_empty() {
            continue;
        }
        let row: BlockRow = serde_json::from_str(&line).map_err(|e| HomeError::Malformed {
            path: path.clone(),
            line: n + 1,
            what: "block row",
            reason: e.to_string(),
        })?;
        rows.push(row);
    }
    Ok(Some(rows))
}
