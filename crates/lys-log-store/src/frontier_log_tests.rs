#![cfg(test)]
//! Gates on the frontier-only log: the same roots and pin rules as [`Log`],
//! with a resumed open that reads only the leaves after its frontier.
//!
//! [`Log`]: crate::Log

use lys_core::merkle::{AppendOnlyTree, RawLeaf, verify_inclusion_raw};

use super::*;
use crate::test_store::{CountingStore, Disk};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn leaf(index: u64) -> Vec<u8> {
    format!("leaf-{index}").into_bytes()
}

fn filled(count: u64) -> Result<Disk, StoreError> {
    let (mut log, _) = FrontierLog::open(CountingStore::new())?;
    for index in 0..count {
        log.append(&leaf(index))?;
    }
    Ok(log.store.disk)
}

#[test]
fn an_open_from_nothing_reads_every_leaf_and_reaches_the_full_trees_root() -> Outcome {
    let disk = filled(9)?;
    let (log, tail) = FrontierLog::open(CountingStore::over(disk))?;
    assert_eq!(log.store().reads(), 9);
    assert_eq!(tail.from, 0);
    assert_eq!(tail.leaves.len(), 9);
    assert_eq!(
        log.root(),
        AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves((0..9).map(leaf)).root()
    );
    assert_eq!(log.recovered_to(), None);
    Ok(())
}

#[test]
fn a_resumed_open_reads_only_the_leaves_after_its_frontier() -> Outcome {
    let disk = filled(20)?;
    let frontier = Frontier::from_leaves((0..16).map(leaf));
    let (log, tail) = FrontierLog::resume(CountingStore::over(disk), frontier)?;
    assert_eq!(log.store().reads(), 4, "leaves 16 to 19 and nothing else");
    assert_eq!(tail.from, 16);
    assert_eq!(tail.leaves, (16..20).map(leaf).collect::<Vec<_>>());
    assert_eq!(log.len(), 20);
    Ok(())
}

#[test]
fn a_resumed_open_whose_frontier_is_another_logs_is_a_pin_mismatch() -> Outcome {
    let disk = filled(10)?;
    let other = Frontier::from_leaves((100..106).map(leaf));
    let err = FrontierLog::resume(CountingStore::over(disk), other).unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
    Ok(())
}

#[test]
fn an_append_whose_flush_fails_stores_nothing_and_holds_the_handle() -> Outcome {
    let disk = filled(5)?;
    let mut store = CountingStore::over(disk);
    store.fail_next_append();
    let (mut log, _) = FrontierLog::resume(store, Frontier::from_leaves((0..5).map(leaf)))?;
    let err = log.append(&leaf(5)).unwrap_err();
    assert!(matches!(err, StoreError::Io { .. }), "{err}");
    assert_eq!(log.len(), 5, "nothing was acknowledged");
    assert!(matches!(log.append(&leaf(6)), Err(StoreError::Poisoned)));
    let disk = log.store.disk;
    assert_eq!(
        disk.leaves.len(),
        5,
        "the store kept no leaf of the failed append"
    );
    assert_eq!(disk.pinned.map(|pin| pin.tree_size), Some(5));
    let (log, tail) = FrontierLog::resume(
        CountingStore::over(disk),
        Frontier::from_leaves((0..5).map(leaf)),
    )?;
    assert_eq!(log.recovered_to(), None, "a reopen finds nothing to repair");
    assert!(tail.leaves.is_empty());
    assert_eq!(log.store().pinned().tree_size, 5);
    Ok(())
}

#[test]
fn the_proof_tree_is_built_on_first_use_and_proves_inclusion() -> Outcome {
    let disk = filled(12)?;
    let (mut log, _tail) = FrontierLog::resume(
        CountingStore::over(disk),
        Frontier::from_leaves((0..12).map(leaf)),
    )?;
    assert_eq!(log.store().reads(), 0, "nothing read at open");
    let proof = log.proof_tree()?.prove_inclusion(3)?;
    assert!(verify_inclusion_raw(&log.root(), &leaf(3), 3, &proof).is_ok());
    log.append(&leaf(12))?;
    let proof = log.proof_tree()?.prove_inclusion(12)?;
    assert!(verify_inclusion_raw(&log.root(), &leaf(12), 12, &proof).is_ok());
    assert_eq!(log.store().reads(), 12, "the tree was built once");
    Ok(())
}

#[test]
fn a_leaf_damaged_before_the_frontier_is_refused_when_the_proof_tree_is_built() -> Outcome {
    let mut disk = filled(8)?;
    disk.leaves[2] = b"not leaf-2".to_vec();
    let (log, _tail) = FrontierLog::resume(
        CountingStore::over(disk),
        Frontier::from_leaves((0..8).map(leaf)),
    )?;
    let err = log.proof_tree().unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
    Ok(())
}

#[test]
fn leaf_bytes_reads_from_the_store_and_answers_none_past_the_end() -> Outcome {
    let disk = filled(3)?;
    let (log, _tail) = FrontierLog::resume(
        CountingStore::over(disk),
        Frontier::from_leaves((0..3).map(leaf)),
    )?;
    assert_eq!(log.leaf_bytes(1)?, Some(leaf(1)));
    assert_eq!(log.leaf_bytes(3)?, None);
    assert_eq!(log.leaves_from(1)?.leaves, vec![leaf(1), leaf(2)]);
    Ok(())
}

#[test]
fn a_poisoned_log_writes_no_snapshot() -> Outcome {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("key"))?;
    let mut store = CountingStore::new();
    store.fail_next_append();
    let (mut log, _) = FrontierLog::open(store)?;
    assert!(log.append(&leaf(0)).is_err());
    let err = log
        .write_snapshot("test/state", b"state", &key)
        .unwrap_err();
    assert!(matches!(err, StoreError::Poisoned), "{err}");
    assert_eq!(log.store.disk.snapshot, None);
    Ok(())
}
