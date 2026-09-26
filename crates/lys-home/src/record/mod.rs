//! The home and its sessions.
//!
//! A [`Home`] is a directory: `sessions/<id>.jsonl` files in Pi's grammar,
//! each with its index and head beside it, and `blocks/` for content by hash.
//! A [`Session`] is one open session file: append a child of the head, move
//! the head, read the path from the head to the root by seeking, and build the
//! context path the way Pi's `buildSessionContext` does.
//!
//! One owner at a time: opening or creating a session takes an exclusive lock
//! on `<id>.lock` beside the file (held by the process for as long as the
//! [`Session`] lives), so two owners in one process or two processes cannot
//! both append with their own idea of where the file ends. A second opener is
//! refused by name ([`HomeError::SessionHeld`]).
//!
//! An append is durable in three steps: the entry line, then its index row,
//! then the head. When a later step fails after the line is durable, the
//! session reconciles itself from the file before it admits anything else
//! (the index is rebuilt by scanning, the head re-read), so what the process
//! believes about the file never runs ahead of or behind the disk.

pub mod beside;
#[cfg(test)]
mod beside_tests;
pub mod blocks;
#[cfg(test)]
mod blocks_tests;
pub mod call;
#[cfg(test)]
mod call_tests;
pub mod canon;
#[cfg(test)]
mod canon_tests;
pub mod entries;
pub mod epilogue;
#[cfg(test)]
mod epilogue_tests;
pub mod fork;
pub mod fork_cut;
#[cfg(test)]
pub(crate) mod fork_cut_tests;
pub mod fork_report;
#[cfg(test)]
pub(crate) mod fork_tests;
pub mod given;
#[cfg(test)]
mod given_tests;
pub mod index;
pub mod lantern;
#[cfg(test)]
mod lantern_tests;
pub mod reader;
#[cfg(test)]
pub(crate) mod reader_tests;
pub mod recall;
#[cfg(test)]
mod recall_tests;
#[cfg(test)]
mod record_tests;
pub mod templates;
#[cfg(test)]
mod templates_tests;

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::HomeError;
use crate::record::blocks::BlockStore;
use crate::record::entries::{Entry, EntryBase, EntryBody, SessionHeader};
use crate::record::index::{Index, IndexRow, read_head, write_head};
use crate::record::reader::SessionReader;
use crate::record::templates::TemplateStore;

/// The most bytes a session or block name may have.
pub const MAX_NAME_BYTES: usize = 200;

/// Check that a name is one safe path component: letters, digits, `.`, `_`
/// and `-`, not beginning with `.`, non-empty and at most [`MAX_NAME_BYTES`].
/// Every name that is joined onto a directory passes through here, so `..`,
/// `/` and an absolute path can never leave the directory chosen.
pub fn safe_component(what: &'static str, name: &str) -> Result<(), HomeError> {
    let ok = !name.is_empty()
        && name.len() <= MAX_NAME_BYTES
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(HomeError::BadName {
            what,
            name: name.to_owned(),
        })
    }
}

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

    /// A home at `root` for reading: nothing is created. A `root` that is
    /// absent or has no `sessions/` directory is refused by name, so a
    /// reader given the wrong path never makes a home there.
    pub fn read(root: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let root = root.into();
        if root.join("sessions").is_dir() {
            Ok(Self { root })
        } else {
            Err(HomeError::NoHome { path: root })
        }
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

    /// The template store of this home, `templates/` beside `sessions/` and
    /// `blocks/`; its directory appears when the first template is stored.
    #[must_use]
    pub fn templates(&self) -> TemplateStore {
        TemplateStore::at(self.root.join("templates"))
    }

    /// The path of a session file by id; an id that is not one safe path
    /// component is refused by name.
    pub fn session_path(&self, id: &str) -> Result<PathBuf, HomeError> {
        safe_component("session id", id)?;
        Ok(self.root.join("sessions").join(format!("{id}.jsonl")))
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
        Session::create(self.session_path(id)?, header)
    }

    /// Open a session by id.
    pub fn open_session(&self, id: &str) -> Result<Session, HomeError> {
        Session::open(self.session_path(id)?)
    }

    /// Read a session by id without owning it: no lock is taken and nothing
    /// is written beside the file (see [`SessionReader`]).
    pub fn read_session(&self, id: &str) -> Result<SessionReader, HomeError> {
        SessionReader::open(self.session_path(id)?)
    }

    /// The ids of every session under `sessions/`, in ascending byte order:
    /// each regular file named `<id>.jsonl`, except an index file, which is
    /// one named `<stem>.index.jsonl` whose first line is not a session
    /// header. A file that cannot be read while listing refuses by path.
    pub fn session_ids(&self) -> Result<Vec<String>, HomeError> {
        reader::session_ids(&self.root.join("sessions"))
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
    /// The exclusive lock on `<id>.lock`, held for the life of this owner.
    lock: std::fs::File,
    /// Set when an append left the index or head behind the file; cleared by
    /// [`Session::reconcile`], which runs before anything else is admitted.
    stale: bool,
    /// How many times this owner reconciled from the file.
    reconciliations: u64,
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
    fn reconcile(&mut self) -> Result<(), HomeError> {
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
    fn fresh(&self) -> Result<(), HomeError> {
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
    fn append_line(&mut self, entry: &Entry) -> Result<(), HomeError> {
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

/// Append a line to the session file and sync it.
fn write_durable(file: &Path, line: &str) -> Result<(), HomeError> {
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(file)
        .map_err(|e| HomeError::io("opening the session file", file, e))?;
    f.write_all(line.as_bytes())
        .map_err(|e| HomeError::io("appending an entry", file, e))?;
    f.sync_all()
        .map_err(|e| HomeError::io("syncing the session file", file, e))
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
