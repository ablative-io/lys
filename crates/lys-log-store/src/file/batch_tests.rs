#![cfg(test)]
//! Batch directory flush count, uncertainty, and write-once conflicts.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{AfterLink, FileLeafStore, StoreError};
use crate::file::{remove_file, sync_dir};
use crate::{LeafStore, Log};

type Outcome = Result<(), Box<dyn std::error::Error>>;
static FLUSHES: AtomicUsize = AtomicUsize::new(0);

fn counted_flush(path: &std::path::Path) -> std::io::Result<()> {
    FLUSHES.fetch_add(1, Ordering::Relaxed);
    sync_dir(path)
}

fn refuse_flush(path: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::Error::other(format!(
        "injected batch directory flush at {}",
        path.display()
    )))
}

#[test]
fn one_directory_flush_covers_all_named_leaves_and_none_is_needed_for_empty_batch() -> Outcome {
    let temporary = tempfile::tempdir()?;
    FileLeafStore::create(temporary.path(), "example.com/lys/file-batch")?;
    let mut store = FileLeafStore::open(temporary.path())?;
    let after_link = AfterLink {
        remove_temp: remove_file,
        flush_dir: counted_flush,
    };
    store.put_batch_with(0, &[b"first", b"second", b"third"], &after_link)?;
    assert_eq!(FLUSHES.load(Ordering::Relaxed), 1);
    store.put_batch_with(3, &[], &after_link)?;
    assert_eq!(FLUSHES.load(Ordering::Relaxed), 1);
    assert_eq!(store.extent(), 3);
    assert_eq!(FileLeafStore::open(temporary.path())?.extent(), 3);
    assert_eq!(
        std::fs::read_dir(temporary.path().join("leaves"))?.count(),
        3
    );
    Ok(())
}

#[test]
fn failed_batch_directory_flush_refuses_further_leaves_and_pins_until_reopen() -> Outcome {
    let temporary = tempfile::tempdir()?;
    FileLeafStore::create(temporary.path(), "example.com/lys/file-batch")?;
    let mut store = FileLeafStore::open(temporary.path())?;
    let after_link = AfterLink {
        remove_temp: remove_file,
        flush_dir: refuse_flush,
    };
    assert!(matches!(
        store.put_batch_with(0, &[b"first", b"second"], &after_link),
        Err(StoreError::LeafDurabilityUncertain { index: 0, .. })
    ));
    assert_eq!(store.extent(), 2);
    assert!(matches!(
        store.put_leaf(2, b"later"),
        Err(StoreError::ReopenRequired { index: 0 })
    ));
    assert!(matches!(
        store.pin(store.pinned()),
        Err(StoreError::ReopenRequired { index: 0 })
    ));
    let mut reopened = FileLeafStore::open(temporary.path())?;
    reopened.put_leaf(2, b"later")?;
    assert_eq!(reopened.extent(), 3);
    Ok(())
}

#[test]
fn partial_batch_conflict_preserves_the_other_leaf_and_requires_reopen() -> Outcome {
    let temporary = tempfile::tempdir()?;
    FileLeafStore::create(temporary.path(), "example.com/lys/file-batch")?;
    let mut store = FileLeafStore::open(temporary.path())?;
    let occupied = temporary.path().join("leaves").join(format!("{:020}", 1));
    std::fs::write(&occupied, b"other writer")?;
    assert!(matches!(
        store.put_leaves(0, &[b"first", b"replacement"]),
        Err(StoreError::LeafAlreadyWritten { index: 1 })
    ));
    assert_eq!(std::fs::read(occupied)?, b"other writer");
    assert_eq!(store.leaf(0)?.as_deref(), Some(b"first".as_slice()));
    assert!(matches!(
        store.put_leaves(1, &[b"replacement"]),
        Err(StoreError::ReopenRequired { index: 0 })
    ));
    assert!(matches!(
        store.pin(store.pinned()),
        Err(StoreError::ReopenRequired { index: 0 })
    ));
    let reopened = FileLeafStore::open(temporary.path())?;
    assert_eq!(reopened.extent(), 2);
    assert_eq!(
        reopened.leaf(1)?.as_deref(),
        Some(b"other writer".as_slice())
    );
    assert_eq!(
        std::fs::read_dir(temporary.path().join("leaves"))?.count(),
        2
    );
    Ok(())
}

#[test]
fn old_state_opens_without_rewrite_and_the_next_batch_clears_its_optional_intent() -> Outcome {
    use base64::Engine;
    let temporary = tempfile::tempdir()?;
    let store = FileLeafStore::create(temporary.path(), "example.com/lys/old-state")?;
    let pin = store.pinned();
    drop(store);
    let legacy = format!(
        "{{\"tree_size\":0,\"root_hash\":\"{}\"}}\n",
        base64::engine::general_purpose::STANDARD.encode(pin.root)
    );
    let state = temporary.path().join("state.json");
    std::fs::write(&state, &legacy)?;
    let reader = FileLeafStore::open_read_only(temporary.path())?;
    assert_eq!(reader.pinned(), pin);
    assert_eq!(reader.batch_intent(), None);
    assert_eq!(std::fs::read(&state)?, legacy.as_bytes());
    let mut log = Log::open(FileLeafStore::open(temporary.path())?)?;
    assert_eq!(std::fs::read(&state)?, legacy.as_bytes());
    log.append_batch(&[b"first", b"second"])?;
    assert_eq!(log.store().batch_intent(), None);
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&state)?)?;
    assert!(saved.get("batch_end").is_none());
    assert_eq!(saved["tree_size"], 2);
    let reopened = Log::open(FileLeafStore::open(temporary.path())?)?;
    assert_eq!(reopened.tree().root(), log.tree().root());
    assert_eq!(reopened.recovered_to(), None);
    Ok(())
}

#[test]
fn intent_is_durable_before_leaves_and_an_empty_interruption_clears_it_on_recovery() -> Outcome {
    let temporary = tempfile::tempdir()?;
    let mut store = FileLeafStore::create(temporary.path(), "example.com/lys/intent")?;
    let pin = store.pinned();
    store.begin_batch(3)?;
    let state = temporary.path().join("state.json");
    let intent_bytes = std::fs::read(&state)?;
    let saved: serde_json::Value = serde_json::from_slice(&intent_bytes)?;
    assert_eq!(saved["batch_end"], 3);
    assert_eq!(saved["tree_size"], 0);
    assert_eq!(store.extent(), 0);
    assert_eq!(store.pinned(), pin);
    assert!(matches!(
        store.begin_batch(4),
        Err(StoreError::BatchIntentPending { end: 3 })
    ));
    drop(store);
    let reader = Log::open_at_pin(FileLeafStore::open_read_only(temporary.path())?)?;
    assert_eq!(reader.pending_repair(), Some(0));
    assert_eq!(std::fs::read(&state)?, intent_bytes);
    let log = Log::open(FileLeafStore::open(temporary.path())?)?;
    assert_eq!(log.recovered_to(), Some(0));
    assert_eq!(log.store().pinned(), pin);
    assert_eq!(log.store().batch_intent(), None);
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&state)?)?;
    assert!(saved.get("batch_end").is_none());
    Ok(())
}

#[test]
fn intent_refuses_a_read_only_store_and_cannot_authorize_an_existing_tail() -> Outcome {
    let temporary = tempfile::tempdir()?;
    let mut store = FileLeafStore::create(temporary.path(), "example.com/lys/intent")?;
    let mut reader = FileLeafStore::open_read_only(temporary.path())?;
    assert!(matches!(
        reader.begin_batch(2),
        Err(StoreError::ReadOnly { .. })
    ));
    store.put_leaves(0, &[b"first", b"second"])?;
    assert!(matches!(
        store.begin_batch(3),
        Err(StoreError::BatchStartUnpinned {
            extent: 2,
            pinned: 0
        })
    ));
    assert_eq!(store.batch_intent(), None);
    assert!(matches!(
        Log::open(store),
        Err(StoreError::PinMismatch { .. })
    ));
    Ok(())
}

#[test]
fn a_failed_intent_write_holds_the_store_before_any_leaf_is_written() -> Outcome {
    let temporary = tempfile::tempdir()?;
    let mut store = FileLeafStore::create(temporary.path(), "example.com/lys/intent")?;
    std::fs::create_dir(temporary.path().join("state.json.tmp"))?;
    assert!(matches!(store.begin_batch(2), Err(StoreError::Io { .. })));
    assert_eq!(store.extent(), 0);
    assert!(matches!(
        store.put_leaf(0, b"first"),
        Err(StoreError::ReopenRequired { index: 0 })
    ));
    assert!(matches!(
        store.pin(store.pinned()),
        Err(StoreError::ReopenRequired { index: 0 })
    ));
    assert_eq!(FileLeafStore::open(temporary.path())?.batch_intent(), None);
    Ok(())
}

#[test]
fn an_intent_that_does_not_extend_the_pin_is_corrupt_on_both_opens() -> Outcome {
    let temporary = tempfile::tempdir()?;
    FileLeafStore::create(temporary.path(), "example.com/lys/intent")?;
    let state = temporary.path().join("state.json");
    let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&state)?)?;
    saved["batch_end"] = 0.into();
    let bytes = serde_json::to_vec(&saved)?;
    std::fs::write(&state, &bytes)?;
    assert!(matches!(
        FileLeafStore::open_read_only(temporary.path()),
        Err(StoreError::Corrupt { .. })
    ));
    assert!(matches!(
        FileLeafStore::open(temporary.path()),
        Err(StoreError::Corrupt { .. })
    ));
    assert_eq!(std::fs::read(state)?, bytes);
    Ok(())
}
