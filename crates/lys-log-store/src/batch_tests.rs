#![cfg(test)]
//! Batch acknowledgement, failures, and verified recovery.

use lys_core::merkle::{AppendOnlyTree, RawLeaf, raw_leaf_hash};

use crate::test_store::{CountingStore, Disk};
use crate::{FrontierLog, LeafStore, Log, PinnedRoot, StoreError, StoreResult};

type Outcome = Result<(), Box<dyn std::error::Error>>;

struct Store {
    inner: CountingStore,
    pins: usize,
    fail_after: Option<usize>,
}

impl Store {
    fn new(fail_after: Option<usize>) -> Self {
        Self {
            inner: CountingStore::new(),
            pins: 0,
            fail_after,
        }
    }
}

impl LeafStore for Store {
    fn origin(&self) -> &str {
        self.inner.origin()
    }
    fn extent(&self) -> u64 {
        self.inner.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_leaf(index, bytes)
    }
    fn put_leaves(&mut self, index: u64, leaves: &[&[u8]]) -> StoreResult<()> {
        let end = crate::store::batch_end(index, leaves.len())?;
        for (offset, (index, bytes)) in (index..end).zip(leaves).enumerate() {
            if self.fail_after == Some(offset) {
                return Err(StoreError::Io {
                    context: "injected batch write failure".to_owned(),
                    source: std::io::Error::other("batch write failed"),
                });
            }
            self.inner.put_leaf(index, bytes)?;
        }
        Ok(())
    }
    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.pins += 1;
        self.inner.pin(pin)
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

const LEAVES: [&[u8]; 4] = [b"first", b"", b"third", b"fourth"];

#[test]
fn batch_roots_bytes_and_indices_match_individual_appends_with_one_pin() -> Outcome {
    let mut full = Log::open(Store::new(None))?;
    let (mut frontier, _) = FrontierLog::open(Store::new(None))?;
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for leaf in LEAVES {
        tree.append_raw(leaf);
    }
    let expected: Vec<_> = (0..4)
        .zip(LEAVES)
        .map(|(index, bytes)| (index, raw_leaf_hash(bytes)))
        .collect();
    assert_eq!(full.append_batch(&LEAVES)?, expected);
    assert_eq!(frontier.append_batch(&LEAVES)?, expected);
    assert_eq!(full.tree().root(), tree.root());
    assert_eq!(frontier.root(), tree.root());
    assert_eq!(full.store().pins, 1);
    assert_eq!(frontier.store().pins, 1);
    for (index, bytes) in (0..4).zip(LEAVES) {
        assert_eq!(full.store().leaf(index)?.as_deref(), Some(bytes));
        assert_eq!(frontier.leaf_bytes(index)?.as_deref(), Some(bytes));
    }
    assert!(full.append_batch(&[])?.is_empty());
    assert!(frontier.append_batch(&[])?.is_empty());
    assert_eq!(full.store().pins, 1);
    assert_eq!(frontier.store().pins, 1);
    Ok(())
}

#[test]
fn failed_partial_batches_hold_both_logs_and_recover_only_the_written_prefix() -> Outcome {
    for count in 0..LEAVES.len() {
        let mut full = Log::open(Store::new(Some(count)))?;
        let (mut frontier, _) = FrontierLog::open(Store::new(Some(count)))?;
        assert!(matches!(
            full.append_batch(&LEAVES),
            Err(StoreError::Io { .. })
        ));
        assert!(matches!(
            frontier.append_batch(&LEAVES),
            Err(StoreError::Io { .. })
        ));
        assert!(matches!(
            full.append_batch(&LEAVES),
            Err(StoreError::Poisoned)
        ));
        assert!(matches!(
            frontier.append_batch(&LEAVES),
            Err(StoreError::Poisoned)
        ));
        assert_eq!(full.store().pins, 0);
        assert_eq!(frontier.store().pins, 0);
        let size = u64::try_from(count)?;
        assert_eq!(full.store().extent(), size);
        assert_eq!(frontier.store().extent(), size);
        let reopened = Log::open(CountingStore::over(full.store().inner.disk.clone()))?;
        let (resumed, _) =
            FrontierLog::open(CountingStore::over(frontier.store().inner.disk.clone()))?;
        assert_eq!(reopened.tree().len(), size);
        assert_eq!(resumed.len(), size);
        assert_eq!(reopened.tree().root(), resumed.root());
        let repaired = (count > 0).then_some(size);
        assert_eq!(reopened.recovered_to(), repaired);
        assert_eq!(resumed.recovered_to(), repaired);
    }
    Ok(())
}

#[test]
fn failed_final_pin_holds_a_complete_batch_until_reopen() -> Outcome {
    let mut store = Store::new(None);
    store.inner.fail_next_pin();
    let (mut log, _) = FrontierLog::open(store)?;
    assert!(matches!(
        log.append_batch(&LEAVES),
        Err(StoreError::Io { .. })
    ));
    assert!(matches!(log.append(b"later"), Err(StoreError::Poisoned)));
    assert_eq!(log.store().pinned().tree_size, 0);
    assert_eq!(log.store().extent(), 4);
    let (reopened, _) = FrontierLog::open(CountingStore::over(log.store().inner.disk.clone()))?;
    assert_eq!(reopened.recovered_to(), Some(4));
    assert_eq!(reopened.store().pinned().tree_size, 4);
    Ok(())
}

#[test]
fn batch_overflow_is_refused_before_any_leaf_is_written() {
    let mut store = CountingStore::new();
    assert!(matches!(
        store.put_leaves(u64::MAX, &LEAVES),
        Err(StoreError::BatchSizeOverflow { .. })
    ));
    assert_eq!(store.extent(), 0);
}

#[test]
fn extra_leaves_never_excuse_a_corrupt_pinned_prefix() -> Outcome {
    let mut store = Store::new(None);
    let (mut log, _) = FrontierLog::open(store)?;
    log.append(b"pinned")?;
    let pin = log.store().pinned();
    store = Store::new(None);
    store.inner.disk = log.store().inner.disk.clone();
    store.put_leaves(1, &LEAVES)?;
    let mut disk: Disk = store.inner.disk;
    disk.leaves[0] = b"changed".to_vec();
    assert!(matches!(
        Log::open(CountingStore::over(disk.clone())),
        Err(StoreError::PinMismatch { .. })
    ));
    assert!(matches!(
        FrontierLog::open(CountingStore::over(disk)),
        Err(StoreError::PinMismatch { .. })
    ));
    assert_eq!(pin.tree_size, 1);
    Ok(())
}
