//! The directory as the routes reach it: one locked section at a time, a
//! write run off the async workers, and a read of the projection answered
//! from the state the last finished section left, never waiting on a write
//! in progress.
//!
//! # Invariants
//!
//! - Every change to the directory runs inside [`DirectoryAccess::locked`],
//!   with the directory's mutex held. A route that writes runs that section
//!   on a blocking thread through `write_directory`, so an append and its
//!   fsync never occupy an async worker.
//! - Before the mutex is released, the section publishes the projection when
//!   the directory has folded a leaf the published one was not folded at,
//!   and withdraws it when the directory cannot answer its current state (an
//!   append uncertain, or the directory broken). A read that starts after a
//!   write was answered therefore sees that write, and a read during a write
//!   sees the state before it, which the write has not yet answered.
//! - A read with nothing published goes through the lock, where the
//!   directory settles or refuses by name as it always did.
//!
//! The directory's leaf store is held behind [`DirectoryStore`], so the
//! service runs over the file store and a test over a store it controls.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};

use lys_identity::projection::Projection;
use lys_identity::{Directory, IdentityError};
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreResult};

use crate::error::ServerError;
use crate::routes::AppState;

/// The directory's leaf store, whichever store it is.
pub struct DirectoryStore(Box<dyn LeafStore + Send>);

impl DirectoryStore {
    /// The directory's leaves kept in `store`.
    pub fn new(store: impl LeafStore + Send + 'static) -> Self {
        Self(Box::new(store))
    }

    /// The directory's leaves kept in the file store in the directory `dir`.
    pub fn open_file(dir: &Path) -> StoreResult<Self> {
        FileLeafStore::open(dir).map(Self::new)
    }
}

impl LeafStore for DirectoryStore {
    fn origin(&self) -> &str {
        self.0.origin()
    }

    fn extent(&self) -> u64 {
        self.0.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.0.leaf(index)
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.0.put_leaf(index, bytes)
    }

    fn pinned(&self) -> PinnedRoot {
        self.0.pinned()
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.0.pin(pin)
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.0.snapshot()
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.0.put_snapshot(bytes)
    }
}

/// The projection a finished section left, with the leaves folded into it.
struct Published {
    folded: u64,
    projection: Arc<Projection>,
}

/// The directory behind its mutex, and the projection the last section left.
pub struct DirectoryAccess {
    directory: Mutex<Directory<DirectoryStore>>,
    published: RwLock<Option<Published>>,
}

impl DirectoryAccess {
    /// Reach `directory`, publishing its projection as it was opened.
    pub fn new(directory: Directory<DirectoryStore>) -> Self {
        let access = Self {
            directory: Mutex::new(directory),
            published: RwLock::new(None),
        };
        access.publish(&access.lock());
        access
    }

    /// The directory, one caller at a time.
    fn lock(&self) -> MutexGuard<'_, Directory<DirectoryStore>> {
        self.directory
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Run `act` on the directory, one caller at a time, and publish what
    /// it leaves before the next caller runs.
    pub fn locked<T>(
        &self,
        act: impl FnOnce(&mut Directory<DirectoryStore>) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        let mut directory = self.lock();
        let answer = act(&mut directory);
        self.publish(&directory);
        answer
    }

    /// Publish the projection `directory` answers now, cloned only when it
    /// has folded a leaf since the last one published; withdraw it when the
    /// directory cannot answer without settling.
    fn publish(&self, directory: &Directory<DirectoryStore>) {
        let mut published = self
            .published
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let Some((folded, projection)) = directory.settled() else {
            *published = None;
            return;
        };
        if published.as_ref().map(|kept| kept.folded) == Some(folded) {
            return;
        }
        *published = Some(Published {
            folded,
            projection: Arc::new(projection.clone()),
        });
    }

    /// The projection the last section left, when it could answer one.
    pub fn published(&self) -> Option<Arc<Projection>> {
        self.published
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|kept| Arc::clone(&kept.projection))
    }
}

/// Run `act` on the directory as [`DirectoryAccess::locked`] does, on a
/// blocking thread, so the append it makes waits on no async worker.
pub(crate) async fn write_directory<T: Send + 'static>(
    state: &Arc<AppState>,
    act: impl FnOnce(&mut Directory<DirectoryStore>) -> Result<T, ServerError> + Send + 'static,
) -> Result<T, ServerError> {
    let state = Arc::clone(state);
    tokio::task::spawn_blocking(move || state.directory.locked(act))
        .await
        .map_err(|stopped| {
            ServerError::Identity(IdentityError::LogUnavailable {
                reason: format!("the directory write did not finish: {stopped}"),
            })
        })?
}

/// Answer `act` from the projection the last section left, waiting on no
/// write in progress; through the lock, where the directory settles or
/// refuses by name, when nothing is published.
pub(crate) fn read_directory<T>(
    state: &AppState,
    act: impl FnOnce(&Projection) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    if let Some(projection) = state.directory.published() {
        return act(&projection);
    }
    state
        .directory
        .locked(|directory| act(directory.projection()?))
}
