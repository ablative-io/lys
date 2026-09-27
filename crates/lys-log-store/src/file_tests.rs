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
fn an_unexpected_leaves_entry_is_detected_but_dotfiles_are_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = create(&dir);
    store.put_leaf(0, b"leaf-0").unwrap();
    std::fs::write(dir.join("leaves").join(".DS_Store"), b"junk").unwrap();
    assert!(
        FileLeafStore::open(&dir).is_ok(),
        "dotfiles must be ignored"
    );
    std::fs::write(dir.join("leaves").join("stray.txt"), b"junk").unwrap();
    let err = FileLeafStore::open(&dir).unwrap_err();
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
