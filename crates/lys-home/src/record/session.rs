//! One open session file: its lock, its index and head, and the durable
//! append. The reads of the path from the head to the root are in `reads`,
//! the call-id map in `call_map` and the staged import in `staged`.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::HomeError;
use crate::record::blocks;
use crate::record::entries::{Entry, EntryBase, EntryBody, SessionHeader};
use crate::record::helpers::{custom_type_of, fresh_id, now};
use crate::record::index::{Index, IndexRow, read_head, write_head_counted};
use crate::record::io_counts::IoCounter;
use crate::record::lock::SessionLock;
use crate::record::staged::{Staged, clear_remainder};

/// One open session file.
#[derive(Debug)]
pub struct Session {
    pub(super) file: PathBuf,
    pub(super) header: SessionHeader,
    pub(super) index: Index,
    pub(super) head: Option<String>,
    /// Whether opening had to rebuild the index from the file.
    pub(super) rebuilt: bool,
    /// The exclusive lock on `<id>.lock`, held for the life of this owner.
    pub(super) lock: SessionLock,
    /// Set when an append left the index or head behind the file; cleared by
    /// [`Session::reconcile`], which runs before anything else is admitted.
    pub(super) stale: bool,
    /// How many times this owner reconciled from the file.
    pub(super) reconciliations: u64,
    /// The entries this owner read and the syncs its writes made.
    pub(super) io: IoCounter,
    /// Call id to the first `lys.call` entry holding it; `None` until the
    /// first lookup and after a reconcile.
    pub(super) calls: Mutex<Option<HashMap<String, String>>>,
    /// Set while a new session is built under its staging name.
    pub(super) staged: Option<Staged>,
}

impl Session {
    /// Create a session file at `file` with this header.
    pub fn create(file: impl Into<PathBuf>, header: SessionHeader) -> Result<Self, HomeError> {
        let file = file.into();
        let lock = SessionLock::take(&file)?;
        if file.exists() {
            return Err(HomeError::Exists { path: file });
        }
        let io = IoCounter::default();
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
        io.synced();
        blocks::sync_dir(file.parent().unwrap_or_else(|| Path::new(".")))?;
        io.synced();
        let index = Index::new(&file, line.len() as u64);
        write_head_counted(&file, None, &io)?;
        Ok(Self {
            file,
            header,
            index,
            head: None,
            rebuilt: false,
            lock,
            stale: false,
            reconciliations: 0,
            io,
            calls: Mutex::new(None),
            staged: None,
        })
    }

    /// Open an existing session file, loading or rebuilding its index and
    /// reading its persisted head. A file with no head beside it (one Pi
    /// wrote) takes its last entry as the head, as Pi does on reopen, and
    /// that head is persisted here and now, so a side leaf appended later
    /// ([`Session::append_beside`]) can never be taken for the head. What a
    /// part-way staged import of this id left beside it is removed first.
    pub fn open(file: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let file = file.into();
        let lock = SessionLock::take(&file)?;
        let io = IoCounter::default();
        clear_remainder(&file, &io)?;
        let (header, index, rebuilt) = Index::load_counted(&file, &io)?;
        let head = read_head(&file, &index)?;
        if let Some(id) = &head
            && index.row(id).is_none()
        {
            return Err(HomeError::UnknownEntry {
                session: header.id,
                id: id.clone(),
            });
        }
        if !Index::head_path(&file).is_file() {
            write_head_counted(&file, head.as_deref(), &io)?;
        }
        Ok(Self {
            file,
            header,
            index,
            head,
            rebuilt,
            lock,
            stale: false,
            reconciliations: 0,
            io,
            calls: Mutex::new(None),
            staged: None,
        })
    }

    /// Bring the index and head back in step with the file after an append
    /// failed part way. Runs before any other act when the session is stale;
    /// a session that cannot reconcile stays stale and refuses by name.
    pub(super) fn reconcile(&mut self) -> Result<(), HomeError> {
        if !self.stale {
            return Ok(());
        }
        self.refuse_staged()?;
        let (header, index, _) = Index::load_counted(&self.file, &self.io)?;
        let head = read_head(&self.file, &index)?;
        if let Some(id) = &head
            && index.row(id).is_none()
        {
            return Err(HomeError::UnknownEntry {
                session: header.id,
                id: id.clone(),
            });
        }
        self.header = header;
        self.index = index;
        self.head = head;
        self.forget_calls();
        self.stale = false;
        self.reconciliations += 1;
        Ok(())
    }

    /// A read on a stale session is refused by name rather than answered
    /// from an index that is behind the file.
    pub(super) fn fresh(&self) -> Result<(), HomeError> {
        if self.stale {
            return Err(HomeError::StaleIndex {
                path: self.file.clone(),
                reason: "an append left the index behind the file and reconciling failed; the next append or head move retries it",
            });
        }
        Ok(())
    }

    /// How many times this owner had to reconcile from the file.
    #[must_use]
    pub fn reconciliations(&self) -> u64 {
        self.reconciliations
    }

    /// The lock file this owner holds.
    #[must_use]
    pub fn lock_file(&self) -> &std::fs::File {
        self.lock.file()
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

    /// The head entry's id; `None` when the head is the header. Refused while
    /// the session is stale, like every read.
    pub fn head(&self) -> Result<Option<&str>, HomeError> {
        self.fresh()?;
        Ok(self.head.as_deref())
    }

    /// How many entries the file holds.
    pub fn len(&self) -> Result<usize, HomeError> {
        self.fresh()?;
        Ok(self.index.len())
    }

    /// Whether the file holds no entries.
    pub fn is_empty(&self) -> Result<bool, HomeError> {
        self.fresh()?;
        Ok(self.index.is_empty())
    }

    /// Whether opening rebuilt the index from the whole file.
    #[must_use]
    pub fn index_was_rebuilt(&self) -> bool {
        self.rebuilt
    }

    /// Whether an entry of this id is on record.
    pub fn contains(&self, id: &str) -> Result<bool, HomeError> {
        self.fresh()?;
        Ok(self.index.row(id).is_some())
    }

    /// Append an entry as a child of the head, with a fresh id and the current
    /// time, and advance the head to it. The entry line is durable before the
    /// index row, which is durable before the head.
    pub fn append(&mut self, body: EntryBody) -> Result<String, HomeError> {
        self.reconcile()?;
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
        self.append_line(entry)?;
        if let Err(e) = self.persist_head(Some(entry.id())) {
            self.stale = true;
            self.reconcile()?;
            return Err(e);
        }
        self.head = Some(entry.id().to_owned());
        Ok(())
    }

    /// The one durable write path: the entry line, then its index row, with
    /// the head left where it stands. [`Session::append_entry`] moves the head
    /// afterwards; [`Session::append_beside`] does not, so both reconcile the
    /// same way when a step after the line fails.
    pub(super) fn append_line(&mut self, entry: &Entry) -> Result<(), HomeError> {
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
        let line = to_line(entry, &self.file)?;
        let offset = self.index.end();
        self.write_line(&line)?;
        self.record_row(IndexRow {
            id: entry.id().to_owned(),
            parent: entry.parent_id().map(str::to_owned),
            offset,
            len: line.len() as u64,
            custom: custom_type_of(entry),
        })?;
        self.note_call(entry);
        Ok(())
    }

    /// Move the head to an entry on record (or to the header with `None`).
    /// Nothing in the file changes.
    pub fn move_head(&mut self, id: Option<&str>) -> Result<(), HomeError> {
        self.reconcile()?;
        if let Some(id) = id
            && self.index.row(id).is_none()
        {
            return Err(HomeError::UnknownEntry {
                session: self.header.id.clone(),
                id: id.to_owned(),
            });
        }
        // A head that could not be published (or whose directory sync failed)
        // may or may not stand on disk: reconcile from the file before answering.
        if let Err(e) = self.persist_head(id) {
            self.stale = true;
            self.reconcile()?;
            return Err(e);
        }
        self.head = id.map(str::to_owned);
        Ok(())
    }

    /// The positions in a root-to-head path of its context path, in the
    /// order [`Session::context_path`] gives them; `None` when the path holds
    /// no compaction and the context path is the whole path.
    pub(crate) fn context_positions<E: Borrow<Entry>>(path: &[E]) -> Option<Vec<usize>> {
        let ci = path
            .iter()
            .rposition(|e| matches!(e.borrow().body, EntryBody::Compaction { .. }))?;
        let EntryBody::Compaction {
            first_kept_entry_id,
            ..
        } = &path[ci].borrow().body
        else {
            return None;
        };
        let mut out = Vec::with_capacity(path.len());
        out.push(ci);
        if let Some(first) = path[..ci]
            .iter()
            .position(|e| e.borrow().id() == first_kept_entry_id)
        {
            out.extend(first..ci);
        }
        out.extend(ci + 1..path.len());
        Some(out)
    }

    /// Every entry of the file in file order, read through this owner's
    /// index with the file opened once.
    pub fn entries(&self) -> Result<Vec<Entry>, HomeError> {
        self.fresh()?;
        let rows: Vec<&IndexRow> = self.index.rows().iter().collect();
        let (entries, _) = self.index.read_rows_from(&self.file, &rows)?;
        self.io.read(entries.len());
        Ok(entries)
    }
}

pub(super) fn to_line<T: serde::Serialize>(value: &T, file: &Path) -> Result<String, HomeError> {
    let mut line = serde_json::to_string(value).map_err(|e| HomeError::Malformed {
        path: file.to_path_buf(),
        line: 0,
        what: "session line",
        reason: e.to_string(),
    })?;
    line.push('\n');
    Ok(line)
}
