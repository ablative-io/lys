#![cfg(test)]
//! Gates on starting from a snapshot: what a good one saves, and that every
//! bad one is refused by its own name and never used.

use lys_core::Ed25519Identity;

use super::*;
use crate::file::FileLeafStore;
use crate::snapshot::seal;
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
    start(CountingStore::over(disk), DOMAIN, &key.public_key_bytes()).unwrap()
}

fn refused(disk: Disk, key: &Ed25519Identity, count: u64) -> SnapshotRefusal {
    let started = started(disk, key);
    assert_eq!(
        started.state, None,
        "a refused snapshot's state is never used"
    );
    assert_eq!(started.tail.from, 0);
    assert_eq!(started.log.store().reads(), count, "the whole log was read");
    match started.start {
        Start::Rebuilt { refusal, replayed } => {
            assert_eq!(replayed, count);
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
fn a_snapshot_past_the_pinned_log_is_refused_as_beyond_it() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut disk = logged(4, 4, &key);
    let ahead = Frontier::from_leaves((0..9).map(leaf));
    disk.snapshot = Some(seal(DOMAIN, ORIGIN, &ahead, b"ahead", &key));
    assert_eq!(
        refused(disk, &key, 4),
        SnapshotRefusal::BeyondLog { size: 9, pinned: 4 }
    );
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
fn a_crash_between_a_leaf_and_its_pin_after_a_snapshot_is_repaired_on_resume() {
    let dir = tempfile::tempdir().unwrap();
    let key = key(&dir, "key");
    let mut store = CountingStore::over(logged(5, 5, &key));
    store.fail_next_pin();
    let (mut log, _tail) =
        FrontierLog::resume(store, Frontier::from_leaves((0..5).map(leaf))).unwrap();
    assert!(log.append(&leaf(5)).is_err());
    let started = started(log.store().disk.clone(), &key);
    assert_eq!(
        started.start,
        Start::Resumed {
            size: 5,
            replayed: 1
        }
    );
    assert_eq!(started.log.recovered_to(), Some(6));
    assert_eq!(started.tail.leaves, vec![leaf(5)]);
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
    let started = start(
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
