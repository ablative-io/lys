//! The access requests start from their signed snapshot and read only the
//! leaves after it; leaves written before leaves were pinned are pinned once.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::requests_store::{Asked, ORIGIN, RequestStore};
use lys_log_store::{FileLeafStore, LeafStore, Start};

type TestResult = Result<(), Box<dyn Error>>;

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("requests.key"),
    )?))
}

fn asked(id: &str) -> Asked {
    Asked {
        id: id.to_owned(),
        asked_by: "person-a".to_owned(),
        responsible: "person-a".to_owned(),
        resource_kind: "doc".to_owned(),
        resource_id: "1".to_owned(),
        relation: "beta".to_owned(),
        ends_at: None,
        why: "to read".to_owned(),
        asked_at: 5,
    }
}

#[test]
fn a_restart_reads_only_the_requests_after_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests");
    let store = RequestStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 0
        }
    );
    for id in ["op-1", "op-2", "op-3"] {
        store.ask(asked(id))?;
    }
    drop(store);

    let store = RequestStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 3
        }
    );
    assert_eq!(store.requests().count(), 3);
    assert_eq!(store.snapshot_failure(), None);
    Ok(())
}

#[test]
fn a_snapshot_from_another_key_is_refused_and_every_leaf_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests");
    let mut store = RequestStore::open(&path, key(dir.path())?)?;
    store.ask(asked("op-1"))?;
    drop(store);

    let other = tempfile::tempdir()?;
    let store = RequestStore::open(&path, key(other.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 1, .. }),
        "{}",
        store.start()
    );
    assert_eq!(store.requests().count(), 1);
    Ok(())
}

#[test]
fn leaves_written_before_pinning_are_pinned_once_and_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests");
    let mut leaves = FileLeafStore::create(&path, ORIGIN)?;
    for (index, id) in (0..).zip(["op-1", "op-2", "op-3"]) {
        let mut line = serde_json::to_value(asked(id))?;
        line.as_object_mut()
            .ok_or("an asked request is an object")?
            .insert("line".to_owned(), "asked".into());
        leaves.put_leaf(index, &serde_json::to_vec(&line)?)?;
    }
    drop(leaves);

    let store = RequestStore::open(&path, key(dir.path())?)?;
    assert_eq!(store.adopted(), 3);
    assert_eq!(store.requests().count(), 3);
    drop(store);

    let store = RequestStore::open(&path, key(dir.path())?)?;
    assert_eq!(store.adopted(), 0);
    assert_eq!(store.requests().count(), 3);
    Ok(())
}
