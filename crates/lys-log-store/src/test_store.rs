#![cfg(test)]
//! An in-memory [`LeafStore`] that counts the leaves read from it and the
//! appends made to it, so a test can say how much of a log a start touched
//! and how many durable acts a batch cost, rather than infer either from
//! timing.

use std::cell::Cell;

use lys_core::merkle::{AppendOnlyTree, RawLeaf};

use crate::error::{StoreError, StoreResult};
use crate::store::{LeafStore, PinnedRoot, batch_end};

pub(crate) const ORIGIN: &str = "example.com/lys/start-test";

/// The store's durable parts, kept apart from the handle so a test can
/// "crash" by dropping the handle and open a new one over the same parts.
#[derive(Debug, Clone, Default)]
pub(crate) struct Disk {
    pub(crate) leaves: Vec<Vec<u8>>,
    pub(crate) pinned: Option<PinnedRoot>,
    pub(crate) snapshot: Option<Vec<u8>>,
}

#[derive(Debug)]
pub(crate) struct CountingStore {
    pub(crate) disk: Disk,
    reads: Cell<u64>,
    appends: Cell<u64>,
    fail_next_append: bool,
}

impl CountingStore {
    pub(crate) fn new() -> Self {
        Self::over(Disk::default())
    }

    pub(crate) fn over(disk: Disk) -> Self {
        Self {
            disk,
            reads: Cell::new(0),
            appends: Cell::new(0),
            fail_next_append: false,
        }
    }

    /// How many leaves were read through this handle.
    pub(crate) fn reads(&self) -> u64 {
        self.reads.get()
    }

    /// How many durable acts this handle was asked for: one per
    /// [`LeafStore::append`], whatever the batch's size, as one flush is.
    pub(crate) fn appends(&self) -> u64 {
        self.appends.get()
    }

    /// Makes the next append fail at its one flush, so that nothing of it is
    /// stored: the store's parts are as they were before the call.
    pub(crate) fn fail_next_append(&mut self) {
        self.fail_next_append = true;
    }
}

impl LeafStore for CountingStore {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        u64::try_from(self.disk.leaves.len()).unwrap()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.reads.set(self.reads.get() + 1);
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| self.disk.leaves.get(i))
            .cloned())
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        if index != self.extent() {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent(),
            });
        }
        self.disk.leaves.push(bytes.to_vec());
        Ok(())
    }

    /// The leaves and their pin land together or not at all: the parts are
    /// changed only after every check has passed and the flush has not been
    /// made to fail.
    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        self.appends.set(self.appends.get() + 1);
        let end = batch_end(index, leaves.len())?;
        if index < self.extent() {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent() {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent(),
            });
        }
        if pin.tree_size != end {
            return Err(StoreError::PinNotOfAppend {
                end,
                tree_size: pin.tree_size,
            });
        }
        if std::mem::take(&mut self.fail_next_append) {
            return Err(StoreError::Io {
                context: "simulated power cut at the append's one flush".to_string(),
                source: std::io::Error::other("power cut"),
            });
        }
        self.disk
            .leaves
            .extend(leaves.iter().map(|bytes| bytes.to_vec()));
        self.disk.pinned = Some(pin);
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.disk.pinned.unwrap_or_else(|| {
            let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
            PinnedRoot { tree_size, root }
        })
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.disk.pinned = Some(pin);
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.disk.snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.disk.snapshot = Some(bytes.to_vec());
        Ok(())
    }
}
