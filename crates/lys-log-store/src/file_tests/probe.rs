#![cfg(test)]
//! Opening finds the extent from the pin forward, and the audit reads every
//! name.

use std::path::Path;

use crate::error::StoreError;
use crate::file::FileLeafStore;
use crate::store::{LeafStore, PinnedRoot};

use super::{create, leaf_path};

/// A store holding `leaves` leaves with the pin covering the first `pinned`.
fn store_with(dir: &Path, leaves: u64, pinned: u64) {
    let mut store = create(dir);
    for index in 0..leaves {
        store
            .put_leaf(index, format!("leaf-{index}").as_bytes())
            .unwrap();
    }
    store
        .pin(PinnedRoot {
            tree_size: pinned,
            root: [9u8; 32],
        })
        .unwrap();
}

#[test]
fn open_counts_the_leaves_past_the_pin() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with(&dir, 7, 4);
    let store = FileLeafStore::open(&dir).unwrap();
    assert_eq!(store.extent(), 7);
    store.audit_leaves().unwrap();
}

#[test]
fn a_pin_whose_last_leaf_is_gone_opens_to_the_leaves_that_are_there() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with(&dir, 5, 5);
    std::fs::remove_file(leaf_path(&dir, 4)).unwrap();
    let store = FileLeafStore::open(&dir).unwrap();
    assert_eq!(store.extent(), 4, "the log refuses the pin, not the store");
}

#[test]
fn a_pin_ahead_of_a_gap_is_refused_by_the_listing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with(&dir, 5, 5);
    std::fs::remove_file(leaf_path(&dir, 4)).unwrap();
    std::fs::remove_file(leaf_path(&dir, 1)).unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(matches!(err, StoreError::Corrupt { .. }), "{err}");
}

#[test]
fn open_refuses_a_gap_just_past_the_extent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with(&dir, 5, 2);
    std::fs::remove_file(leaf_path(&dir, 3)).unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(
        matches!(&err, StoreError::Corrupt { reason, .. } if reason.contains("expected leaf index 3, found 4")),
        "{err}"
    );
}

#[test]
fn the_audit_finds_a_leaf_removed_below_the_pin() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with(&dir, 6, 6);
    std::fs::remove_file(leaf_path(&dir, 2)).unwrap();
    let store = FileLeafStore::open(&dir).unwrap();
    let err = store.audit_leaves().unwrap_err();
    assert!(matches!(err, StoreError::Corrupt { .. }), "{err}");
}
