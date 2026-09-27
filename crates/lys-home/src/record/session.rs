//! One open session file: its lock, its index and head, the durable append
//! and the reads of the path from the head to the root.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::HomeError;
use crate::record::blocks;
use crate::record::entries::{Entry, EntryBase, EntryBody, SessionHeader};
use crate::record::helpers::{custom_type_of, fresh_id, now, write_durable};
use crate::record::index::{Index, IndexRow, read_head, write_head};

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
    pub(super) lock: std::fs::File,
    /// Set when an append left the index or head behind the file; cleared by
    /// [`Session::reconcile`], which runs before anything else is admitted.
    pub(super) stale: bool,
    /// How many times this owner reconciled from the file.
    pub(super) reconciliations: u64,
}

impl Session {
    /// Create a session file at `file` with this header.
    pub fn create(file: impl Into<PathBuf>, header: SessionHeader) -> Result<Self, HomeError> {
        let file = file.into();
        let lock = take_lock(&file)?;
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
            lock,
            stale: false,
            reconciliations: 0,
        })
    }

    /// Open an existing session file, loading or rebuilding its index and
    /// reading its persisted head. A file with no head beside it (one Pi
    /// wrote) takes its last entry as the head, as Pi does on reopen, and
    /// that head is persisted here and now, so a side leaf appended later
    /// ([`Session::append_beside`]) can never be taken for the head.
    pub fn open(file: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let file = file.into();
        let lock = take_lock(&file)?;
        let (header, index, rebuilt) = load_checked(&file)?;
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
            write_head(&file, head.as_deref())?;
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
        })
    }

    /// Bring the index and head back in step with the file after an append
    /// failed part way. Runs before any other act when the session is stale;
    /// a session that cannot reconcile stays stale and refuses by name.
    pub(super) fn reconcile(&mut self) -> Result<(), HomeError> {
        if !self.stale {
            return Ok(());
        }
        let (header, index, _) = load_checked(&self.file)?;
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
        &self.lock
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
        if let Err(e) = write_head(&self.file, Some(entry.id())) {
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
        // A failure to open, write or sync may leave none, some or all of the
        // line on disk: the session is stale until it has looked at the file.
        if let Err(e) = write_durable(&self.file, &line) {
            self.stale = true;
            self.reconcile()?;
            return Err(e);
        }
        // From here the line is durable. A failure below leaves the index or the
        // head behind the file: mark stale and reconcile at once; if that fails
        // too, the session stays stale until the next act reconciles it.
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
        if let Err(e) = write_head(&self.file, id) {
            self.stale = true;
            self.reconcile()?;
            return Err(e);
        }
        self.head = id.map(str::to_owned);
        Ok(())
    }

    /// One entry by id, read by seeking to it.
    pub fn entry(&self, id: &str) -> Result<Entry, HomeError> {
        self.fresh()?;
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
        self.fresh()?;
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
        self.fresh()?;
        let rows = self.index.rows_of_custom(custom_type);
        let (entries, _) = self.index.read_rows_from(&self.file, &rows)?;
        Ok(entries)
    }
}

/// Take the exclusive lock beside a session file, or refuse by name when
/// another owner holds it. The lock is advisory and per open file, so it
/// refuses a second owner in this process as well as in another.
fn take_lock(file: &Path) -> Result<std::fs::File, HomeError> {
    let path = Index::lock_path(file);
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| HomeError::io("opening the session lock", &path, e))?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(std::fs::TryLockError::WouldBlock) => Err(HomeError::SessionHeld {
            path: file.to_path_buf(),
        }),
        Err(std::fs::TryLockError::Error(e)) => Err(HomeError::io("locking the session", &path, e)),
    }
}

/// Load the index, rebuilding when it is missing or lags the file.
fn load_checked(file: &Path) -> Result<(SessionHeader, Index, bool), HomeError> {
    Index::load(file)
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
