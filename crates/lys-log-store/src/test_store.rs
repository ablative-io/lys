#![cfg(test)]
//! An in-memory [`LeafStore`] that counts the leaves read from it, so a test
//! can say how much of a log a start touched rather than infer it from timing.

use std::cell::Cell;

use lys_core::merkle::{AppendOnlyTree, RawLeaf};

use crate::error::{StoreError, StoreResult};
use crate::store::{LeafStore, PinnedRoot};

pub(crate) const ORIGIN: &str = "example.com/lys/start-test";

/// The store's durable parts, kept apart from the handle so a test can
/// "crash" by dropping the handle and open a new one over the same parts.
#[derive(Clone, Default)]
pub(crate) struct Disk {
    pub(crate) leaves: Vec<Vec<u8>>,
    pub(crate) pinned: Option<PinnedRoot>,
    pub(crate) snapshot: Option<Vec<u8>>,
}

pub(crate) struct CountingStore {
    pub(crate) disk: Disk,
    reads: Cell<u64>,
    fail_next_pin: bool,
}

impl CountingStore {
    pub(crate) fn new() -> Self {
        Self::over(Disk::default())
    }

    pub(crate) fn over(disk: Disk) -> Self {
        Self {
            disk,
            reads: Cell::new(0),
            fail_next_pin: false,
        }
    }

    /// How many leaves were read through this handle.
    pub(crate) fn reads(&self) -> u64 {
        self.reads.get()
    }

    /// Makes the next pin fail after its leaf was stored.
    pub(crate) fn fail_next_pin(&mut self) {
        self.fail_next_pin = true;
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

    fn pinned(&self) -> PinnedRoot {
        self.disk.pinned.unwrap_or_else(|| {
            let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
            PinnedRoot { tree_size, root }
        })
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        if std::mem::take(&mut self.fail_next_pin) {
            return Err(StoreError::Io {
                context: "simulated crash before the pin".to_string(),
                source: std::io::Error::other("power cut"),
            });
        }
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
