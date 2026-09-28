#![cfg(test)]
//! The pinned root, and what open detects in a directory it did not leave.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::StoreError;
use crate::file::{FileLeafStore, LOG_DIR_FORMAT};
use crate::store::{LeafStore, PinnedRoot};

use super::{ORIGIN, create, leaf_path};

/// A store with one leaf and the pin already at `(1, [7; 32])`.
fn store_pinned_at_one(dir: &Path) -> FileLeafStore {
    let mut store = create(dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    store
        .pin(PinnedRoot {
            tree_size: 1,
            root: [7u8; 32],
        })
        .unwrap();
    store
}

#[test]
fn the_pin_refuses_going_backwards() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = store_pinned_at_one(&dir);
    let err = store
        .pin(PinnedRoot {
            tree_size: 0,
            root: [0u8; 32],
        })
        .unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::PinWentBackwards {
                pinned: 1,
                requested: 0
            }
        ),
        "{err}"
    );
}

#[test]
fn re_pinning_the_identical_pin_is_permitted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = store_pinned_at_one(&dir);
    store
        .pin(PinnedRoot {
            tree_size: 1,
            root: [7u8; 32],
        })
        .expect("an identical re-pin is a no-op, not a failure");
    assert_eq!(store.pinned().root, [7u8; 32]);
}

#[test]
fn the_pin_refuses_a_second_root_at_a_size_it_already_holds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = store_pinned_at_one(&dir);
    // Two different roots at one tree size is equivocation — the thing this
    // crate exists to make unrepresentable. A size-only monotonicity check does
    // not catch it, because the size does not go backwards.
    let err = store
        .pin(PinnedRoot {
            tree_size: 1,
            root: [9u8; 32],
        })
        .unwrap_err();
    assert!(
        matches!(err, StoreError::PinRootChanged { tree_size: 1, .. }),
        "{err}"
    );
    assert_eq!(
        store.pinned().root,
        [7u8; 32],
        "the refused pin must not have taken effect"
    );
    // Deliberately NOT asserting the on-disk root here. That would make this
    // case fail whenever pin *durability* broke, for reasons having nothing to
    // do with equivocation — and a drift injection cannot tell a bundled test
    // apart from a specific one. `a_pin_survives_a_reopen` owns durability.
}

#[test]
fn a_pin_survives_a_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let store = store_pinned_at_one(&dir);
    assert_eq!(store.pinned().root, [7u8; 32]);
    assert_eq!(FileLeafStore::open(&dir).unwrap().pinned().root, [7u8; 32]);
}

#[test]
fn a_gap_in_the_stored_indices_is_detected_at_open() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    store.put_leaf(1, b"leaf-1").unwrap();
    store.put_leaf(2, b"leaf-2").unwrap();
    std::fs::remove_file(leaf_path(&dir, 1)).unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("not contiguous"), "{err}");
}

#[test]
fn an_unexpected_leaves_entry_is_found_by_the_audit_but_dotfiles_are_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    std::fs::write(dir.join("leaves").join(".DS_Store"), b"junk").unwrap();
    assert!(
        FileLeafStore::open(&dir).unwrap().audit_leaves().is_ok(),
        "dotfiles must be ignored"
    );
    std::fs::write(dir.join("leaves").join("stray.txt"), b"junk").unwrap();
    let err = FileLeafStore::open(&dir)
        .unwrap()
        .audit_leaves()
        .unwrap_err();
    assert!(err.to_string().contains("unexpected entry"), "{err}");
}

#[test]
fn a_malformed_state_file_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir);
    std::fs::write(dir.join("state.json"), "{\"tree_size\": 0}").unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("state.json is malformed"), "{err}");
}

#[test]
fn a_non_canonical_pinned_root_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir);
    // Read the pinned root back rather than assuming what the empty tree
    // hashes to: a fixture that encodes a guess about lys-core's Merkle
    // convention would pass or fail for reasons unrelated to base64 decoding.
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("state.json")).unwrap()).unwrap();
    let pinned = state["root_hash"].as_str().unwrap();
    assert_eq!(STANDARD.decode(pinned).unwrap().len(), 32);
    std::fs::write(
        dir.join("state.json"),
        "{\"tree_size\":0,\"root_hash\":\"c2hvcnQ=\"}",
    )
    .unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("exactly 32 bytes"), "{err}");
}

#[test]
fn an_unknown_format_marker_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir);
    let config = std::fs::read_to_string(dir.join("log.json")).unwrap();
    std::fs::write(
        dir.join("log.json"),
        config.replacen(LOG_DIR_FORMAT, "lys/log-dir/v99", 1),
    )
    .unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("lys/log-dir/v99"), "{err}");
}

#[test]
fn an_unknown_config_field_is_refused_rather_than_ignored() {
    // `deny_unknown_fields`: a field this version does not understand may be a
    // newer version's invariant, and ignoring it would silently drop a rule.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir);
    std::fs::write(
        dir.join("log.json"),
        format!("{{\"format\":\"{LOG_DIR_FORMAT}\",\"origin\":\"{ORIGIN}\",\"extra\":1}}"),
    )
    .unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("log.json is malformed"), "{err}");
}

#[test]
fn the_stored_origin_survives_a_reopen_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir);
    assert_eq!(FileLeafStore::open(&dir).unwrap().origin(), ORIGIN);
    assert_eq!(FileLeafStore::open(&dir).unwrap().dir(), dir);
}

#[test]
fn debug_summarizes_without_leaf_content() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"secret-looking-leaf-content").unwrap();
    let rendered = format!("{store:?}");
    assert!(
        !rendered.contains("secret-looking-leaf-content"),
        "{rendered}"
    );
    assert!(rendered.contains("extent: 1"), "{rendered}");
}
