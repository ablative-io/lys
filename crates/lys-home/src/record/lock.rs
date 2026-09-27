//! The one-owner lock on a session: `<id>.lock` beside the session file.
//!
//! Between processes the lock is a POSIX record lock (`fcntl` with
//! `F_SETLK`) over the whole lock file. A record lock belongs to the process
//! that took it, not to the open file description, so no child this process
//! spawns ever holds it. A `flock` lock is shared by every duplicate of the
//! descriptor: a child spawned on another thread while a session was open
//! carried a duplicate from the spawn until its exec (or for its whole life,
//! when the descriptor was passed down), and the session stayed held after
//! its owner had let go.
//!
//! Within a process a record lock excludes nobody, and closing any descriptor
//! of the lock file releases it. Owners in this process are therefore
//! excluded by a registry of held lock paths, consulted before the lock file
//! is opened, so a second owner here is refused without the file ever being
//! opened a second time. The descriptor is closed before its path leaves the
//! registry, so a new owner never takes the lock while an old descriptor of
//! it can still be closed.

use std::collections::BTreeSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::error::HomeError;
use crate::record::index::Index;

/// The lock paths an owner in this process holds, resolved through their
/// directory so two spellings of one path meet here.
static HELD: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// The exclusive lock one session owner holds, released when it drops.
pub(crate) struct SessionLock {
    /// Declared before `claim`, so it drops first: the descriptor closes, and
    /// the record lock with it, before the path leaves the registry.
    file: File,
    claim: Claim,
}

/// A lock path entered in [`HELD`]; dropping it removes the entry.
struct Claim(PathBuf);

impl Claim {
    /// Enter `path` in the registry, or `None` when an owner in this process
    /// already holds it.
    fn enter(path: PathBuf) -> Option<Self> {
        // The guard is released at the end of this statement, before any
        // claim exists whose drop would take it again.
        let entered = HELD
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.clone());
        entered.then(|| Self(path))
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        HELD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.0);
    }
}

impl std::fmt::Debug for SessionLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLock")
            .field("path", &self.claim.0)
            .field("file", &self.file)
            .finish()
    }
}

impl SessionLock {
    /// Take the exclusive lock beside `session_file`, or refuse by name when
    /// another owner holds it, naming the holding process when the lock does.
    pub(crate) fn take(session_file: &Path) -> Result<Self, HomeError> {
        let path = Index::lock_path(session_file);
        let claim = Claim::enter(resolved(&path)?).ok_or_else(|| HomeError::SessionHeld {
            path: session_file.to_path_buf(),
            holder: Some(std::process::id()),
        })?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| HomeError::io("opening the session lock", &path, e))?;
        lock_exclusive(&file, &path, session_file)?;
        Ok(Self { file, claim })
    }

    /// The open lock file.
    pub(crate) fn file(&self) -> &File {
        &self.file
    }
}

/// The lock path with its directory resolved, the registry's key.
fn resolved(path: &Path) -> Result<PathBuf, HomeError> {
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let dir = std::fs::canonicalize(dir)
        .map_err(|e| HomeError::io("resolving the session directory", dir, e))?;
    Ok(match path.file_name() {
        Some(name) => dir.join(name),
        None => dir,
    })
}

/// Take the process's record lock on the whole lock file without waiting.
#[cfg(unix)]
fn lock_exclusive(file: &File, path: &Path, session_file: &Path) -> Result<(), HomeError> {
    use rustix::fs::{FlockOperation, fcntl_lock};
    use rustix::io::Errno;
    match fcntl_lock(file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(()),
        Err(e) if e == Errno::AGAIN || e == Errno::ACCESS => Err(HomeError::SessionHeld {
            path: session_file.to_path_buf(),
            holder: holder(file),
        }),
        Err(e) => Err(HomeError::io("locking the session", path, e.into())),
    }
}

/// The process holding a record lock on the file, when one still does.
#[cfg(unix)]
fn holder(file: &File) -> Option<u32> {
    use rustix::process::{Flock, FlockType, fcntl_getlk};
    let conflict = fcntl_getlk(file, &Flock::from(FlockType::WriteLock)).ok()??;
    u32::try_from(conflict.pid?.as_raw_nonzero().get()).ok()
}

/// Take the exclusive lock on the lock file's handle without waiting.
#[cfg(not(unix))]
fn lock_exclusive(file: &File, path: &Path, session_file: &Path) -> Result<(), HomeError> {
    match file.try_lock() {
        Ok(()) => Ok(()),
        Err(std::fs::TryLockError::WouldBlock) => Err(HomeError::SessionHeld {
            path: session_file.to_path_buf(),
            holder: None,
        }),
        Err(std::fs::TryLockError::Error(e)) => Err(HomeError::io("locking the session", path, e)),
    }
}
