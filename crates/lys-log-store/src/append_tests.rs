#![cfg(test)]
//! LYSLOGSTORE-008 R2 and R3: an append hands the store its leaves and their
//! pin as one act, a batch of any size is one such act, and a failed act
//! acknowledges nothing and holds the handle until reopen.

use crate::error::StoreError;
use crate::frontier::Frontier;
use crate::frontier_log::FrontierLog;
use crate::log::Log;
use crate::store::LeafStore;
use crate::test_store::{CountingStore, Disk};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn leaf(index: u64) -> Vec<u8> {
    format!("leaf-{index}").into_bytes()
}

#[test]
fn a_batch_of_a_thousand_leaves_is_one_act_on_the_store() -> Outcome {
    // R3: one call to the store, every index and hash answered, the pin at
    // the batch's end. One act is what one flush counts as on every backend.
    let leaves: Vec<Vec<u8>> = (0..1_000).map(leaf).collect();
    let borrowed: Vec<&[u8]> = leaves.iter().map(Vec::as_slice).collect();
    let (mut log, _tail) = FrontierLog::open(CountingStore::new())?;
    let appended = log.append_batch(&borrowed)?;
    assert_eq!(log.store().appends(), 1, "one durable act for the batch");
    assert_eq!(appended.len(), 1_000);
    for (offset, (index, hash)) in appended.iter().enumerate() {
        assert_eq!(*index, u64::try_from(offset)?);
        assert_eq!(*hash, crate::frontier::hash_leaf(&leaves[offset]));
    }
    let expected = Frontier::from_leaves(&leaves);
    let pinned = log.store().pinned();
    assert_eq!(pinned.tree_size, 1_000);
    assert_eq!(pinned.root, expected.root());
    assert_eq!(log.len(), 1_000);
    assert_eq!(log.root().to_parts(), (expected.root(), 1_000));
    Ok(())
}

#[test]
fn a_store_opened_after_appends_answers_the_frontier_that_wrote_them() -> Outcome {
    // R2: each append carries the size and root its leaf makes, so an open
    // finds exactly the frontier the writer held.
    let (mut log, _tail) = FrontierLog::open(CountingStore::new())?;
    for index in 0..7 {
        log.append(&leaf(index))?;
    }
    let (size, root) = (log.len(), log.root().to_parts().0);
    let disk = log.store().disk.clone();
    let (reopened, tail) = FrontierLog::open(CountingStore::over(disk))?;
    assert_eq!(reopened.len(), size);
    assert_eq!(reopened.root().to_parts().0, root);
    assert_eq!(reopened.store().pinned().tree_size, size);
    assert_eq!(reopened.store().pinned().root, root);
    assert_eq!(reopened.recovered_to(), None);
    assert_eq!(tail.leaves.len(), 7);
    Ok(())
}

#[test]
fn a_batch_whose_flush_fails_acknowledges_none_of_its_leaves_and_asks_for_a_reopen() -> Outcome {
    // R3: the batch's one flush fails; no leaf of it is acknowledged, the
    // store holds nothing of it, and the handle refuses every later act until
    // a fresh open, which finds nothing to repair.
    let (mut first, _tail) = FrontierLog::open(CountingStore::new())?;
    first.append_batch(&[&leaf(0), &leaf(1)])?;
    let mut store = CountingStore::over(first.store().disk.clone());
    store.fail_next_append();
    let (mut log, _tail) = FrontierLog::open(store)?;
    let batch: Vec<Vec<u8>> = (2..52).map(leaf).collect();
    let borrowed: Vec<&[u8]> = batch.iter().map(Vec::as_slice).collect();
    let err = log.append_batch(&borrowed).unwrap_err();
    assert!(matches!(err, StoreError::Io { .. }), "{err}");
    assert_eq!(log.len(), 2, "the frontier did not move");
    assert_eq!(log.store().extent(), 2, "the store kept no leaf of it");
    assert_eq!(log.store().pinned().tree_size, 2);
    assert!(matches!(log.append(&leaf(2)), Err(StoreError::Poisoned)));
    assert!(matches!(
        log.append_batch(&[&leaf(2)]),
        Err(StoreError::Poisoned)
    ));
    let disk = log.store().disk.clone();
    let (reopened, tail) = FrontierLog::open(CountingStore::over(disk))?;
    assert_eq!(reopened.len(), 2);
    assert_eq!(reopened.recovered_to(), None);
    assert_eq!(tail.leaves, vec![leaf(0), leaf(1)]);
    Ok(())
}

#[test]
fn the_leaf_holding_log_hands_the_store_leaves_and_pin_as_one_act_too() -> Outcome {
    let mut log = Log::open(CountingStore::new())?;
    let appended = log.append_batch(&[&leaf(0), &leaf(1), &leaf(2)])?;
    assert_eq!(log.store().appends(), 1);
    assert_eq!(appended.len(), 3);
    log.append(&leaf(3))?;
    assert_eq!(log.store().appends(), 2);
    let (root, size) = log.tree().root().to_parts();
    assert_eq!(size, 4);
    assert_eq!(log.store().pinned().tree_size, 4);
    assert_eq!(log.store().pinned().root, root);
    let disk: Disk = log.store().disk.clone();
    let reopened = Log::open(CountingStore::over(disk))?;
    assert_eq!(reopened.tree().root().to_parts(), (root, 4));
    assert_eq!(reopened.recovered_to(), None);
    Ok(())
}
