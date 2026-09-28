#![cfg(test)]
//! A removal that fails after the link is kept by name, never dropped.

use std::io::Write;
use std::path::Path;

use super::{AFTER_LINK, AfterLink, FileLeafStore, next_process_sequence, sync_dir};
use crate::store::LeafStore;

const ORIGIN: &str = "example.com/log";

/// A removal that always fails, naming the path it was given.
fn refuse(path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(format!(
        "injected failure at {}",
        path.display()
    )))
}

fn create(dir: &Path) -> FileLeafStore {
    FileLeafStore::create(dir, ORIGIN).unwrap();
    FileLeafStore::open(dir).unwrap()
}

#[test]
fn a_temporary_name_that_cannot_be_removed_is_kept_by_name_with_its_reason() {
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
    let left = store.left_behind();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].index, 0);
    assert_eq!(left[0].path.parent(), Some(dir.join("leaves").as_path()));
    assert!(left[0].path.is_file(), "{}", left[0].path.display());
    assert_eq!(
        left[0].source.to_string(),
        format!("injected failure at {}", left[0].path.display())
    );
}

#[test]
fn a_temporary_name_that_is_removed_leaves_nothing_to_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store
        .put_leaf_with(
            0,
            |file| file.write_all(b"leaf-0"),
            &AFTER_LINK,
            &mut next_process_sequence,
        )
        .unwrap();
    assert_eq!(store.extent(), 1);
    assert!(store.left_behind().is_empty());
    let names: Vec<_> = std::fs::read_dir(dir.join("leaves"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, [std::ffi::OsString::from(format!("{:020}", 0))]);
}
