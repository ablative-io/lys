//! The one-broker lock on a store: `broker.lock` inside the store directory.
//!
//! Between processes the lock is a POSIX record lock over the whole lock
//! file. A record lock belongs to the process that took it, so within one
//! process it excludes nobody, and closing any descriptor of the lock file
//! releases it. Brokers in this process are therefore excluded by a registry
//! of held lock paths, consulted before the lock file is opened, so a second
//! broker here is refused without the file ever being opened a second time.

use std::collections::BTreeSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::error::SecretsError;
use crate::fsutil::io;

/// The lock file's name inside the store directory.
const LOCK: &str = "broker.lock";

/// The lock paths a broker in this process holds, each with its directory
/// resolved so two spellings of one store meet here.
static HELD: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// A lock path entered in [`HELD`]; dropping it removes the entry.
struct Claim(PathBuf);

impl Claim {
    /// Enter `path` in the registry, or `None` when a broker in this process
    /// already holds it.
    fn enter(path: PathBuf) -> Option<Self> {
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

/// The exclusive lock one broker holds on a store, released when it drops.
pub(super) struct StoreLock {
    /// Declared before `claim`, so it drops first: the descriptor closes, and
    /// the record lock with it, before the path leaves the registry.
    file: File,
    claim: Claim,
}

impl std::fmt::Debug for StoreLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreLock")
            .field("path", &self.claim.0)
            .field("file", &self.file)
            .finish()
    }
}

impl StoreLock {
    /// Take the lock on the store at `dir`, or refuse as `StoreLocked` when
    /// a broker in this process or in another holds it.
    pub(super) fn take(dir: &Path) -> Result<Self, SecretsError> {
        let locked = || SecretsError::StoreLocked {
            path: dir.to_path_buf(),
        };
        let resolved = std::fs::canonicalize(dir)
            .map_err(io(format!("resolving {}", dir.display())))?
            .join(LOCK);
        let claim = Claim::enter(resolved).ok_or_else(locked)?;
        let path = dir.join(LOCK);
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(io(format!("opening {}", path.display())))?;
        match rustix::fs::fcntl_lock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(Self { file, claim }),
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::ACCESS) => Err(locked()),
            Err(errno) => Err(SecretsError::Io {
                context: format!("locking {}", path.display()),
                source: std::io::Error::from(errno),
            }),
        }
    }
}
