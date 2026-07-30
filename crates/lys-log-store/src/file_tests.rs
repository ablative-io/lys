#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the file-backed store.
//!
//! # The two absent-checks are tested SEPARATELY, on purpose
//!
//! [`FileLeafStore::put_leaf`] refuses a re-used index twice over: once against
//! the cached `extent`, and once via `create_new` at the filesystem. Two checks
//! guarding one rule is how a check rots unnoticed — remove either and the
//! obvious test (append twice at index 0) still fails, because *the other one*
//! catches it. A drift injection that leaves the suite red has proven nothing
//! about which check is load-bearing.
//!
//! So each is isolated by a case only it can catch:
//!
//! | injection | the only case that fails |
//! |---|---|
//! | remove the `index < extent` check | `extent_check_alone_refuses_a_deleted_leafs_index` — the file is *gone*, so `create_new` succeeds and would silently rewrite history |
//! | remove `create_new` | `create_new_alone_refuses_a_leaf_this_store_never_saw` — the index *is* the next free one, so the extent check is satisfied and another writer's leaf would be clobbered |
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
    // store's back, so `create_new` would succeed. Only the extent check can
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
fn create_new_alone_refuses_a_leaf_this_store_never_saw() {
    // Isolates the `create_new` check: index 1 IS the next free index as far as
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
