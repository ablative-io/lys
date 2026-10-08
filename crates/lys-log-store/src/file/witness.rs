//! File-backed tail reads and verification within the existing head lock.

use crate::witness::{TailLeaf, TailWitness, TailWitnessProvider, validate};
use crate::{FileLeafStore, Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};
use std::sync::Arc;

/// An explicit file-backed witness owner, separate from ordinary leaf storage.
pub struct FileTailProvider {
    store: FileLeafStore,
    owner: Arc<()>,
}

impl FileTailProvider {
    /// Bind a coherently opened file handle to a new ephemeral witness owner.
    /// Witnesses from a previous owner are never silently reused after reopen.
    #[must_use]
    pub fn new(store: FileLeafStore) -> Self {
        Self { store, owner: Arc::new(()) }
    }
}

impl TailWitnessProvider for FileTailProvider {
    fn acquire(&self, settled: &Frontier) -> StoreResult<TailWitness> {
        self.store.with_current_head(|store| {
            let upper = store.pinned();
            if upper.tree_size != store.extent() || settled.size() > upper.tree_size {
                return Err(StoreError::Corrupt {
                    path: store.dir().to_path_buf(),
                    reason: "tail reading bounds do not meet the current acknowledged head".to_owned(),
                });
            }
            let mut leaves = Vec::new();
            for index in settled.size()..upper.tree_size {
                let bytes = store.leaf(index)?.ok_or(StoreError::LeafMissingWithinExtent {
                    index, extent: upper.tree_size,
                })?;
                leaves.push(TailLeaf { index, bytes });
            }
            let witness = TailWitness {
                owner: Arc::clone(&self.owner),
                origin: store.origin().to_owned(),
                lower: PinnedRoot { tree_size: settled.size(), root: settled.root() },
                upper, leaves,
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
        if !Arc::ptr_eq(&witness.owner, &self.owner) {
            return Err(StoreError::Corrupt {
                path: self.store.dir().to_path_buf(),
                reason: "tail witness belongs to a different reading owner".to_owned(),
            });
        }
        self.store.with_current_head(|store| {
            validate(witness, settled, store.origin(), store.pinned(), store.dir())?;
            reading(witness)
        })
    }
}
