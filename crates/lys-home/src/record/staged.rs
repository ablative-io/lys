//! A new session built under a staging name and published once (ADR-108).
//!
//! The import command builds its new session `<id>` in
//! `sessions/<id>.jsonl.importing`: the header and every entry are written
//! there with one write per line and no sync, and the index rows and the
//! head are held in memory. What an import writes beside the session, its
//! block rows, is named for the published file ([`Session::published_file`]),
//! never for the staging file. [`Session::publish`] then makes it durable in
//! this order: one sync of the staging file; the index rows written whole to
//! `<id>.index.jsonl` through a temporary file, synced and renamed; the head
//! written once to `<id>.head` the same way; one sync of the sessions
//! directory; the staging file renamed to `<id>.jsonl`; one more sync of the
//! directory. Five syncs of the session's own, whatever the record count.
//!
//! Until the rename no `<id>.jsonl` exists, so a failed or interrupted import
//! leaves no session. The next import of `<id>`, and the next
//! [`Session::open`] of `<id>.jsonl`, each holding the lock, remove
//! `<id>.jsonl.importing` when it is present and, when `<id>.jsonl` is
//! absent, `<id>.index.jsonl`, `<id>.head` and `<id>.blocks.jsonl`, and sync
//! the directory after any removal. A staging file is never listed as a
//! session.
//!
//! Every other append, and an import into a session already open, keeps the
//! per-append order: the line, then its index row, then the head, each
//! synced before the next is written. The write steps below choose between
//! the two, so the append paths in [`Session`] are the same for both.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::HomeError;
use crate::record::blocks::sync_dir;
use crate::record::entries::SessionHeader;
use crate::record::helpers::{PI_FORMAT_VERSION, now, write_durable};
use crate::record::home::Home;
use crate::record::index::{Index, IndexRow, place_head, write_head_counted};
use crate::record::io_counts::IoCounter;
use crate::record::lock::SessionLock;
use crate::record::session::{Session, to_line};

/// Where a staged session is written until it is published: the session
/// file's name with `.importing` after it.
#[must_use]
pub(crate) fn staging_path(session_file: &Path) -> PathBuf {
    let mut name = session_file.as_os_str().to_owned();
    name.push(".importing");
    PathBuf::from(name)
}

/// The session file a staged session publishes to, and the staging file
/// its lines are written to until then.
#[derive(Debug)]
pub(crate) struct Staged {
    file: PathBuf,
    out: fs::File,
}

impl Home {
    /// Stage a new session for an import: the lock `<id>.lock` taken, an
    /// existing `<id>.jsonl` refused by name, the remainder of an earlier
    /// part-way import removed, and the header written to the staging file.
    /// Nothing is synced until [`Session::publish`].
    pub(crate) fn stage_session(&self, id: &str, cwd: &str) -> Result<Session, HomeError> {
        let header = SessionHeader {
            version: Some(PI_FORMAT_VERSION),
            id: id.to_owned(),
            timestamp: now(),
            cwd: cwd.to_owned(),
            parent_session: None,
        };
        Session::stage(self.session_path(id)?, header)
    }
}

impl Session {
    fn stage(file: PathBuf, header: SessionHeader) -> Result<Self, HomeError> {
        let lock = SessionLock::take(&file)?;
        if file.exists() {
            return Err(HomeError::Exists { path: file });
        }
        let io = IoCounter::default();
        clear_remainder(&file, &io)?;
        let staging = staging_path(&file);
        let line = to_line(&header, &file)?;
        let mut out = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staging)
            .map_err(|e| HomeError::io("creating the staging file", &staging, e))?;
        out.write_all(line.as_bytes())
            .map_err(|e| HomeError::io("writing the header", &staging, e))?;
        let index = Index::new(&file, line.len() as u64);
        Ok(Self {
            file: staging,
            header,
            index,
            head: None,
            rebuilt: false,
            lock,
            stale: false,
            reconciliations: 0,
            io,
            calls: Mutex::new(None),
            staged: Some(Staged { file, out }),
        })
    }

    /// The file this session is read from once it is whole: a staged
    /// session's published name, any other session's own file.
    #[must_use]
    pub(crate) fn published_file(&self) -> &Path {
        self.staged
            .as_ref()
            .map_or(self.file.as_path(), |staged| staged.file.as_path())
    }

    /// Publish a staged session under its own name, in the order the module
    /// states, with five syncs. A session that is not staged has nothing to
    /// publish; a staged session that failed part way is refused, and its
    /// staging file is left for the next import or open of the id to remove.
    pub(crate) fn publish(&mut self) -> Result<(), HomeError> {
        self.fresh()?;
        let Some(staged) = &self.staged else {
            return Ok(());
        };
        let target = staged.file.clone();
        staged
            .out
            .sync_all()
            .map_err(|e| HomeError::io("syncing the staging file", &self.file, e))?;
        self.io.synced();
        self.index.place_all(&self.io)?;
        place_head(&target, self.head.as_deref(), &self.io)?;
        let dir = parent_dir(&target);
        sync_dir(dir)?;
        self.io.synced();
        fs::rename(&self.file, &target)
            .map_err(|e| HomeError::io("publishing the staged session", &target, e))?;
        sync_dir(dir)?;
        self.io.synced();
        self.file = target;
        self.staged = None;
        Ok(())
    }

    /// Write one entry line. A staged session writes it with no sync; any
    /// other appends it durably. A failure may leave none, some or all of
    /// the line on disk, so the session is stale until it has looked at the
    /// file; a staged one stays stale and refuses every later act.
    pub(super) fn write_line(&mut self, line: &str) -> Result<(), HomeError> {
        let written = match &mut self.staged {
            Some(staged) => staged
                .out
                .write_all(line.as_bytes())
                .map_err(|e| HomeError::io("appending an entry", &self.file, e)),
            None => write_durable(&self.file, line, &self.io),
        };
        if let Err(e) = written {
            self.stale = true;
            if self.staged.is_none() {
                self.reconcile()?;
            }
            return Err(e);
        }
        Ok(())
    }

    /// Record the row of a line just written. A staged session holds it in
    /// memory; any other appends it to the index file and syncs. From here
    /// the line is durable, so a failure leaves the index behind the file:
    /// mark stale and reconcile at once; if that fails too, the session
    /// stays stale until the next act reconciles it.
    pub(super) fn record_row(&mut self, row: IndexRow) -> Result<(), HomeError> {
        if self.staged.is_some() {
            self.index.push_row(row);
            return Ok(());
        }
        if let Err(e) = self.index.append(row, &self.io) {
            self.stale = true;
            self.reconcile()?;
            drop(e);
        }
        Ok(())
    }

    /// Persist the head: held in memory by a staged session until it is
    /// published, written whole and synced by any other.
    pub(super) fn persist_head(&self, id: Option<&str>) -> Result<(), HomeError> {
        if self.staged.is_some() {
            return Ok(());
        }
        write_head_counted(&self.file, id, &self.io)
    }

    /// A staged session that went stale cannot reconcile from a file that is
    /// not yet a session: it is refused by name, and the next import or open
    /// of its id removes what it left.
    pub(super) fn refuse_staged(&self) -> Result<(), HomeError> {
        if self.staged.is_some() {
            return Err(HomeError::StaleIndex {
                path: self.file.clone(),
                reason: "a staged import failed part way; the next import of this id removes it",
            });
        }
        Ok(())
    }
}

/// Remove what a part-way staged import of this session left: the staging
/// file when it is present and, when the session file is absent, its index,
/// head and block rows. The sessions directory is synced after any removal.
pub(super) fn clear_remainder(session_file: &Path, counts: &IoCounter) -> Result<(), HomeError> {
    let mut removed = remove_present(&staging_path(session_file))?;
    let present = session_file
        .try_exists()
        .map_err(|e| HomeError::io("looking for the session file", session_file, e))?;
    if !present {
        removed |= remove_present(&Index::index_path(session_file))?;
        removed |= remove_present(&Index::head_path(session_file))?;
        removed |= remove_present(&Index::blocks_path(session_file))?;
    }
    if removed {
        sync_dir(parent_dir(session_file))?;
        counts.synced();
    }
    Ok(())
}

/// Remove a file when it is there; `true` when one was removed.
fn remove_present(path: &Path) -> Result<bool, HomeError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(HomeError::io("removing a part-way import", path, e)),
    }
}

fn parent_dir(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}
