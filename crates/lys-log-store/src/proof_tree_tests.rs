#![cfg(test)]
//! Counting gates on the proof tree: it holds hashes only, is built holding
//! one leaf at a time, an append hashes its leaf once, and every proof it
//! gives is the leaf-holding tree's. Work is counted (reads through the store
//! double, leaf hashes through the crate's one hashing seam, hash bytes the
//! tree retains), never timed.

use std::cell::Cell;

use lys_core::merkle::{AppendOnlyTree, RawLeaf, verify_consistency, verify_inclusion_raw};

use super::*;
use crate::leaf_count::leaf_hashes;
use crate::test_store::{CountingStore, Disk};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn leaf(index: u64) -> Vec<u8> {
    format!("leaf-{index}").into_bytes()
}

/// A pinned store of `count` leaves, and the frontier of those leaves.
fn disk_of(count: u64) -> (Disk, Frontier) {
    let leaves: Vec<Vec<u8>> = (0..count).map(leaf).collect();
    let frontier = Frontier::from_leaves(&leaves);
    let pinned = PinnedRoot {
        tree_size: frontier.size(),
        root: frontier.root(),
    };
    let disk = Disk {
        leaves,
        pinned: Some(pinned),
        snapshot: None,
    };
    (disk, frontier)
}

/// A [`CountingStore`] that measures, at every read after [`Holding::watch`],
/// how many leaves its reader holds: those read and not yet hashed, and the
/// one being read. A reader that hashes each leaf before reading the next
/// holds one; a reader that collects the leaves first holds all of them.
struct Holding {
    inner: CountingStore,
    since: Cell<Option<(u64, u64)>>,
    peak: Cell<u64>,
}

impl Holding {
    fn over(disk: Disk) -> Self {
        Self {
            inner: CountingStore::over(disk),
            since: Cell::new(None),
            peak: Cell::new(0),
        }
    }

    /// Starts measuring from the reads and leaf hashes made so far.
    fn watch(&self) {
        self.since.set(Some((self.inner.reads(), leaf_hashes())));
        self.peak.set(0);
    }
}

impl LeafStore for Holding {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn extent(&self) -> u64 {
        self.inner.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        if let Some((reads, hashes)) = self.since.get() {
            let unhashed = (self.inner.reads() - reads) - (leaf_hashes() - hashes);
            self.peak.set(self.peak.get().max(unhashed + 1));
        }
        self.inner.leaf(index)
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        self.inner.append(index, leaves, pin)
    }

    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.inner.pin(pin)
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

#[test]
fn a_100_000_leaf_proof_tree_holds_hashes_only_and_is_built_one_leaf_at_a_time() -> Outcome {
    const LEAVES: u64 = 100_000;
    let (disk, frontier) = disk_of(LEAVES);
    let (log, _) = FrontierLog::resume(Holding::over(disk), frontier)?;
    log.store().watch();
    let hashed_before = leaf_hashes();
    let tree = log.proof_tree()?;
    assert_eq!(log.store().inner.reads(), LEAVES, "each leaf read once");
    assert_eq!(
        leaf_hashes() - hashed_before,
        LEAVES,
        "each leaf hashed once"
    );
    assert_eq!(
        log.store().peak.get(),
        1,
        "the build held more than one leaf at once"
    );
    let retained = u64::try_from(tree.retained_bytes())?;
    assert!(
        retained <= 2 * 32 * LEAVES,
        "the tree retains {retained} bytes for {LEAVES} leaves, more than two hashes each"
    );
    let proof = tree.prove_inclusion(77_777)?;
    verify_inclusion_raw(&log.root(), &leaf(77_777), 77_777, &proof)?;
    Ok(())
}

#[test]
fn each_append_to_a_built_proof_tree_hashes_its_leaf_once() -> Outcome {
    let (disk, frontier) = disk_of(12);
    let (mut log, _) = FrontierLog::resume(CountingStore::over(disk), frontier)?;
    log.proof_tree()?;
    let before = leaf_hashes();
    for index in 12..20 {
        log.append(&leaf(index))?;
    }
    assert_eq!(
        leaf_hashes() - before,
        8,
        "eight appends hash eight leaves: the tree takes the frontier's hash"
    );
    let tree = log.proof_tree()?;
    assert_eq!(
        log.store().reads(),
        12,
        "the built tree was extended, not rebuilt"
    );
    let mut verified = 0;
    for index in 0..20 {
        let proof = tree.prove_inclusion(index)?;
        verify_inclusion_raw(&log.root(), &leaf(index), index, &proof)?;
        verified += 1;
    }
    assert_eq!(verified, 20);
    Ok(())
}

#[test]
fn every_proof_the_log_gives_is_the_leaf_holding_trees_and_verifies() -> Outcome {
    let (mut inclusions, mut consistencies) = (0_u64, 0_u64);
    for size in [1_u64, 2, 3, 5, 8, 16, 17, 33, 64, 100] {
        let (disk, frontier) = disk_of(size);
        let (log, _) = FrontierLog::resume(CountingStore::over(disk), frontier)?;
        let tree = log.proof_tree()?;
        let base = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves((0..size).map(leaf));
        assert_eq!(tree.root()?, base.root());
        for index in 0..size {
            let proof = tree.prove_inclusion(index)?;
            let expected = base.prove_inclusion(index)?;
            assert_eq!(proof.as_bytes(), expected.as_bytes(), "{index} of {size}");
            verify_inclusion_raw(&log.root(), &leaf(index), index, &proof)?;
            inclusions += 1;
        }
        for old in 1..=size {
            let proof = tree.prove_consistency(old, size)?;
            let expected = base.prove_consistency(old, size)?;
            assert_eq!(proof.as_bytes(), expected.as_bytes(), "{old} to {size}");
            let old_root = Frontier::from_leaves((0..old).map(leaf)).root_hash();
            verify_consistency(&old_root, &log.root(), &proof)?;
            consistencies += 1;
        }
    }
    assert_eq!(inclusions, 249);
    assert_eq!(consistencies, 249);
    Ok(())
}
