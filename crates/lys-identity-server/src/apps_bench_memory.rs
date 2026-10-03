//! Private, process-only leaves for throwaway bench questions. No receipt
//! from this store is published as durable history.
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use lys_log_store::{LeafStore, PinnedRoot, StoreError, StoreResult};
use std::sync::{Arc, Mutex, MutexGuard};

pub(crate) struct Kept {
    leaves: Vec<Vec<u8>>,
    pin: PinnedRoot,
    snapshot: Option<Vec<u8>>,
}

pub(crate) struct MemoryStore {
    kept: Arc<Mutex<Kept>>,
    origin: &'static str,
    extent: u64,
    pin: PinnedRoot,
}

impl MemoryStore {
    pub(crate) fn empty() -> Arc<Mutex<Kept>> {
        let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
        Arc::new(Mutex::new(Kept {
            leaves: Vec::new(),
            pin: PinnedRoot { root, tree_size },
            snapshot: None,
        }))
    }

    pub(crate) fn open(kept: Arc<Mutex<Kept>>, origin: &'static str) -> StoreResult<Self> {
        let held = kept.lock().map_err(poisoned)?;
        let extent = u64::try_from(held.leaves.len()).map_err(|error| StoreError::Io {
            context: "scratch_store_extent_overflow".to_owned(),
            source: std::io::Error::other(error),
        })?;
        let pin = held.pin;
        drop(held);
        Ok(Self {
            kept,
            origin,
            extent,
            pin,
        })
    }

    fn lock(&self) -> StoreResult<MutexGuard<'_, Kept>> {
        self.kept.lock().map_err(poisoned)
    }
}

fn poisoned(error: impl std::fmt::Display) -> StoreError {
    StoreError::Io {
        context: "scratch_store_poisoned".to_owned(),
        source: std::io::Error::other(error.to_string()),
    }
}

/// A pin the store admits after `held`: never backwards, and never a second
/// root at the size already pinned.
fn admits(held: PinnedRoot, pin: PinnedRoot) -> StoreResult<()> {
    if pin.tree_size < held.tree_size {
        return Err(StoreError::PinWentBackwards {
            pinned: held.tree_size,
            requested: pin.tree_size,
        });
    }
    if pin.tree_size == held.tree_size && pin.root != held.root {
        return Err(StoreError::PinRootChanged {
            tree_size: pin.tree_size,
            held: STANDARD.encode(held.root),
            offered: STANDARD.encode(pin.root),
        });
    }
    Ok(())
}

impl LeafStore for MemoryStore {
    fn origin(&self) -> &str {
        self.origin
    }

    fn extent(&self) -> u64 {
        self.extent
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        let held = self.lock()?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| held.leaves.get(index).cloned()))
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        let overflow = |error: &dyn std::fmt::Display| StoreError::Io {
            context: "scratch_store_extent_overflow".to_owned(),
            source: std::io::Error::other(error.to_string()),
        };
        let mut held = self.lock()?;
        let next = u64::try_from(held.leaves.len()).map_err(|error| overflow(&error))?;
        if index < next {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > next {
            return Err(StoreError::LeafWouldLeaveGap { index, next });
        }
        let count = u64::try_from(leaves.len()).map_err(|error| overflow(&error))?;
        let end = next
            .checked_add(count)
            .ok_or_else(|| overflow(&"the appended leaves exceed the store extent"))?;
        if pin.tree_size != end {
            return Err(StoreError::PinNotOfAppend {
                end,
                tree_size: pin.tree_size,
            });
        }
        admits(held.pin, pin)?;
        // One act under one lock: the leaves and their pin land together or
        // not at all, the memory shape of the file store's one write and flush.
        held.leaves
            .extend(leaves.iter().map(|bytes| bytes.to_vec()));
        held.pin = pin;
        drop(held);
        self.extent = end;
        self.pin = pin;
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.pin
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        let mut held = self.lock()?;
        admits(held.pin, pin)?;
        held.pin = pin;
        drop(held);
        self.pin = pin;
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.lock()?.snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.lock()?.snapshot = Some(bytes.to_vec());
        Ok(())
    }
}
