//! File-backed tail reads and verification within the existing head lock.
//!
//! Each acquisition opens a fresh read-only handle, so the head it certifies
//! is the store's current one whichever writer moved it, and the witness is
//! bound to that handle alone. Verification holds that same handle's append
//! lock for its one reading: an append by any writer since the acquisition
//! shows there as [`StoreError::LeafAlreadyWritten`] before the reading is
//! called. Nothing is held between the two calls, and no lease is claimed.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::witness::{TailLeaf, TailWitness, TailWitnessProvider, validate};
use crate::{FileLeafStore, Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

/// The handle one acquisition read, and the owner its witness carries.
struct Reading {
    store: FileLeafStore,
    owner: Arc<()>,
}

/// An explicit file-backed witness owner, separate from ordinary leaf storage.
pub struct FileTailProvider {
    dir: PathBuf,
    reading: Mutex<Option<Arc<Reading>>>,
}

impl FileTailProvider {
    /// Bind a coherently opened file handle to a new ephemeral witness owner.
    /// Witnesses from a previous owner are never silently reused after reopen,
    /// and each acquisition replaces the reading the last one certified.
    #[must_use]
    pub fn new(store: FileLeafStore) -> Self {
        Self {
            dir: store.dir().to_path_buf(),
            reading: Mutex::new(Some(Arc::new(Reading {
                store,
                owner: Arc::new(()),
            }))),
        }
    }

    /// A provider for the store at `dir` that opens nothing until its first
    /// acquisition, so an owner that migrates or creates the store when it
    /// opens is not refused by a reader opened before it.
    #[must_use]
    pub fn at(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            reading: Mutex::new(None),
        }
    }

    fn current(&self) -> Option<Arc<Reading>> {
        self.reading
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(Arc::clone)
    }

    fn reopen(&self) -> StoreResult<Arc<Reading>> {
        let fresh = Arc::new(Reading {
            store: FileLeafStore::open_read_only(&self.dir)?,
            owner: Arc::new(()),
        });
        *self.reading.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&fresh));
        Ok(fresh)
    }
}

impl TailWitnessProvider for FileTailProvider {
    fn acquire(&self, settled: &Frontier) -> StoreResult<TailWitness> {
        let reading = self.reopen()?;
        reading.store.with_current_head(|store| {
            let upper = store.pinned();
            if upper.tree_size != store.extent() || settled.size() > upper.tree_size {
                return Err(StoreError::Corrupt {
                    path: store.dir().to_path_buf(),
                    reason: "tail reading bounds do not meet the current acknowledged head"
                        .to_owned(),
                });
            }
            let mut leaves = Vec::new();
            for index in settled.size()..upper.tree_size {
                let bytes = store
                    .leaf(index)?
                    .ok_or(StoreError::LeafMissingWithinExtent {
                        index,
                        extent: upper.tree_size,
                    })?;
                leaves.push(TailLeaf { index, bytes });
            }
            let witness = TailWitness {
                owner: Arc::clone(&reading.owner),
                origin: store.origin().to_owned(),
                lower: PinnedRoot {
                    tree_size: settled.size(),
                    root: settled.root(),
                },
                upper,
                leaves,
            };
            validate(&witness, settled, store.origin(), upper, store.dir())?;
            Ok(witness)
        })
    }

    fn verify(
        &self,
        witness: &TailWitness,
        settled: &Frontier,
        reading: &mut dyn FnMut(&TailWitness) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let Some(certified) = self
            .current()
            .filter(|certified| Arc::ptr_eq(&witness.owner, &certified.owner))
        else {
            return Err(StoreError::TailWitnessRefused {
                path: self.dir.clone(),
                reason: "tail witness belongs to a different reading owner",
            });
        };
        certified.store.with_current_head(|store| {
            validate(
                witness,
                settled,
                store.origin(),
                store.pinned(),
                store.dir(),
            )?;
            reading(witness)
        })
    }
}
