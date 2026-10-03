#![cfg(test)]
//! Gates on starting from a snapshot: what a good one saves, and that every
//! bad one is refused by its own name and never used.

use lys_core::Ed25519Identity;

use super::*;
use crate::file::FileLeafStore;
use crate::snapshot::seal;
use crate::store::PinnedRoot;
use crate::test_store::{CountingStore, Disk, ORIGIN};

const DOMAIN: &str = "example.com/lys/test-state/v1";

fn leaf(index: u64) -> Vec<u8> {
    format!("leaf-{index}").into_bytes()
}

fn key(dir: &tempfile::TempDir, name: &str) -> Ed25519Identity {
    Ed25519Identity::load_or_generate(&dir.path().join(name)).unwrap()
}

/// A log of `count` leaves with a snapshot written at `snapshot_at`.
fn logged(count: u64, snapshot_at: u64, key: &Ed25519Identity) -> Disk {
    let (mut log, _tail) = FrontierLog::open(CountingStore::new()).unwrap();
    for index in 0..count {
        if index == snapshot_at {
            log.write_snapshot(DOMAIN, format!("state@{index}").as_bytes(), key)
                .unwrap();
        }
        log.append(&leaf(index)).unwrap();
    }
    if snapshot_at == count {
        log.write_snapshot(DOMAIN, format!("state@{count}").as_bytes(), key)
            .unwrap();
    }
    log.store().disk.clone()
}

fn started(disk: Disk, key: &Ed25519Identity) -> Started<CountingStore> {
    open_with_snapshot(CountingStore::over(disk), DOMAIN, &key.public_key_bytes()).unwrap()
}

fn refused(disk: Disk, key: &Ed25519Identity, count: u64) -> SnapshotRefusal {
    let started = started(disk, key);
    assert_eq!(
        started.state, None,
        "a refused snapshot's state is never used"
    );
    assert_eq!(started.tail.from, 0);
    match started.start {
        Start::Rebuilt { refusal, replayed } => {
            assert_eq!(replayed, count);
            let checking = match refusal {
                SnapshotRefusal::WrongRoot { size } => count - size,
                _ => 0,
            };
            assert_eq!(
                started.log.store().reads(),
                count + checking,
                "the tail read to check the snapshot, then the whole log"
            );
            refusal
        }
        Start::Resumed { .. } => panic!("a bad snapshot was resumed from"),
    }
}

#[test]
fn a_start_from_a_snapshot_reads_only_the_tail() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let started = started(logged(50, 40, &key), &key);
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 40,
            replayed: 10
        }
    );
    assert_eq!(started.log.store().reads(), 10, "leaves 40 to 49 only");
    assert_eq!(started.state.as_deref(), Some(b"state@40".as_slice()));
    assert_eq!(started.tail.from, 40);
    assert_eq!(started.tail.leaves, (40..50).map(leaf).collect::<Vec<_>>());
    assert_eq!(started.log.len(), 50);
}

#[test]
fn a_snapshot_at_the_head_reads_no_leaf() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let started = started(logged(30, 30, &key), &key);
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 30,
            replayed: 0
        }
    );
    assert_eq!(started.log.store().reads(), 0);
}

#[test]
fn no_snapshot_is_refused_as_missing_and_the_whole_log_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(7, 3, &key);
    disk.snapshot = None;
    assert_eq!(refused(disk, &key, 7), SnapshotRefusal::Missing);
}

#[test]
fn a_tampered_snapshot_is_refused_by_its_signature() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(12, 8, &key);
    let sealed = disk.snapshot.as_mut().unwrap();
    let state_byte = sealed.len() - 64 - 8 - 1;
    sealed[state_byte] ^= 0x01;
    assert_eq!(refused(disk, &key, 12), SnapshotRefusal::SignatureInvalid);
}

#[test]
fn a_snapshot_signed_by_another_key_is_refused_by_its_signature() {
    let dir = tempfile::tempdir().unwrap();
    let owner = key(&dir, "owner");
    let stranger = key(&dir, "stranger");
    let disk = logged(6, 4, &stranger);
    assert_eq!(refused(disk, &owner, 6), SnapshotRefusal::SignatureInvalid);
}

#[test]
fn a_snapshot_without_its_signature_is_refused_as_unsigned() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(6, 4, &key);
    let sealed = disk.snapshot.as_mut().unwrap();
    sealed.truncate(sealed.len() - 64 - 8);
    assert_eq!(refused(disk, &key, 6), SnapshotRefusal::Unsigned);
}

#[test]
fn a_signed_snapshot_of_other_leaves_is_refused_as_the_wrong_root() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(10, 10, &key);
    let other = Frontier::from_leaves((500..506).map(leaf));
    disk.snapshot = Some(seal(DOMAIN, ORIGIN, &other, b"forged state", &key));
    assert_eq!(
        refused(disk, &key, 10),
        SnapshotRefusal::WrongRoot { size: 6 }
    );
}

#[test]
fn a_snapshot_past_the_pinned_log_refuses_the_open_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(4, 4, &key);
    let ahead = Frontier::from_leaves((0..9).map(leaf));
    disk.snapshot = Some(seal(DOMAIN, ORIGIN, &ahead, b"ahead", &key));
    let store = CountingStore::over(disk);
    let err = open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).unwrap_err();
    assert!(
        matches!(err, StoreError::SnapshotBeyondLog { size: 9, pinned: 4 }),
        "{err}"
    );
}

/// Every file under `dir` with its bytes, so a test can say a refused open
/// added, removed and changed nothing.
fn tree_bytes(dir: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                files.insert(path, bytes);
            }
        }
    }
    files
}

/// A file store of `count` leaves with a snapshot written at its head.
fn file_logged(dir: &std::path::Path, count: u64, key: &Ed25519Identity) {
    FileLeafStore::create(dir, ORIGIN).unwrap();
    let (mut log, _tail) = FrontierLog::open(FileLeafStore::open(dir).unwrap()).unwrap();
    for index in 0..count {
        log.append(&leaf(index)).unwrap();
    }
    log.write_snapshot(DOMAIN, format!("state@{count}").as_bytes(), key)
        .unwrap();
}

#[test]
fn a_snapshot_one_past_the_last_whole_record_refuses_the_open_and_rewrites_nothing() {
    // LYSLOGSTORE-008 R2: the snapshot names N+1 while the last whole record
    // is N. The pin at N+1 was acknowledged before the snapshot was written,
    // so a leaf the log counted is gone; the open is refused by name, writable
    // and read-only, and no file is rebuilt or rewritten.
    let tmp = tempfile::tempdir().unwrap();
    let key = key(&tmp, "key");
    let dir = tmp.path().join("log");
    file_logged(&dir, 5, &key);
    // The acknowledged frame at index 4 is lost: its leaf and its pin.
    std::fs::remove_file(dir.join("leaves").join(format!("{:020}", 4))).unwrap();
    let four = Frontier::from_leaves((0..4).map(leaf));
    crate::file::write_state(
        &dir,
        PinnedRoot {
            tree_size: four.size(),
            root: four.root(),
        },
    )
    .unwrap();
    let before = tree_bytes(&dir);
    for store in [
        FileLeafStore::open(&dir).unwrap(),
        FileLeafStore::open_read_only(&dir).unwrap(),
    ] {
        let err = open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).unwrap_err();
        assert!(
            matches!(err, StoreError::SnapshotBeyondLog { size: 5, pinned: 4 }),
            "{err}"
        );
        assert_eq!(tree_bytes(&dir), before, "the refusal changed no file");
    }
}

#[test]
fn a_torn_last_leaf_alone_opens_at_the_pin_and_is_named_read_only() {
    // LYSLOGSTORE-008 R2: the last frame alone is torn and the snapshot names
    // N or less. In this layout a torn frame is a temporary leaf file never
    // linked; a writable open answers at N, and a read-only open names the
    // leftover rather than tidying it.
    let tmp = tempfile::tempdir().unwrap();
    let key = key(&tmp, "key");
    let dir = tmp.path().join("log");
    file_logged(&dir, 5, &key);
    let torn = dir
        .join("leaves")
        .join(format!(".{}-{:020}-0.tmp", std::process::id() + 1, 5));
    std::fs::write(&torn, b"leaf-5 cut sh").unwrap();
    let started = open_with_snapshot(
        FileLeafStore::open(&dir).unwrap(),
        DOMAIN,
        &key.public_key_bytes(),
    )
    .unwrap();
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 5,
            replayed: 0
        }
    );
    assert_eq!(started.log.len(), 5);
    assert_eq!(started.log.recovered_to(), None);
    let read_only = FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(read_only.extent(), 5);
    assert_eq!(
        read_only.leftover_temporaries().unwrap(),
        vec![torn.file_name().unwrap().to_str().unwrap().to_owned()]
    );
    assert_eq!(std::fs::read(&torn).unwrap(), b"leaf-5 cut sh");
}

#[test]
fn a_snapshot_of_another_kind_or_log_is_refused_by_that_name() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let frontier = Frontier::from_leaves((0..3).map(leaf));
    let mut disk = logged(3, 3, &key);
    disk.snapshot = Some(seal(
        "example.com/other-state",
        ORIGIN,
        &frontier,
        b"s",
        &key,
    ));
    assert!(matches!(
        refused(disk.clone(), &key, 3),
        SnapshotRefusal::WrongKind { .. }
    ));
    disk.snapshot = Some(seal(DOMAIN, "example.com/other-log", &frontier, b"s", &key));
    assert!(matches!(
        refused(disk, &key, 3),
        SnapshotRefusal::WrongLog { .. }
    ));
}

#[test]
fn garbage_in_the_slot_is_refused_as_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(2, 2, &key);
    disk.snapshot = Some(b"not a snapshot".to_vec());
    assert!(matches!(
        refused(disk, &key, 2),
        SnapshotRefusal::Malformed { .. }
    ));
}

#[test]
fn a_crash_after_appends_and_before_the_next_snapshot_resumes_from_the_last_one() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let disk = logged(9, 6, &key);
    let started = started(disk, &key);
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 6,
            replayed: 3
        }
    );
    assert_eq!(started.log.recovered_to(), None);
}

#[test]
fn a_failed_append_after_a_snapshot_leaves_the_store_at_the_snapshot_on_resume() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut store = CountingStore::over(logged(5, 5, &key));
    store.fail_next_append();
    let (mut log, _tail) =
        FrontierLog::resume(store, Frontier::from_leaves((0..5).map(leaf))).unwrap();
    assert!(log.append(&leaf(5)).is_err());
    let started = started(log.store().disk.clone(), &key);
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 5,
            replayed: 0
        }
    );
    assert_eq!(started.log.recovered_to(), None);
    assert!(started.tail.leaves.is_empty());
}

#[test]
fn the_file_store_keeps_its_snapshot_across_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let log_dir = dir.path().join("log");
    FileLeafStore::create(&log_dir, ORIGIN).unwrap();
    let (mut log, _tail) = FrontierLog::open(FileLeafStore::open(&log_dir).unwrap()).unwrap();
    for index in 0..5 {
        log.append(&leaf(index)).unwrap();
    }
    log.write_snapshot(DOMAIN, b"five", &key).unwrap();
    log.append(&leaf(5)).unwrap();
    drop(log);
    std::fs::write(log_dir.join("snapshot.bin.tmp"), b"a torn write").unwrap();
    let started = open_with_snapshot(
        FileLeafStore::open(&log_dir).unwrap(),
        DOMAIN,
        &key.public_key_bytes(),
    )
    .unwrap();
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 5,
            replayed: 1
        }
    );
    assert_eq!(started.state.as_deref(), Some(b"five".as_slice()));
}
