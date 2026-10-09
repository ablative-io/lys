//! The reversible writer preserves the old reader's bytes and durability.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::{FileLeafStore, Frontier, FrontierLog, LeafStore, StoreError};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn fixture(dir: &Path, leaves: &[&[u8]], pinned: usize) -> Outcome {
    std::fs::create_dir_all(dir.join("leaves"))?;
    std::fs::write(
        dir.join("log.json"),
        br#"{"format":"lys/log-dir/v1","origin":"example.com/legacy"}"#,
    )?;
    for (index, bytes) in leaves.iter().enumerate() {
        std::fs::write(dir.join("leaves").join(format!("{index:020}")), bytes)?;
    }
    let frontier = Frontier::from_leaves(&leaves[..pinned]);
    std::fs::write(
        dir.join("state.json"),
        serde_json::to_vec(&serde_json::json!({
            "tree_size": frontier.size(), "root_hash": STANDARD.encode(frontier.root()),
        }))?,
    )?;
    Ok(())
}

#[test]
fn deferred_legacy_append_is_durable_then_migrates_once() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("log");
    fixture(&dir, &[b"before"], 1)?;
    let ready = Arc::new(AtomicBool::new(false));
    let visits = Arc::new(AtomicU64::new(0));
    let ready_in = Arc::clone(&ready);
    let visits_in = Arc::clone(&visits);
    let store = FileLeafStore::open_deferred(
        &dir,
        Arc::new(move || {
            visits_in.fetch_add(1, Ordering::Relaxed);
            Ok(ready_in.load(Ordering::Acquire))
        }),
    )?;
    let (mut log, _) = FrontierLog::open(store)?;
    let before = crate::durability::flush_count();
    log.append(b"window")?;
    assert_eq!(crate::durability::flush_count() - before, 4);
    let old = super::v1::read(&dir)?;
    assert_eq!(old.leaves, [b"before".to_vec(), b"window".to_vec()]);
    assert_eq!(old.pinned.tree_size, 2);
    assert!(!dir.with_extension("v1").exists());
    ready.store(true, Ordering::Release);
    log.append(b"committed")?;
    assert_eq!(visits.load(Ordering::Relaxed), 3);
    log.append(b"after")?;
    assert_eq!(
        visits.load(Ordering::Relaxed),
        3,
        "v2 no longer checks intent"
    );
    assert_eq!(log.leaf_bytes(1)?, Some(b"window".to_vec()));
    let kept = log
        .store()
        .migrated()
        .ok_or("migration missing")?
        .kept
        .clone();
    assert_eq!(super::v1::read(&kept)?.leaves, old.leaves);
    drop(log);
    let (reopened, _) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
    assert_eq!(reopened.len(), 4);
    assert_eq!(reopened.leaf_bytes(3)?, Some(b"after".to_vec()));
    Ok(())
}

#[test]
fn legacy_interrupted_leaf_is_repaired_without_migration() -> Outcome {
    let root = tempfile::tempdir()?;
    fixture(root.path(), &[b"before", b"unacknowledged"], 1)?;
    let (log, _) = FrontierLog::open(FileLeafStore::open_deferred(
        root.path(),
        Arc::new(|| Ok(false)),
    )?)?;
    assert_eq!(log.recovered_to(), Some(2));
    assert_eq!(super::v1::read(root.path())?.pinned.tree_size, 2);
    assert!(log.store().deferred_migration().is_some());
    Ok(())
}

#[test]
fn deferred_legacy_refuses_duplicate_and_gap_before_checking_commit() -> Outcome {
    let root = tempfile::tempdir()?;
    fixture(root.path(), &[b"before"], 1)?;
    let mut store = FileLeafStore::open_deferred(root.path(), Arc::new(|| Ok(false)))?;
    let pin = store.pinned();
    assert!(matches!(
        store.append(0, &[b"replacement"], pin),
        Err(StoreError::LeafAlreadyWritten { .. })
    ));
    assert!(matches!(
        store.append(2, &[b"gap"], pin),
        Err(StoreError::LeafWouldLeaveGap { .. })
    ));
    assert_eq!(store.leaf(0)?, Some(b"before".to_vec()));
    Ok(())
}
