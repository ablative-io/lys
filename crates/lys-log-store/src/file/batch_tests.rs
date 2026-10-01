#![cfg(test)]
//! Batch directory flush count, uncertainty, and write-once conflicts.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{AfterLink, FileLeafStore, StoreError};
use crate::LeafStore;
use crate::file::{remove_file, sync_dir};

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
