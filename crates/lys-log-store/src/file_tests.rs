#![cfg(test)]
//! Gates on the file-backed store.
//!
//! # The two absent-checks are tested SEPARATELY, on purpose
//!
//! [`FileLeafStore::put_leaf`] refuses a re-used index twice over: once against
//! the cached `extent`, and once via the no-replace link at the filesystem. Two checks
//! guarding one rule is how a check rots unnoticed — remove either and the
//! obvious test (append twice at index 0) still fails, because *the other one*
//! catches it. A drift injection that leaves the suite red has proven nothing
//! about which check is load-bearing.
//!
//! So each is isolated by a case only it can catch:
//!
//! | injection | the only case that fails |
//! |---|---|
//! | remove the `index < extent` check | `extent_check_alone_refuses_a_deleted_leafs_index` — the file is *gone*, so the link succeeds and would silently rewrite history |
//! | make the link replace an existing name | `no_replace_link_alone_refuses_a_leaf_this_store_never_saw` — the index *is* the next free one, so the extent check is satisfied and another writer's leaf would be clobbered |
//!
//! Both were injected before this landed; each failed exactly one case.
//!
//! # The same rule applies to the pin, and it was NOT applied here at first
//!
//! The reasoning above was written for the absent-checks and then not carried
//! three tests down the file: `pin` had a single `the_pin_only_advances` case
//! bundling **three independent rules** — refuse a smaller size, permit an
//! identical re-pin, and keep the pin across a reopen. Any one of the three
//! breaking failed that one test, so *"exactly one test failed"* was satisfied
//! by construction and identified nothing.
//!
//! **A bundled test is invisible to any single injection.** One injection tells
//! you a rule is guarded; only a *second* injection on a *different* rule, whose
//! failure signature you compare against the first, tells you the guard is
//! specific. That is why this went unnoticed through five injections that each
//! looked decisive.
//!
//! | injection | the only case that fails |
//! |---|---|
//! | remove the `size <` monotonic check | `the_pin_refuses_going_backwards` |
//! | refuse an identical re-pin | `re_pinning_the_identical_pin_is_permitted` |
//! | remove the equal-size root comparison | `the_pin_refuses_a_second_root_at_a_size_it_already_holds` |
//!
//! Durability is deliberately **not** in that table. Making `pin` skip its
//! write fails six cases, and correctly so: persistence is a substrate the
//! integrity checks stand on, not a rule with one guard. The criterion applies
//! to rules. What it does require is that no *rule* case be dragged into the
//! substrate's failure signature — which is why the equivocation case asserts
//! nothing about the on-disk root.
//!
//! One probe had to be rebuilt to learn this. Flipping the monotonic `<` to
//! `<=` fails two cases, and that is honest: it changes the allowance *and*
//! re-routes an equivocation attempt to the wrong error, so it is not a
//! single-rule drift. **The criterion presumes the injection isolates one rule;
//! an injection that moves a boundary inside a shared condition does not, and
//! its two failures indict the probe rather than the tests.**

use std::io::Write;
use std::path::Path;

use super::*;

const ORIGIN: &str = "example.com/lys/store-test";

#[path = "file_tests/after_link.rs"]
mod after_link;
#[path = "file_tests/pins.rs"]
mod pins;
#[path = "file_tests/probe.rs"]
mod probe;

fn create(dir: &Path) -> FileLeafStore {
    FileLeafStore::create(dir, ORIGIN).unwrap();
    FileLeafStore::open(dir).unwrap()
}

fn leaf_path(dir: &Path, index: u64) -> std::path::PathBuf {
    dir.join("leaves").join(format!("{index:020}"))
}

#[test]
fn create_writes_the_layout_and_open_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let store = create(&dir);
    assert_eq!(store.origin(), ORIGIN);
    assert_eq!(store.extent(), 0);
    assert_eq!(store.pinned().tree_size, 0);
    assert!(dir.join("log.json").is_file());
    assert!(dir.join("state.json").is_file());
    assert!(dir.join("leaves").is_dir());
}

#[test]
fn create_refuses_to_reinitialize() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    create(&dir);
    let err = FileLeafStore::create(&dir, "example.com/other").unwrap_err();
    assert!(
        matches!(err, StoreError::AlreadyInitialized { .. }),
        "{err}"
    );
}

#[test]
fn create_rejects_an_origin_a_checkpoint_would_reject() {
    let tmp = tempfile::tempdir().unwrap();
    let bad_origins = ["", "has space", "has+plus"];
    for (index, bad) in bad_origins.iter().enumerate() {
        // Indexed, not named by `bad.len()`: two rejected origins of equal
        // length would silently share one directory, and a fixture that
        // collides by coincidence is one edit away from testing nothing.
        let dir = tmp.path().join(format!("log-{index}"));
        let err = FileLeafStore::create(&dir, bad).unwrap_err();
        assert!(matches!(err, StoreError::Trust(_)), "{bad:?}: {err}");
        // Nothing was created for a rejected origin.
        assert!(!dir.join("log.json").exists(), "{bad:?}");
    }
    // A loop that silently ran zero times would satisfy every assertion inside
    // it: a control that passes without firing looks exactly like one that
    // passed.
    assert_eq!(bad_origins.len(), 3);
}

#[test]
fn open_on_an_uninitialized_dir_is_not_initialized() {
    let tmp = tempfile::tempdir().unwrap();
    let err = FileLeafStore::open(&tmp.path().join("nope")).unwrap_err();
    assert!(matches!(err, StoreError::NotInitialized { .. }), "{err}");
}

#[test]
fn leaf_files_hold_raw_bytes_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"raw \x00 bytes").unwrap();
    // The leaf file IS the RFC 6962 preimage: no framing, no padding.
    assert_eq!(
        std::fs::read(leaf_path(&dir, 0)).unwrap(),
        b"raw \x00 bytes"
    );
    assert_eq!(
        store.leaf(0).unwrap().as_deref(),
        Some(b"raw \x00 bytes".as_slice())
    );
}

#[test]
fn an_empty_leaf_is_legal_and_distinct_from_an_absent_one() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"").unwrap();
    assert_eq!(store.leaf(0).unwrap(), Some(Vec::new()));
    assert_eq!(store.leaf(1).unwrap(), None);
    assert_eq!(store.extent(), 1);
}

#[test]
fn extent_check_alone_refuses_a_deleted_leafs_index() {
    // Isolates the `index < extent` check: the leaf FILE is removed behind the
    // store's back, so the link would succeed. Only the extent check can
    // refuse this, and it must — writing here would replace a leaf the tree
    // already covers, which is a second history for a settled position.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    store.put_leaf(1, b"leaf-1").unwrap();
    std::fs::remove_file(leaf_path(&dir, 0)).unwrap();
    let err = store.put_leaf(0, b"replacement").unwrap_err();
    assert!(
        matches!(err, StoreError::LeafAlreadyWritten { index: 0 }),
        "{err}"
    );
    assert!(!leaf_path(&dir, 0).exists(), "the refusal wrote nothing");
}

#[test]
fn no_replace_link_alone_refuses_a_leaf_this_store_never_saw() {
    // Isolates the no-replace link: index 1 IS the next free index as far as
    // this store knows, so the extent check passes. A file appearing there
    // after open is another writer, and clobbering it would destroy a leaf this
    // store never knew existed.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    std::fs::write(leaf_path(&dir, 1), b"another writers leaf").unwrap();
    let err = store.put_leaf(1, b"ours").unwrap_err();
    assert!(
        matches!(err, StoreError::LeafAlreadyWritten { index: 1 }),
        "{err}"
    );
    assert_eq!(
        std::fs::read(leaf_path(&dir, 1)).unwrap(),
        b"another writers leaf",
        "the other writer's leaf survived untouched"
    );
}

#[test]
fn a_reused_index_is_refused_by_the_ordinary_route_too() {
    // The obvious case, kept because it is the one that actually happens: two
    // appends racing for the same position. Either check catches it, which is
    // exactly why it cannot stand in for the two isolating tests above.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    let err = store.put_leaf(0, b"leaf-0-again").unwrap_err();
    assert!(
        matches!(err, StoreError::LeafAlreadyWritten { index: 0 }),
        "{err}"
    );
    assert_eq!(std::fs::read(leaf_path(&dir, 0)).unwrap(), b"leaf-0");
    assert_eq!(store.extent(), 1, "a refused write does not advance extent");
}

#[test]
fn a_write_past_the_next_index_is_refused_and_leaves_no_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    let err = store.put_leaf(2, b"leaf-2").unwrap_err();
    assert!(
        matches!(err, StoreError::LeafWouldLeaveGap { index: 2, next: 1 }),
        "{err}"
    );
    assert!(!leaf_path(&dir, 2).exists(), "no file for a refused write");
    assert_eq!(store.extent(), 1);
}

/// Names in `leaves/`, sorted, so a test can say exactly what is there.
fn leaves_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.join("leaves"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn a_write_failing_before_the_link_leaves_no_leaf() {
    // Half the bytes reach the file and then the write fails. Nothing may be
    // left under a leaf name, because the next open would count it and a torn
    // leaf would then be pinned as history.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    let err = store
        .put_leaf_with(
            1,
            |file| {
                file.write_all(b"torn")?;
                Err(std::io::Error::other("injected write failure"))
            },
            &AFTER_LINK,
            &mut next_process_sequence,
        )
        .unwrap_err();
    assert!(matches!(err, StoreError::Io { .. }), "{err}");
    assert!(!leaf_path(&dir, 1).exists(), "no leaf for a failed write");
    assert_eq!(
        leaves_entries(&dir),
        vec![format!("{:020}", 0)],
        "the temporary file was removed too"
    );
    assert_eq!(store.extent(), 1, "a failed write does not advance extent");
    assert_eq!(FileLeafStore::open(&dir).unwrap().extent(), 1);
    store.put_leaf(1, b"leaf-1").unwrap();
    assert_eq!(std::fs::read(leaf_path(&dir, 1)).unwrap(), b"leaf-1");
}

#[test]
fn a_leftover_temporary_file_is_ignored_at_open() {
    // A crash between writing the temporary file and linking it leaves the
    // file behind under its hidden name. It holds a full leaf's bytes, and it
    // must still not count: only a linked name is a leaf.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    let leftover = dir
        .join("leaves")
        .join(leaf_temp_name(std::process::id() + 1, 1, 0));
    std::fs::write(&leftover, b"leaf-1-never-linked").unwrap();
    let mut reopened = FileLeafStore::open(&dir).unwrap();
    assert_eq!(reopened.extent(), 1);
    assert_eq!(reopened.leaf(1).unwrap(), None);
    reopened.put_leaf(1, b"leaf-1").unwrap();
    assert_eq!(std::fs::read(leaf_path(&dir, 1)).unwrap(), b"leaf-1");
    assert!(
        leftover.exists(),
        "open ignores the leftover, it does not own it"
    );
}

#[test]
fn open_leaves_a_leftover_temporary_file_byte_identical() {
    // Opening reads; it never deletes or changes anything, so a store opened
    // read-only stays untouched. The leftover is skipped, not tidied away.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    let leftover = dir
        .join("leaves")
        .join(leaf_temp_name(std::process::id() + 1, 1, 0));
    std::fs::write(&leftover, b"half a leaf").unwrap();
    let before = leaves_entries(&dir);
    assert_eq!(FileLeafStore::open(&dir).unwrap().extent(), 1);
    assert_eq!(leaves_entries(&dir), before, "open changed no entry");
    assert_eq!(std::fs::read(&leftover).unwrap(), b"half a leaf");
}

/// Every file under `dir`, dot-prefixed names included, with its bytes, so a
/// test can say an open or a refusal added, removed and changed nothing.
fn tree_bytes(dir: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
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

/// The pin a store holding exactly `leaves` carries.
fn pin_over(leaves: &[&[u8]]) -> PinnedRoot {
    let leaves: Vec<Vec<u8>> = leaves.iter().copied().map(<[u8]>::to_vec).collect();
    let (root, tree_size) = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(&leaves)
        .root()
        .to_parts();
    PinnedRoot { tree_size, root }
}

/// A store holding `leaf` at index 0 with the pin covering it.
fn store_pinned_over(dir: &Path, leaf: &[u8]) {
    let mut store = FileLeafStore::create(dir, ORIGIN).unwrap();
    store.put_leaf(0, leaf).unwrap();
    store.pin(pin_over(&[leaf])).unwrap();
}

#[test]
fn the_read_only_refusals_name_the_store_and_the_refused_act() {
    let read_only = StoreError::ReadOnly {
        path: PathBuf::from("store"),
        operation: "write a leaf",
    };
    assert_eq!(
        read_only.to_string(),
        "refusing to write a leaf in the log store at store: it was opened read-only"
    );
    let repair_pending = StoreError::RepairPending {
        path: PathBuf::from("store"),
        pinned_size: 1,
        extent: 2,
    };
    assert_eq!(
        repair_pending.to_string(),
        "refusing to open the log store at store read-only: it holds 2 leaves but its pin is \
         at tree size 1, an interrupted append that only a writable open repairs"
    );
}

#[test]
fn open_read_only_refuses_a_store_one_leaf_past_its_pin_and_changes_no_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = FileLeafStore::create(&dir, ORIGIN).unwrap();
    store.put_leaf(0, b"leaf-0").unwrap();
    drop(store);
    let before = tree_bytes(&dir);
    let err = FileLeafStore::open_read_only(&dir).unwrap_err();
    assert!(
        matches!(
            &err,
            StoreError::RepairPending { path, pinned_size: 0, extent: 1 } if *path == dir
        ),
        "{err}"
    );
    assert_eq!(tree_bytes(&dir), before, "the refusal changed no file");
}

#[test]
fn open_read_only_on_an_uninitialized_dir_is_not_initialized() {
    let tmp = tempfile::tempdir().unwrap();
    let err = FileLeafStore::open_read_only(tmp.path()).unwrap_err();
    assert!(
        matches!(&err, StoreError::NotInitialized { path } if path == tmp.path()),
        "{err}"
    );
}

#[test]
fn a_read_only_handle_refuses_a_leaf_write_and_changes_no_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_pinned_over(&dir, b"leaf-0");
    let mut store = FileLeafStore::open_read_only(&dir).unwrap();
    let before = tree_bytes(&dir);
    for (index, bytes) in [(1, b"leaf-1"), (0, b"leaf-X")] {
        let err = store.put_leaf(index, bytes).unwrap_err();
        assert!(
            matches!(
                &err,
                StoreError::ReadOnly { path, operation: "write a leaf" } if *path == dir
            ),
            "leaf {index}: {err}"
        );
    }
    assert_eq!(store.extent(), 1);
    assert_eq!(tree_bytes(&dir), before, "the refusals changed no file");
}

#[test]
fn a_read_only_handle_refuses_a_pin_and_changes_no_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_pinned_over(&dir, b"leaf-0");
    let mut store = FileLeafStore::open_read_only(&dir).unwrap();
    let held = store.pinned();
    let before = tree_bytes(&dir);
    for tree_size in [2, 0] {
        let err = store
            .pin(PinnedRoot {
                tree_size,
                root: [0; 32],
            })
            .unwrap_err();
        assert!(
            matches!(&err, StoreError::ReadOnly { path, operation: "pin" } if *path == dir),
            "tree size {tree_size}: {err}"
        );
    }
    assert_eq!(store.pinned(), held);
    assert!(!dir.join("state.json.tmp").exists());
    assert_eq!(tree_bytes(&dir), before, "the refusals changed no file");
}

#[test]
fn a_read_only_handle_refuses_a_snapshot_write_and_changes_no_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_pinned_over(&dir, b"leaf-0");
    let mut store = FileLeafStore::open_read_only(&dir).unwrap();
    let before = tree_bytes(&dir);
    let err = store.put_snapshot(b"a snapshot").unwrap_err();
    assert!(
        matches!(&err, StoreError::ReadOnly { path, operation: "write a snapshot" } if *path == dir),
        "{err}"
    );
    assert_eq!(tree_bytes(&dir), before, "the refusal changed no file");
}

#[test]
fn leftover_temporaries_names_the_stores_own_temporary_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_pinned_over(&dir, b"a");
    let leaves = dir.join("leaves");
    let leftover = leaves.join(".4242-00000000000000000001-0.tmp");
    std::fs::write(&leftover, b"partial").unwrap();
    let expected = vec![".4242-00000000000000000001-0.tmp".to_string()];
    let writable = FileLeafStore::open(&dir).unwrap();
    assert_eq!(writable.extent(), 1);
    assert_eq!(writable.leftover_temporaries().unwrap(), expected);
    let read_only = FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(read_only.extent(), 1);
    assert_eq!(read_only.leftover_temporaries().unwrap(), expected);
    assert_eq!(std::fs::read(&leftover).unwrap(), b"partial");
}

#[test]
fn leftover_temporaries_are_in_lexical_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    store_pinned_over(&dir, b"a");
    for name in [
        ".4242-00000000000000000001-1.tmp",
        ".17-00000000000000000001-0.tmp",
    ] {
        std::fs::write(dir.join("leaves").join(name), b"partial").unwrap();
    }
    let store = FileLeafStore::open(&dir).unwrap();
    assert_eq!(
        store.leftover_temporaries().unwrap(),
        vec![
            ".17-00000000000000000001-0.tmp".to_string(),
            ".4242-00000000000000000001-1.tmp".to_string(),
        ]
    );
}

#[test]
fn leftover_temporaries_is_empty_without_one_of_the_stores_form() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut created = FileLeafStore::create(&dir, ORIGIN).unwrap();
    assert!(created.leftover_temporaries().unwrap().is_empty());
    created.put_leaf(0, b"a").unwrap();
    created.pin(pin_over(&[b"a".as_slice()])).unwrap();
    let store = FileLeafStore::open(&dir).unwrap();
    assert!(store.leftover_temporaries().unwrap().is_empty());
    assert_eq!(store.extent(), 1);
    std::fs::write(dir.join("leaves").join(".DS_Store"), b"finder").unwrap();
    let store = FileLeafStore::open(&dir).unwrap();
    assert!(store.leftover_temporaries().unwrap().is_empty());
    assert_eq!(store.extent(), 1);
}
