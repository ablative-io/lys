#![cfg(test)]
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
use crate::file::{FileLeafStore, tamper};

const ORIGIN: &str = "example.com/lys/log-test";

fn open_file_log(dir: &Path) -> Log<FileLeafStore> {
    FileLeafStore::create(dir, ORIGIN).unwrap();
    Log::open(FileLeafStore::open(dir).unwrap()).unwrap()
}

fn reopen(dir: &Path) -> StoreResult<Log<FileLeafStore>> {
    Log::open(FileLeafStore::open(dir)?)
}

/// An in-memory [`LeafStore`] holding the same rules as the file store, plus a
/// switch to make the next [`LeafStore::pin`] fail.
struct MemStore {
    origin: String,
    leaves: Vec<Vec<u8>>,
    pinned: PinnedRoot,
    snapshot: Option<Vec<u8>>,
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
            snapshot: None,
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

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        if index < self.extent() {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent() {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent(),
            });
        }
        // One act: a pin that fails stores none of its leaves.
        self.pin(pin)?;
        self.leaves
            .extend(leaves.iter().map(|bytes| bytes.to_vec()));
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

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.snapshot = Some(bytes.to_vec());
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
    log.append(b"leaf-1").unwrap();
    drop(log);
    // Leaf 0 has a second act behind it, so its damaged record is not the
    // store's tail and cannot be read as an act that never finished.
    tamper::tamper_leaf_byte(&dir, 0);
    let err = reopen(&dir).unwrap_err();
    assert!(
        matches!(err, StoreError::CorruptRecord { offset: 0, .. }),
        "{err}"
    );
}

#[test]
fn a_damaged_pin_is_never_served_and_is_named_at_open() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    let segment = tamper::segment_of(&dir, 1);
    tamper::tamper_pin_byte(&dir, 1);
    // The pin rides inside its act's last record, under the record's
    // checksum: a pin that does not check is an act that is not there.
    let read = FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(read.extent(), 1);
    assert_eq!(read.pinned().tree_size, 1);
    let tail = read.unfinished_tail().expect("the damaged act is named");
    assert_eq!(tail.segment, segment);
    drop(read);
    let log = reopen(&dir).unwrap();
    assert_eq!(log.tree().len(), 1);
    assert_eq!(log.recovered_to(), None);
}

#[test]
fn an_interrupted_append_is_cut_and_reported_never_adopted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    // A crash inside the second act: three bytes of its record reached the
    // segment and its flush never returned.
    tamper::cut_inside_record(&dir, 1);
    let recovered = reopen(&dir).unwrap();
    assert_eq!(recovered.tree().len(), 1);
    assert_eq!(
        recovered.recovered_to(),
        None,
        "nothing past the pin is adopted"
    );
    let cut = recovered
        .store()
        .unfinished_tail()
        .expect("the cut must be reportable, not silent")
        .bytes;
    assert_eq!(cut, 3);
    drop(recovered);
    // The cut was made on disk, so the next open is clean.
    let reread = reopen(&dir).unwrap();
    assert_eq!(reread.tree().len(), 1);
    assert!(reread.store().unfinished_tail().is_none());
}

#[test]
fn crash_recovery_does_not_mask_a_tampered_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    // An interrupted second act AND a planted leaf inside the pinned prefix,
    // behind a checksum that holds. Cutting the unfinished act must not excuse
    // the prefix: only the tree comparison can refuse this.
    tamper::plant_leaf(&dir, 0, b"leaf-X");
    tamper::cut_inside_record(&dir, 1);
    let err = reopen(&dir).unwrap_err();
    assert!(matches!(err, StoreError::PinMismatch { .. }), "{err}");
}

/// Appends `leaf-0`, `leaf-1` and `leaf-2` to a fresh store with origin
/// `example.com/log`, returning the root after the third append.
fn three_leaf_store(dir: &Path) -> [u8; 32] {
    FileLeafStore::create(dir, "example.com/log").unwrap();
    let mut log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    for leaf in [b"leaf-0".as_slice(), b"leaf-1", b"leaf-2"] {
        log.append(leaf).unwrap();
    }
    let (root, size) = log.tree().root().to_parts();
    assert_eq!(size, 3);
    root
}

/// The root the file store's refusal must state for the given leaves,
/// computed from the bytes the test planted rather than from what open read.
fn rebuilt_root_b64(leaves: &[&[u8]]) -> String {
    let owned: Vec<Vec<u8>> = leaves.iter().copied().map(<[u8]>::to_vec).collect();
    let tree = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(&owned);
    STANDARD.encode(tree.root().to_parts().0)
}

/// Replaces leaf 1 of a three-leaf store with `planted`, opens it, and returns
/// the refusal's Display text and roots, holding every file unchanged. The
/// leaf is planted behind a checksum that holds, so only the tree can tell.
fn refusal_over_planted_leaf_1(planted: &[u8]) -> (String, String, String) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let pinned_root = STANDARD.encode(three_leaf_store(&dir));
    tamper::plant_leaf(&dir, 1, planted);
    let before = tree_bytes(&dir);
    let err = reopen(&dir).unwrap_err();
    assert_eq!(tree_bytes(&dir), before, "a refused open must not write");
    let display = err.to_string();
    let (pinned_size, got_pinned_root, rebuilt_size, rebuilt_root) = match err {
        StoreError::PinMismatch {
            pinned_size,
            pinned_root,
            rebuilt_size,
            rebuilt_root,
        } => (pinned_size, pinned_root, rebuilt_size, rebuilt_root),
        other => panic!("expected PinMismatch, got {other}"),
    };
    assert_eq!(pinned_size, 3);
    assert_eq!(rebuilt_size, 3);
    assert_eq!(got_pinned_root, pinned_root);
    assert_eq!(
        rebuilt_root,
        rebuilt_root_b64(&[b"leaf-0", planted, b"leaf-2"])
    );
    assert_ne!(rebuilt_root, pinned_root);
    (display, rebuilt_root, pinned_root)
}

#[test]
fn a_torn_leaf_inside_the_pinned_prefix_is_refused_with_the_pin_and_the_rebuilt_root() {
    let (display, rebuilt_root, pinned_root) = refusal_over_planted_leaf_1(b"lea");
    assert_eq!(
        display,
        format!(
            "stored leaves rebuild to tree size 3 with root {rebuilt_root}, but the pinned \
             state is tree size 3 with root {pinned_root}"
        )
    );
}

#[test]
fn a_one_byte_change_inside_the_pinned_prefix_is_refused_with_the_pin_and_the_rebuilt_root() {
    let (display, rebuilt_root, pinned_root) = refusal_over_planted_leaf_1(b"leaf-X");
    assert_eq!(
        display,
        format!(
            "stored leaves rebuild to tree size 3 with root {rebuilt_root}, but the pinned \
             state is tree size 3 with root {pinned_root}"
        )
    );
}

#[test]
fn an_act_lost_whole_leaves_the_log_at_its_earlier_pin() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    // A crash before any byte of the second act reached the segment.
    tamper::cut_before_record(&dir, 1);
    let reopened = reopen(&dir).unwrap();
    assert_eq!(reopened.tree().len(), 1);
    assert_eq!(reopened.recovered_to(), None);
    assert!(
        reopened.store().unfinished_tail().is_none(),
        "nothing was there to cut"
    );
}

#[test]
fn recovery_refuses_a_pin_that_is_ahead_of_the_leaves() {
    // A pin covering a leaf that was never stored describes a tree nobody can
    // rebuild. The pin rides on its act's last record, so the only way to hold
    // one is a record whose pin names a size past its own leaf, planted behind
    // a checksum that holds; the open refuses it by record.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    let (root, tree_size) = log.tree().root().to_parts();
    assert_eq!(tree_size, 2);
    let fresh = tmp.path().join("fresh");
    let mut short = open_file_log(&fresh);
    short.append(b"leaf-0").unwrap();
    drop(short);
    tamper::plant_pin(&fresh, 0, PinnedRoot { tree_size, root });
    let err = reopen(&fresh).unwrap_err();
    assert!(
        matches!(err, StoreError::CorruptRecord { offset: 0, .. }),
        "{err}"
    );
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
        fn append(&mut self, _index: u64, _leaves: &[&[u8]], _pin: PinnedRoot) -> StoreResult<()> {
            Ok(())
        }
        fn pinned(&self) -> PinnedRoot {
            self.0
        }
        fn pin(&mut self, _pin: PinnedRoot) -> StoreResult<()> {
            Ok(())
        }
        fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
            Ok(None)
        }
        fn put_snapshot(&mut self, _bytes: &[u8]) -> StoreResult<()> {
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

/// Every file under `dir`, dot-prefixed names included, with its bytes, so a
/// test can say an open added, removed and changed nothing.
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

#[test]
fn a_log_opens_over_a_read_only_store_at_its_pin_and_changes_no_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    let before = tree_bytes(&dir);
    let read = Log::open(FileLeafStore::open_read_only(&dir).unwrap()).unwrap();
    assert_eq!(read.tree().len(), 2);
    assert_eq!(read.recovered_to(), None);
    assert_eq!(
        tree_bytes(&dir),
        before,
        "the read-only open changed no file"
    );
}

#[test]
fn a_read_only_store_with_an_unfinished_act_serves_its_pin_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    tamper::cut_inside_record(&dir, 1);
    let before = tree_bytes(&dir);
    let store = FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(store.extent(), 1);
    assert_eq!(store.unfinished_tail().map(|tail| tail.bytes), Some(3));
    let read = Log::open(store).unwrap();
    assert_eq!(read.tree().len(), 1);
    assert_eq!(read.recovered_to(), None);
    assert_eq!(tree_bytes(&dir), before, "the reader changed no file");
}

#[test]
fn a_writable_open_cuts_an_unfinished_act_and_a_reader_then_finds_none() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    log.append(b"leaf-0").unwrap();
    log.append(b"leaf-1").unwrap();
    drop(log);
    tamper::cut_inside_record(&dir, 1);
    let store = FileLeafStore::open(&dir).unwrap();
    assert_eq!(store.extent(), 1);
    assert_eq!(store.pinned().tree_size, 1);
    assert_eq!(store.unfinished_tail().map(|tail| tail.bytes), Some(3));
    drop(store);
    let read = FileLeafStore::open_read_only(&dir).unwrap();
    assert_eq!(read.extent(), 1);
    assert_eq!(read.pinned().tree_size, 1);
    assert!(read.unfinished_tail().is_none());
}

#[test]
fn a_torn_leaf_inside_the_pinned_prefix_is_refused_with_both_trees() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut log = open_file_log(&dir);
    for leaf in [b"leaf-0", b"leaf-1", b"leaf-2"] {
        log.append(leaf).unwrap();
    }
    let root_after_three = log.tree().root().to_parts().0;
    drop(log);
    tamper::plant_leaf(&dir, 1, b"lea");
    let before = tree_bytes(&dir);
    let leaves = [b"leaf-0".to_vec(), b"lea".to_vec(), b"leaf-2".to_vec()];
    let torn_root = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(&leaves)
        .root()
        .to_parts()
        .0;
    let pinned_b64 = STANDARD.encode(root_after_three);
    let rebuilt_b64 = STANDARD.encode(torn_root);
    let err = reopen(&dir).unwrap_err();
    assert!(
        matches!(
            &err,
            StoreError::PinMismatch { pinned_size: 3, pinned_root, rebuilt_size: 3, rebuilt_root }
                if *pinned_root == pinned_b64 && *rebuilt_root == rebuilt_b64
        ),
        "{err}"
    );
    assert_eq!(
        err.to_string(),
        format!(
            "stored leaves rebuild to tree size 3 with root {rebuilt_b64}, but the pinned state \
             is tree size 3 with root {pinned_b64}"
        )
    );
    assert_eq!(tree_bytes(&dir), before, "a refused open writes nothing");
}

#[test]
fn a_log_opened_at_its_pin_leaves_the_pin_and_refuses_to_append_while_a_repair_is_pending() {
    // The file store cannot hold a leaf past its pin: leaves and pin are one
    // act there. A backend that can is what this path exists for, so the state
    // is built in the memory store, which this module owns.
    let ahead = || {
        let mut frontier = Frontier::new();
        frontier.push(b"leaf-0");
        let mut store = MemStore::new(ORIGIN);
        let pin = PinnedRoot {
            tree_size: frontier.size(),
            root: frontier.root(),
        };
        store.append(0, &[b"leaf-0".as_slice()], pin).unwrap();
        // An append interrupted after storing its leaf and before its pin.
        store
            .leaves
            .push(b"an append interrupted before its pin".to_vec());
        store
    };
    {
        let mut log = Log::open_at_pin(ahead()).unwrap();
        assert_eq!(log.tree().len(), 1);
        assert_eq!(log.pending_repair(), Some(2));
        assert_eq!(log.recovered_to(), None);
        assert_eq!(log.store().pinned().tree_size, 1);
        let err = log.append(b"leaf-2").unwrap_err();
        assert!(
            matches!(
                err,
                StoreError::AppendAwaitsRepair {
                    pinned_tree_size: 1,
                    leaves: 2
                }
            ),
            "{err}"
        );
        assert!(!matches!(err, StoreError::Poisoned), "{err}");
        assert_eq!(
            err.to_string(),
            "refusing to append: the log has a pending repair, its store holds 2 leaves but its \
             pin is at tree size 1; a writable open must repair it before any append"
        );
        assert_eq!(log.store().pinned().tree_size, 1, "the pin was left");
        assert_eq!(log.store().leaves.len(), 2, "no leaf was written");
    }

    // Only a full open repairs it, as it always has.
    let mut repaired = Log::open(ahead()).unwrap();
    assert_eq!(repaired.recovered_to(), Some(2));
    assert_eq!(repaired.pending_repair(), None);
    assert_eq!(repaired.store().pinned().tree_size, 2);
    assert_eq!(repaired.append(b"leaf-2").unwrap().0, 2);
    assert_eq!(repaired.tree().len(), 3);
    assert_eq!(repaired.store().pinned().tree_size, 3);
}
