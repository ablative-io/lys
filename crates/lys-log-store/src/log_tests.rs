#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the tree-over-storage layer.
//!
//! # A second implementation, so the trait is a seam and not a shape
//!
//! Most cases here run over [`FileLeafStore`], but [`MemStore`] implements
//! [`LeafStore`] independently — in memory, with its own copy of the write-once
//! and monotonic-pin rules. It earns its keep twice over: it proves the trait is
//! satisfiable by something that is not a directory (a trait with one
//! implementation has an untested shape), and it can be told to **fail a pin on
//! demand**, which is the only way to reach the half-completed append that
//! poisoning exists to contain. That state is unreachable through the file store
//! without crashing the process mid-call.

use std::path::Path;

use super::*;
use crate::file::FileLeafStore;

const ORIGIN: &str = "example.com/lys/log-test";

fn open_file_log(dir: &Path) -> Log<FileLeafStore> {
    FileLeafStore::create(dir, ORIGIN).unwrap();
    Log::open(FileLeafStore::open(dir).unwrap()).unwrap()
}

fn reopen(dir: &Path) -> StoreResult<Log<FileLeafStore>> {
    Log::open(FileLeafStore::open(dir)?)
}

fn leaf_path(dir: &Path, index: u64) -> std::path::PathBuf {
    dir.join("leaves").join(format!("{index:020}"))
}

/// An in-memory [`LeafStore`] holding the same rules as the file store, plus a
/// switch to make the next [`LeafStore::pin`] fail.
struct MemStore {
    origin: String,
    leaves: Vec<Vec<u8>>,
    pinned: PinnedRoot,
    /// A [`Cell`] so a test can arm it through `Log::store()`'s shared
    /// reference — the alternative was a test-only `store_mut` on the public
    /// [`Log`], and a type does not grow API to suit its tests.
    fail_next_pin: std::cell::Cell<bool>,
}

impl MemStore {
    fn new(origin: &str) -> Self {
        let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
        Self {
            origin: origin.to_string(),
            leaves: Vec::new(),
            pinned: PinnedRoot { tree_size, root },
            fail_next_pin: std::cell::Cell::new(false),
        }
    }
}

impl LeafStore for MemStore {
    fn origin(&self) -> &str {
        &self.origin
    }

    fn extent(&self) -> u64 {
        u64::try_from(self.leaves.len()).unwrap()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| self.leaves.get(i))
            .cloned())
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        if index < self.extent() {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent() {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent(),
            });
        }
        self.leaves.push(bytes.to_vec());
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.pinned
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        if self.fail_next_pin.replace(false) {
            return Err(StoreError::Io {
                context: "simulated pin failure".to_string(),
                source: std::io::Error::other("disk full"),
            });
        }
        if pin.tree_size < self.pinned.tree_size {
            return Err(StoreError::PinWentBackwards {
                pinned: self.pinned.tree_size,
                requested: pin.tree_size,
            });
        }
        self.pinned = pin;
        Ok(())
    }
}

#[test]
fn append_then_reopen_reproduces_the_root_and_the_golden_leaf_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    let (index0, hash0) = log.append(b"leaf-0").unwrap();
    let (index1, _hash1) = log.append(b"leaf-1").unwrap();
    assert_eq!((index0, index1), (0, 1));
    // Golden vector: SHA-256(0x00 || "leaf-0"), the RFC 6962 leaf hash. Pinned
    // as a literal so a change to the preimage cannot pass by agreeing with a
    // recomputation of itself.
    assert_eq!(
        STANDARD.encode(hash0),
        "MF31n5WQw8msY9KydDw4jjeSRJB4zr9/s9vmRxZDsrc="
    );
    assert_eq!(hash0, raw_leaf_hash(b"leaf-0"));
    let root_before = log.tree().root();
    let reopened = reopen(&dir).unwrap();
    assert_eq!(reopened.tree().root(), root_before);
    assert_eq!(reopened.leaf_bytes(0), Some(b"leaf-0".as_slice()));
    assert_eq!(reopened.leaf_bytes(1), Some(b"leaf-1".as_slice()));
    assert_eq!(reopened.leaf_bytes(2), None);
    assert_eq!(
        reopened.recovered_to(),
        None,
        "a clean open recovers nothing"
    );
}

#[test]
fn an_empty_leaf_is_legal() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    let (_index, hash) = log.append(b"").unwrap();
    assert_eq!(hash, raw_leaf_hash(b""));
    assert_eq!(reopen(&dir).unwrap().tree().len(), 1);
}

#[test]
fn a_tampered_leaf_byte_is_detected_at_open() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    std::fs::write(leaf_path(&dir, 0), b"leaf-X").unwrap();
    let err = reopen(&dir).unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
}

#[test]
fn a_tampered_pinned_size_is_detected_at_open() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    let state = std::fs::read_to_string(dir.join("state.json")).unwrap();
    let tampered = state.replacen("\"tree_size\": 1", "\"tree_size\": 2", 1);
    assert_ne!(state, tampered);
    std::fs::write(dir.join("state.json"), tampered).unwrap();
    let err = reopen(&dir).unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
}

#[test]
fn crash_recovery_repairs_exactly_one_interrupted_append_and_reports_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    let state_after_one = std::fs::read(dir.join("state.json")).unwrap();
    log.append(b"leaf-1").unwrap();
    // A crash between storing the leaf and advancing the pin.
    std::fs::write(dir.join("state.json"), &state_after_one).unwrap();
    let recovered = reopen(&dir).unwrap();
    assert_eq!(recovered.tree().len(), 2);
    assert_eq!(
        recovered.recovered_to(),
        Some(2),
        "the repair must be reportable, not silent"
    );
    // The pin was repaired on disk, so the next open is clean.
    let reread = reopen(&dir).unwrap();
    assert_eq!(reread.tree().len(), 2);
    assert_eq!(reread.recovered_to(), None);
}

#[test]
fn crash_recovery_does_not_mask_a_tampered_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    let state_after_one = std::fs::read(dir.join("state.json")).unwrap();
    log.append(b"leaf-1").unwrap();
    // Stale pin AND a tampered leaf inside the pinned prefix. The leaf count
    // matches the recoverable shape exactly, so only the prefix-root comparison
    // can refuse this.
    std::fs::write(leaf_path(&dir, 0), b"leaf-X").unwrap();
    std::fs::write(dir.join("state.json"), &state_after_one).unwrap();
    let err = reopen(&dir).unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
}

#[test]
fn recovery_refuses_a_pin_that_is_ahead_of_the_leaves() {
    // The mirror image of an interrupted append, and deliberately NOT
    // recoverable: a pin covering a leaf that was never stored describes a tree
    // nobody can rebuild. Storing the leaf after pinning would produce this
    // state, which is why append never pins first.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    let state_after_two = std::fs::read(dir.join("state.json")).unwrap();
    let fresh = tmp.path().join("fresh");
    let mut short = open_file_log(&fresh);
    short.append(b"leaf-0").unwrap();
    std::fs::write(fresh.join("state.json"), &state_after_two).unwrap();
    let err = reopen(&fresh).unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
}

#[test]
fn prefix_tree_matches_a_directly_built_tree_and_is_bounds_checked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    log.append(b"leaf-2").unwrap();
    let prefix = log.prefix_tree(2).unwrap();
    let direct = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves([b"leaf-0", b"leaf-1"]);
    assert_eq!(prefix.root(), direct.root());
    assert_eq!(log.prefix_tree(3).unwrap().root(), log.tree().root());
    assert_eq!(log.prefix_tree(0).unwrap().len(), 0);
    let err = log.prefix_tree(4).unwrap_err();
    assert!(
        matches!(err, StoreError::LeafWouldLeaveGap { index: 4, next: 3 }),
        "{err}"
    );
}

#[test]
fn a_log_runs_over_a_store_that_is_not_a_directory() {
    let mut log = Log::open(MemStore::new(ORIGIN)).unwrap();
    assert_eq!(log.origin(), ORIGIN);
    let (index, hash) = log.append(b"leaf-0").unwrap();
    assert_eq!((index, hash), (0, raw_leaf_hash(b"leaf-0")));
    log.append(b"leaf-1").unwrap();
    assert_eq!(log.tree().len(), 2);
    assert_eq!(log.store().pinned().tree_size, 2);
    assert_eq!(log.store().pinned().root, log.tree().root().to_parts().0);
}

#[test]
fn a_failed_pin_poisons_the_handle_instead_of_compounding_the_damage() {
    let mut log = Log::open(MemStore::new(ORIGIN)).unwrap();
    log.append(b"leaf-0").unwrap();
    log.store().fail_next_pin.set(true);
    let err = log.append(b"leaf-1").unwrap_err();
    assert!(matches!(err, StoreError::Io { .. }), "{err}");
    // The leaf IS stored — that is the recoverable one-ahead state.
    assert_eq!(log.store().extent(), 2);
    assert_eq!(log.store().pinned().tree_size, 1);
    // A further append would put storage two ahead of the pin, past what
    // recovery repairs. The handle refuses instead.
    let err = log.append(b"leaf-2").unwrap_err();
    assert!(matches!(err, StoreError::Poisoned), "{err}");
    assert_eq!(
        log.store().extent(),
        2,
        "the poisoned handle stored nothing"
    );
}

#[test]
fn a_store_that_breaks_its_contiguity_promise_is_named_as_the_culprit() {
    struct LyingStore(PinnedRoot);
    impl LeafStore for LyingStore {
        fn origin(&self) -> &str {
            ORIGIN
        }
        fn extent(&self) -> u64 {
            3
        }
        fn leaf(&self, _index: u64) -> StoreResult<Option<Vec<u8>>> {
            Ok(None)
        }
        fn put_leaf(&mut self, _index: u64, _bytes: &[u8]) -> StoreResult<()> {
            Ok(())
        }
        fn pinned(&self) -> PinnedRoot {
            self.0
        }
        fn pin(&mut self, _pin: PinnedRoot) -> StoreResult<()> {
            Ok(())
        }
    }
    let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
    let err = Log::open(LyingStore(PinnedRoot { tree_size, root })).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::LeafMissingWithinExtent {
                index: 0,
                extent: 3
            }
        ),
        "{err}"
    );
}

#[test]
fn debug_summarizes_without_leaf_content() {
    let mut log = Log::open(MemStore::new(ORIGIN)).unwrap();
    log.append(b"secret-looking-leaf-content").unwrap();
    let rendered = format!("{log:?}");
    assert!(
        !rendered.contains("secret-looking-leaf-content"),
        "{rendered}"
    );
    assert!(rendered.contains("num_leaves: 1"), "{rendered}");
}

#[test]
fn validate_origin_matches_what_a_checkpoint_would_accept() {
    validate_origin("example.com/lys/ok").unwrap();
    let bad_origins = ["", "has space", "has+plus"];
    for bad in bad_origins {
        assert!(validate_origin(bad).is_err(), "{bad:?}");
    }
    assert_eq!(bad_origins.len(), 3, "an empty list would pass vacuously");
}
