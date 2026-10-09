#![cfg(test)]
//! Gates on the file-backed store over segments (LYSLOGSTORE-008 R1): the
//! layout, every leaf back byte for byte across rolled segments, a damaged
//! record refused by segment and offset, an offsets file repaired from its
//! last good entry, the read-only refusals, and a real process kill inside an
//! append that reopens at the pin with nothing adopted.

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use super::*;
use crate::frontier::Frontier;

const ORIGIN: &str = "example.com/lys/store-test";

/// The environment variable that turns the kill test's own binary into the
/// writer it kills.
const WRITER_DIR: &str = "LYS_LOG_STORE_KILL_WRITER_DIR";

fn create(dir: &Path) -> FileLeafStore {
    FileLeafStore::create(dir, ORIGIN).unwrap()
}

fn leaf(index: u64) -> Vec<u8> {
    format!("leaf-{index:06}").into_bytes()
}

/// Append `count` leaves in acts of `per_act`, pinning each act's end.
fn fill(store: &mut FileLeafStore, frontier: &mut Frontier, count: u64, per_act: u64) {
    let mut index = frontier.size();
    while index < count {
        let end = (index + per_act).min(count);
        let leaves: Vec<Vec<u8>> = (index..end).map(leaf).collect();
        let borrowed: Vec<&[u8]> = leaves.iter().map(Vec::as_slice).collect();
        for bytes in &leaves {
            frontier.push(bytes);
        }
        let pin = PinnedRoot {
            tree_size: frontier.size(),
            root: frontier.root(),
        };
        store.append(index, &borrowed, pin).unwrap();
        index = end;
    }
}

#[test]
fn create_writes_the_layout_and_open_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let store = create(&dir);
    assert_eq!(store.extent(), 0);
    assert!(dir.join("log.json").is_file());
    assert!(segment_file(&dir, 0).is_file());
    assert!(
        dir.join("leaves/segments/00000000000000000000.offsets")
            .is_file()
    );
    drop(store);
    let reopened = FileLeafStore::open(&dir).unwrap();
    assert_eq!(reopened.origin(), ORIGIN);
    assert_eq!(reopened.extent(), 0);
    assert_eq!(reopened.pinned(), empty_pin());
    assert_eq!(reopened.unfinished_tail(), None);
    assert_eq!(reopened.migrated(), None);
    assert!(matches!(
        FileLeafStore::create(&dir, ORIGIN).unwrap_err(),
        StoreError::AlreadyInitialized { .. }
    ));
    assert!(matches!(
        FileLeafStore::open(&tmp.path().join("absent")).unwrap_err(),
        StoreError::NotInitialized { .. }
    ));
}

#[test]
fn every_leaf_reads_back_byte_for_byte_across_rolled_segments() {
    for count in [1u64, 2, 1_000, 100_000] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("store");
        let mut store = create(&dir).with_roll_bytes(4 * 1024);
        let mut frontier = Frontier::new();
        // Segments roll between acts, so the acts are kept small enough for
        // a thousand leaves to cross the roll size several times.
        fill(&mut store, &mut frontier, count, 100);
        assert_eq!(store.extent(), count);
        assert_eq!(store.pinned().tree_size, count);
        assert_eq!(store.pinned().root, frontier.root());
        let rolled = store.segments.len();
        if count >= 1_000 {
            assert!(rolled > 1, "{count} leaves rolled into {rolled} segments");
        }
        drop(store);
        let reopened = FileLeafStore::open(&dir).unwrap();
        assert_eq!(reopened.extent(), count);
        assert_eq!(reopened.pinned().root, frontier.root());
        assert_eq!(reopened.segments.len(), rolled);
        for index in [0, count / 2, count - 1] {
            assert_eq!(
                reopened.leaf(index).unwrap().as_deref(),
                Some(leaf(index).as_slice())
            );
        }
        if count <= 1_000 {
            for index in 0..count {
                assert_eq!(
                    reopened.leaf(index).unwrap().as_deref(),
                    Some(leaf(index).as_slice())
                );
            }
        }
        assert_eq!(reopened.leaf(count).unwrap(), None);
        reopened.audit_leaves().unwrap();
    }
}

#[test]
fn a_record_with_one_flipped_byte_is_refused_by_segment_and_offset() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let mut store = create(&dir);
    let mut frontier = Frontier::new();
    fill(&mut store, &mut frontier, 5, 1);
    let offsets = segment_offsets(&dir, 0).unwrap();
    let segment = segment_file(&dir, 0);
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&segment)
            .unwrap();
        // Byte 2 of leaf 2: at its record's offset plus the length prefix.
        file.seek(SeekFrom::Start(offsets[2] + 4 + 2)).unwrap();
        file.write_all(b"X").unwrap();
    }
    let err = store.leaf(2).unwrap_err();
    match &err {
        StoreError::CorruptRecord {
            segment: named,
            offset,
            ..
        } => {
            assert_eq!(named, &segment);
            assert_eq!(*offset, offsets[2]);
        }
        other => panic!("{other}"),
    }
    assert!(
        err.to_string().starts_with(&format!(
            "corrupt record: {} at offset {}: ",
            segment.display(),
            offsets[2]
        )),
        "{err}"
    );
    assert_eq!(store.leaf(1).unwrap().as_deref(), Some(leaf(1).as_slice()));
    assert!(matches!(
        store.audit_leaves().unwrap_err(),
        StoreError::CorruptRecord { .. }
    ));
}

#[test]
fn an_offsets_file_cut_short_or_run_past_the_end_is_repaired_from_its_last_good_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let mut store = create(&dir);
    let mut frontier = Frontier::new();
    fill(&mut store, &mut frontier, 6, 2);
    drop(store);
    let offsets = dir.join("leaves/segments/00000000000000000000.offsets");
    let whole = std::fs::read(&offsets).unwrap();
    // Cut short, through the middle of an entry.
    std::fs::write(&offsets, &whole[..8 * 3 + 5]).unwrap();
    let reopened = FileLeafStore::open(&dir).unwrap();
    assert_eq!(reopened.extent(), 6);
    assert_eq!(reopened.unfinished_tail(), None);
    for index in 0..6 {
        assert_eq!(
            reopened.leaf(index).unwrap().as_deref(),
            Some(leaf(index).as_slice())
        );
    }
    assert_eq!(
        std::fs::read(&offsets).unwrap(),
        whole,
        "the writable open rewrote the offsets"
    );
    drop(reopened);
    // Run past the end, with entries no record stands at.
    let mut long = whole.clone();
    long.extend_from_slice(&u64::MAX.to_le_bytes());
    long.extend_from_slice(&(u64::MAX - 1).to_le_bytes());
    std::fs::write(&offsets, &long).unwrap();
    let read_only = FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(read_only.extent(), 6);
    assert_eq!(
        read_only.leaf(5).unwrap().as_deref(),
        Some(leaf(5).as_slice())
    );
    assert_eq!(
        std::fs::read(&offsets).unwrap(),
        long,
        "a reader changes no file"
    );
    let reopened = FileLeafStore::open(&dir).unwrap();
    assert_eq!(reopened.extent(), 6);
    assert_eq!(std::fs::read(&offsets).unwrap(), whole);
}

#[test]
fn the_read_only_refusals_name_the_store_and_the_refused_act() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let mut store = create(&dir);
    let mut frontier = Frontier::new();
    fill(&mut store, &mut frontier, 2, 2);
    drop(store);
    let mut read_only = FileLeafStore::open_read_only(&dir).unwrap();
    frontier.push(&leaf(2));
    let pin = PinnedRoot {
        tree_size: 3,
        root: frontier.root(),
    };
    for (err, act) in [
        (
            read_only.append(2, &[&leaf(2)], pin).unwrap_err(),
            "append leaves with their pin",
        ),
        (read_only.pin(pin).unwrap_err(), "pin"),
        (
            read_only.put_snapshot(b"s").unwrap_err(),
            "write a snapshot",
        ),
    ] {
        match err {
            StoreError::ReadOnly { path, operation } => {
                assert_eq!(path, dir);
                assert_eq!(operation, act);
            }
            other => panic!("{other}"),
        }
    }
    assert_eq!(read_only.extent(), 2);
}

#[test]
fn a_lone_pin_only_repeats_the_held_one() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let mut store = create(&dir);
    let mut frontier = Frontier::new();
    fill(&mut store, &mut frontier, 3, 3);
    store.pin(store.pinned()).unwrap();
    let mut ahead = frontier.clone();
    ahead.push(&leaf(3));
    let err = store
        .pin(PinnedRoot {
            tree_size: 4,
            root: ahead.root(),
        })
        .unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::PinNotOfAppend {
                end: 3,
                tree_size: 4
            }
        ),
        "{err}"
    );
    assert_eq!(store.extent(), 3);
}

#[test]
fn an_unexpected_entry_among_the_segments_is_corrupt_and_a_dot_name_is_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    create(&dir);
    std::fs::write(dir.join("leaves/segments/.editor-swap"), b"").unwrap();
    FileLeafStore::open(&dir).unwrap();
    std::fs::write(dir.join("leaves/segments/notes.txt"), b"").unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    match err {
        StoreError::Corrupt { reason, .. } => {
            assert!(reason.contains("unexpected entry"), "{reason}");
        }
        other => panic!("{other}"),
    }
}

/// The writer the kill test runs in a second process: it appends acts of
/// many leaves to the store at `dir` until it is killed.
fn kill_writer(dir: &Path) -> ! {
    let mut store = FileLeafStore::open(dir).unwrap();
    let mut frontier = Frontier::new();
    let mut index = 0u64;
    loop {
        let leaves: Vec<Vec<u8>> = (index..index + 64).map(big_leaf).collect();
        let borrowed: Vec<&[u8]> = leaves.iter().map(Vec::as_slice).collect();
        for bytes in &leaves {
            frontier.push(bytes);
        }
        let pin = PinnedRoot {
            tree_size: frontier.size(),
            root: frontier.root(),
        };
        store.append(index, &borrowed, pin).unwrap();
        index += 64;
    }
}

/// A leaf big enough that an act takes real time to write and flush.
fn big_leaf(index: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64 * 1024);
    while bytes.len() < 64 * 1024 {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes
}

#[test]
fn a_process_killed_inside_an_append_reopens_at_the_pin_with_nothing_adopted() {
    if let Some(dir) = std::env::var_os(WRITER_DIR) {
        kill_writer(Path::new(&dir));
    }
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    create(&dir);
    let segment = segment_file(&dir, 0);
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("file::tests::a_process_killed_inside_an_append_reopens_at_the_pin_with_nothing_adopted")
        .arg("--nocapture")
        .env(WRITER_DIR, &dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    // Kill once real acts have landed and another is in flight. A writer
    // that exits on its own is named; a slow one is waited for.
    loop {
        let len = std::fs::metadata(&segment).map_or(0, |meta| meta.len());
        if len > 12 * 1024 * 1024 {
            break;
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!("the writer exited on its own with {status} after {len} bytes");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let len_at_kill = std::fs::metadata(&segment).unwrap().len();

    let read_only = FileLeafStore::open_read_only(&dir).unwrap();
    let extent = read_only.extent();
    assert_eq!(read_only.pinned().tree_size, extent);
    assert_eq!(extent % 64, 0, "only whole acts are adopted");
    let tail = read_only.unfinished_tail().cloned();
    assert_eq!(
        std::fs::metadata(&segment).unwrap().len(),
        len_at_kill,
        "a reader cut nothing"
    );

    let store = FileLeafStore::open(&dir).unwrap();
    assert_eq!(store.extent(), extent);
    assert_eq!(store.pinned(), read_only.pinned());
    assert_eq!(store.unfinished_tail().cloned(), tail);
    let mut frontier = Frontier::new();
    for index in 0..extent {
        let bytes = store.leaf(index).unwrap().unwrap();
        assert_eq!(
            bytes,
            big_leaf(index),
            "leaf {index} is what the writer pinned"
        );
        frontier.push(&bytes);
    }
    assert_eq!(store.pinned().root, frontier.root());
    let after = std::fs::metadata(&segment).unwrap().len();
    match tail {
        Some(tail) => {
            assert_eq!(tail.offset, after, "the cut is exactly the unfinished act");
            assert_eq!(tail.offset + tail.bytes, len_at_kill);
        }
        None => assert_eq!(after, len_at_kill),
    }
    store.audit_leaves().unwrap();
}

/// One leaf offered at `index` behind `frontier`, as a writer whose view of
/// the store ends there would offer it.
fn offer(store: &mut FileLeafStore, frontier: &Frontier, index: u64) -> StoreResult<()> {
    let mut ahead = frontier.clone();
    ahead.push(b"the second writer's leaf");
    let pin = PinnedRoot {
        tree_size: ahead.size(),
        root: ahead.root(),
    };
    store.append(index, &[b"the second writer's leaf".as_slice()], pin)
}

#[test]
fn a_second_writers_act_at_a_taken_index_is_refused_with_nothing_written() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let mut first = create(&dir);
    let mut frontier = Frontier::new();
    fill(&mut first, &mut frontier, 2, 2);
    // A second writable handle, opened before the first writes again: its
    // view of the store ends at leaf 2.
    let mut second = FileLeafStore::open(&dir).unwrap();
    let stale = frontier.clone();
    fill(&mut first, &mut frontier, 3, 1);
    let before = std::fs::read(segment_file(&dir, 0)).unwrap();
    let err = offer(&mut second, &stale, 2).unwrap_err();
    assert!(
        matches!(err, StoreError::LeafAlreadyWritten { index: 2 }),
        "{err}"
    );
    assert_eq!(
        std::fs::read(segment_file(&dir, 0)).unwrap(),
        before,
        "the refused act wrote no byte"
    );
    drop((first, second));
    let reopened = FileLeafStore::open(&dir).unwrap();
    assert_eq!(reopened.extent(), 3);
    assert_eq!(reopened.pinned().root, frontier.root());
    assert!(reopened.unfinished_tail().is_none());
}

#[test]
fn a_second_writers_act_behind_another_writers_roll_is_refused_with_nothing_written() {
    // The first writer rolls at leaf 4, so the old segment is no longer than
    // the second writer left it: only the new segment's name says the index
    // is taken. The second writer is refused whether or not it would roll.
    for second_rolls in [true, false] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("store");
        let mut first = create(&dir).with_roll_bytes(64);
        let mut frontier = Frontier::new();
        fill(&mut first, &mut frontier, 4, 4);
        let mut second = FileLeafStore::open(&dir).unwrap();
        if second_rolls {
            second = second.with_roll_bytes(64);
        }
        let stale = frontier.clone();
        fill(&mut first, &mut frontier, 5, 1);
        assert_eq!(first.segments, [0, 4], "the first writer rolled at leaf 4");
        let before = [
            std::fs::read(segment_file(&dir, 0)).unwrap(),
            std::fs::read(segment_file(&dir, 4)).unwrap(),
        ];
        let err = offer(&mut second, &stale, 4).unwrap_err();
        assert!(
            matches!(err, StoreError::LeafAlreadyWritten { index: 4 }),
            "second_rolls {second_rolls}: {err}"
        );
        assert_eq!(
            [
                std::fs::read(segment_file(&dir, 0)).unwrap(),
                std::fs::read(segment_file(&dir, 4)).unwrap(),
            ],
            before,
            "second_rolls {second_rolls}: the refused act wrote no byte"
        );
        drop((first, second));
        let reopened = FileLeafStore::open(&dir).unwrap();
        assert_eq!(reopened.extent(), 5);
        assert_eq!(reopened.pinned().root, frontier.root());
    }
}

/// ACCESS-006 R5, R6: each leaf read and each snapshot read that goes to the
/// files is counted once; a leaf beyond the extent and every answer held in
/// memory are not.
#[test]
fn every_physical_read_is_counted_and_nothing_held_in_memory_is() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("store");
    let mut store = create(&dir);
    let mut frontier = Frontier::new();
    fill(&mut store, &mut frontier, 3, 3);
    let before = crate::process_read_count();
    assert_eq!(store.leaf(1).unwrap(), Some(leaf(1)));
    assert_eq!(store.leaf(2).unwrap(), Some(leaf(2)));
    assert_eq!(store.leaf(3).unwrap(), None, "beyond the extent");
    assert_eq!(crate::process_read_count() - before, 2);
    let _ = (store.extent(), store.pinned(), store.origin());
    assert_eq!(crate::process_read_count() - before, 2, "memory is no read");
    let _ = store.snapshot().unwrap();
    assert_eq!(crate::process_read_count() - before, 3);
}
