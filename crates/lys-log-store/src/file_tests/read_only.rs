#![cfg(test)]
//! The two opens: a writable open flushes `leaves/` before counting and is
//! refused when that flush fails, and a read-only open counts without flushing
//! and returns a handle that refuses every write by name.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use lys_core::merkle::raw_leaf_hash;

use crate::error::StoreError;
use crate::file::{Access, FileLeafStore};
use crate::store::{LeafStore, PinnedRoot};

use super::{ORIGIN, create};

/// A leaves-directory flush at open that always fails, with a kind no real
/// flush in these tests returns.
fn refuse_flush(_: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "injected leaves directory flush failure",
    ))
}

/// Every file under `dir`, as (path relative to `dir`, bytes).
fn files_under(dir: &Path) -> BTreeSet<(PathBuf, Vec<u8>)> {
    let mut found = BTreeSet::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                found.insert((path.strip_prefix(dir).unwrap().to_path_buf(), bytes));
            }
        }
    }
    found
}

/// A store at `dir` holding leaf 0 and leaf 1.
fn store_with_two_leaves(dir: &Path) {
    let mut store = create(dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    store.put_leaf(1, b"leaf-1").unwrap();
}

#[test]
fn a_read_only_open_never_flushes_the_leaves_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with_two_leaves(&dir);
    let store = FileLeafStore::open_as(&dir, Access::ReadOnly, refuse_flush).unwrap();
    assert_eq!(store.extent(), 2);
    assert_eq!(store.leaf(1).unwrap(), Some(b"leaf-1".to_vec()));
    assert_eq!(FileLeafStore::open_read_only(&dir).unwrap().extent(), 2);
}

#[test]
fn a_writable_open_whose_leaves_flush_fails_is_refused_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_with_two_leaves(&dir);
    let before = files_under(&dir);
    let err = FileLeafStore::open_as(&dir, Access::Writable, refuse_flush).unwrap_err();
    let StoreError::Io { context, source } = err else {
        panic!("expected Io, got {err}");
    };
    let leaves_dir = dir.join("leaves").display().to_string();
    assert!(context.contains(&leaves_dir), "{context}");
    assert!(context.contains("flush"), "{context}");
    assert_eq!(source.kind(), std::io::ErrorKind::TimedOut);
    assert_eq!(files_under(&dir), before, "the refused open changed no file");
}

#[test]
fn a_read_only_handle_refuses_put_leaf_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir).put_leaf(0, b"leaf-0").unwrap();
    let before = files_under(&dir);
    let mut store = FileLeafStore::open_read_only(&dir).unwrap();
    let err = store.put_leaf(1, b"leaf-1").unwrap_err();
    let rendered = err.to_string();
    let StoreError::ReadOnly { path, operation } = err else {
        panic!("expected ReadOnly, got {err}");
    };
    assert_eq!(path, dir);
    assert_eq!(operation, "write a leaf");
    let shown = dir.display().to_string();
    assert!(rendered.contains(&shown), "{rendered}");
    assert!(rendered.contains("read-only"), "{rendered}");
    assert_eq!(store.extent(), 1);
    assert_eq!(files_under(&dir), before, "the refused write changed no file");
}

#[test]
fn a_read_only_handle_refuses_pin_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut writer = FileLeafStore::create(&dir, ORIGIN).unwrap();
    writer.put_leaf(0, b"leaf-0").unwrap();
    writer
        .pin(PinnedRoot {
            tree_size: 1,
            root: [1; 32],
        })
        .unwrap();
    writer.put_leaf(1, b"leaf-1").unwrap();
    drop(writer);
    let mut store = FileLeafStore::open_read_only(&dir).unwrap();
    let state_hash = raw_leaf_hash(&std::fs::read(dir.join("state.json")).unwrap());
    let err = store
        .pin(PinnedRoot {
            tree_size: 2,
            root: [2; 32],
        })
        .unwrap_err();
    let StoreError::ReadOnly { path, operation } = err else {
        panic!("expected ReadOnly, got {err}");
    };
    assert_eq!(path, dir);
    assert_eq!(operation, "pin a root");
    assert_eq!(store.pinned().tree_size, 1);
    assert_eq!(
        raw_leaf_hash(&std::fs::read(dir.join("state.json")).unwrap()),
        state_hash
    );
}

#[test]
fn a_read_only_handle_refuses_a_snapshot_write_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir).put_leaf(0, b"leaf-0").unwrap();
    let before = files_under(&dir);
    let mut store = FileLeafStore::open_read_only(&dir).unwrap();
    let err = store.put_snapshot(b"a snapshot").unwrap_err();
    let StoreError::ReadOnly { path, operation } = err else {
        panic!("expected ReadOnly, got {err}");
    };
    assert_eq!(path, dir);
    assert_eq!(operation, "write a snapshot");
    assert_eq!(files_under(&dir), before, "the refused write changed no file");
}
