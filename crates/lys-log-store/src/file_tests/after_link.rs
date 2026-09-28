#![cfg(test)]
//! Temporary names and the steps after a link: a taken name is never replaced,
//! a name that cannot be removed still commits the leaf, and a failed directory
//! flush is named and halts the handle until it is reopened.

use std::io::Write;
use std::path::Path;

use crate::error::StoreError;
use crate::file::{
    AFTER_LINK, AfterLink, FileLeafStore, LEAF_TEMP_ATTEMPTS, leaf_temp_name,
    next_process_sequence, remove_file, sync_dir,
};
use crate::store::LeafStore;

use super::{create, leaf_path, leaves_entries};

/// A post-link step that always fails, naming the path it was given.
fn refuse(path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(format!(
        "injected failure at {}",
        path.display()
    )))
}

/// The message the injected leaves-directory flush failure carries.
const INJECTED_FLUSH_FAILURE: &str = "injected leaves directory flush failure";

/// A leaves-directory flush that always fails, with a kind no real flush in
/// these tests returns, so the error that reaches the caller can be traced to
/// this injection and no other.
fn refuse_flush(_: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        INJECTED_FLUSH_FAILURE,
    ))
}

#[test]
fn leaf_temp_name_differs_by_sequence_and_by_pid() {
    assert_ne!(leaf_temp_name(41, 3, 0), leaf_temp_name(41, 3, 1));
    assert_ne!(leaf_temp_name(41, 3, 0), leaf_temp_name(42, 3, 0));
    assert_eq!(
        leaf_temp_name(41, 3, 5),
        format!(".41-{:020}-5.tmp", 3),
        "the name is the documented layout"
    );
}

/// A sequence source counting up from zero, private to one test, so the names
/// a write tries are known before it runs.
fn sequences_from_zero() -> impl FnMut() -> u64 {
    let mut next = 0;
    move || {
        let sequence = next;
        next += 1;
        sequence
    }
}

#[test]
fn a_taken_temporary_name_is_skipped_and_never_replaced() {
    // A leftover sits at the exact name this write tries first. Opening it
    // with truncation would destroy bytes this write does not own; the write
    // must move to the next name and leave the leftover whole.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    let pid = std::process::id();
    let taken = dir.join("leaves").join(leaf_temp_name(pid, 0, 0));
    std::fs::write(&taken, b"a leftover this write does not own").unwrap();
    // Records each sequence the write asks for, so the name it settled on is
    // known without depending on how the temporary file is linked.
    let mut tried = Vec::new();
    let mut from_zero = sequences_from_zero();
    let mut next_sequence = || {
        let sequence = from_zero();
        tried.push(sequence);
        sequence
    };
    store
        .put_leaf_with(
            0,
            |file| file.write_all(b"leaf-0"),
            &AFTER_LINK,
            &mut next_sequence,
        )
        .unwrap();
    assert_eq!(
        tried,
        vec![0, 1],
        "the write moved from the taken name to the next"
    );
    assert_eq!(
        store.leaf(0).unwrap().as_deref(),
        Some(b"leaf-0".as_slice())
    );
    assert_eq!(
        std::fs::read(&taken).unwrap(),
        b"a leftover this write does not own"
    );
    let mut expected = vec![format!("{:020}", 0), leaf_temp_name(pid, 0, 0)];
    expected.sort();
    assert_eq!(leaves_entries(&dir), expected);
}

#[test]
fn a_write_finding_every_temporary_name_taken_refuses_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    let pid = std::process::id();
    for sequence in 0..u64::from(LEAF_TEMP_ATTEMPTS) {
        std::fs::write(
            dir.join("leaves").join(leaf_temp_name(pid, 0, sequence)),
            b"taken",
        )
        .unwrap();
    }
    let before = leaves_entries(&dir);
    assert_eq!(before.len(), 16);
    let err = store
        .put_leaf_with(
            0,
            |file| file.write_all(b"leaf-0"),
            &AFTER_LINK,
            &mut sequences_from_zero(),
        )
        .unwrap_err();
    let StoreError::LeafTempNamesTaken {
        index,
        attempts,
        path,
    } = err
    else {
        panic!("expected LeafTempNamesTaken, got {err}");
    };
    assert_eq!(index, 0);
    assert_eq!(attempts, LEAF_TEMP_ATTEMPTS);
    assert_eq!(path, dir.join("leaves"));
    assert_eq!(store.extent(), 0);
    assert!(!leaf_path(&dir, 0).exists(), "no leaf for a refused write");
    assert_eq!(leaves_entries(&dir), before, "no new file appeared");
}

#[test]
fn two_handles_writing_in_turn_leave_no_temporary_names() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut first = create(&dir);
    first.put_leaf(0, b"leaf-0").unwrap();
    first.put_leaf(1, b"leaf-1").unwrap();
    let mut second = FileLeafStore::open(&dir).unwrap();
    second.put_leaf(2, b"leaf-2").unwrap();
    assert_eq!(first.extent(), 2);
    assert_eq!(
        leaves_entries(&dir),
        vec![
            format!("{:020}", 0),
            format!("{:020}", 1),
            format!("{:020}", 2)
        ]
    );
}

#[test]
fn a_temporary_name_that_cannot_be_removed_still_commits_the_leaf() {
    // The link is the commit point. Failing to tidy the temporary name after
    // it is not a failed append: the leaf is stored and the extent covers it.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    let keep_temp = AfterLink {
        remove_temp: refuse,
        flush_dir: sync_dir,
    };
    store
        .put_leaf_with(
            0,
            |file| file.write_all(b"leaf-0"),
            &keep_temp,
            &mut next_process_sequence,
        )
        .unwrap();
    assert_eq!(store.extent(), 1);
    assert_eq!(std::fs::read(leaf_path(&dir, 0)).unwrap(), b"leaf-0");
    store.put_leaf(1, b"leaf-1").unwrap();
    assert_eq!(FileLeafStore::open(&dir).unwrap().extent(), 2);
}

#[test]
fn a_failed_directory_flush_is_named_and_halts_the_handle_until_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    let no_flush = AfterLink {
        remove_temp: remove_file,
        flush_dir: refuse_flush,
    };
    let err = store
        .put_leaf_with(
            1,
            |file| file.write_all(b"leaf-1"),
            &no_flush,
            &mut next_process_sequence,
        )
        .unwrap_err();
    assert!(
        matches!(err, StoreError::LeafDurabilityUncertain { index: 1, .. }),
        "{err}"
    );
    assert_eq!(store.extent(), 2, "the link committed the leaf");
    let err = store.put_leaf(2, b"leaf-2").unwrap_err();
    assert!(
        matches!(err, StoreError::ReopenRequired { index: 1 }),
        "{err}"
    );
    assert!(
        !leaf_path(&dir, 2).exists(),
        "the halted handle wrote nothing"
    );

    let mut reopened = FileLeafStore::open(&dir).unwrap();
    assert_eq!(reopened.extent(), 2, "reopening counts the uncertain leaf");
    assert_eq!(reopened.leaf(1).unwrap().unwrap(), b"leaf-1");
    reopened.put_leaf(2, b"leaf-2").unwrap();
    assert_eq!(reopened.extent(), 3);
}

#[test]
fn a_failed_directory_flush_carries_the_flush_error_as_its_source() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    let no_flush = AfterLink {
        remove_temp: remove_file,
        flush_dir: refuse_flush,
    };
    let err = store
        .put_leaf_with(
            0,
            |file| file.write_all(b"leaf-0"),
            &no_flush,
            &mut next_process_sequence,
        )
        .unwrap_err();
    let StoreError::LeafDurabilityUncertain { source, .. } = err else {
        panic!("expected LeafDurabilityUncertain, got {err}");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::TimedOut);
    assert_eq!(source.to_string(), INJECTED_FLUSH_FAILURE);
}

#[test]
fn leaf_durability_uncertain_names_its_index_and_its_source() {
    let err = StoreError::LeafDurabilityUncertain {
        index: 7_340_033,
        source: std::io::Error::new(std::io::ErrorKind::TimedOut, INJECTED_FLUSH_FAILURE),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("7340033"), "{rendered}");
    assert!(rendered.contains(INJECTED_FLUSH_FAILURE), "{rendered}");
    assert!(rendered.contains("reopen"), "{rendered}");
}

#[test]
fn a_refused_link_leaves_no_temporary_file() {
    // The refusal is driven here rather than left to the link's own rule: the
    // leaf's final name is taken by a directory, which no way of naming a file
    // can take over. So this case holds however the link is made, and asserts
    // only that the abandoned temporary file is removed.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    std::fs::create_dir(leaf_path(&dir, 1)).unwrap();
    std::fs::write(leaf_path(&dir, 1).join("occupant"), b"occupant").unwrap();
    store.put_leaf(1, b"leaf-1").unwrap_err();
    assert_eq!(
        leaves_entries(&dir),
        vec![format!("{:020}", 0), format!("{:020}", 1)],
        "no temporary name is left behind"
    );
}
