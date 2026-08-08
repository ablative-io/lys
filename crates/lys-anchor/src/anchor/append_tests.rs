#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`Anchor::append`] — the ungated write path.
//!
//! # What this file is for, and what it deliberately leaves to `submit_tests`
//!
//! `submit_tests` already holds the receipt against the checkpoint, the leaf
//! hash against `sha2` and against literals computed outside Rust, and the leaf
//! bytes against a handle that never saw the write. **Repeating those here
//! would be one party agreeing with itself through a second entry point**, and
//! `submit` now *is* this function plus `receipt_for`, so those assertions
//! already exercise this code.
//!
//! What is new, and is only checkable here, is the pair of claims the split was
//! made to establish:
//!
//! - **This path runs the admission policy.** It is the ungated write path, so
//!   a default build reaches it without `unstable-anchor`; an `append` that
//!   skipped the policy would hand every default deployment a write path the
//!   operator never authorised. `submit_tests`'s admission gates cannot see
//!   that, because they run only in the feature-full shape.
//! - **This path is reachable, and reaches past tree size 1.** That is the
//!   whole defect: with the gate on the verb, a default-features anchor could
//!   never hold a second leaf.
//!
//! # Where the second party comes from
//!
//! - **The disk.** Leaf bytes and the tree extent are read back through a
//!   `Log` opened fresh over the same directory, which never saw the append.
//!   An in-memory count agreeing with itself would not distinguish an append
//!   that landed from one that returned.
//! - **`sha2`, driven by this file**, for the one place the leaf hash is
//!   checked — never the helper the append path used.
//! - **A refusal count.** The admission gate asserts how many refusals fired,
//!   not merely that the successes succeeded: a policy that was never consulted
//!   and a policy that admitted everything are indistinguishable by a test that
//!   only counts passes.
//!
//! The axis is *implementation*, not platform: one machine, one toolchain.

use std::path::Path;

use lys_log_store::{FileLeafStore, Log};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{AcceptAll, AdmissionPolicy, MaxSize, SubmitterContext};
use crate::error::AnchorError;
use crate::keys::FileSigner;

use super::*;

/// The origin this test supplies to the store.
const ORIGIN: &str = "example.com/lys/anchor-append-test";

/// The genesis bytes for every anchor built here.
const GENESIS: &[u8] = b"lys-anchor append gate genesis fixture";

/// `lys-core`'s conformance fixture seed, so key material is deterministic.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a store at `dir` under [`ORIGIN`] and an anchor over it, under a
/// named admission policy.
fn create_anchor_with<P: AdmissionPolicy>(
    dir: &Path,
    policy: P,
) -> Anchor<FileLeafStore, FileSigner, P> {
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(
        store,
        GENESIS,
        signer(dir),
        policy,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// The number of leaves on disk, through a handle that never saw the append.
fn extent_from_disk(dir: &Path) -> u64 {
    Log::open(FileLeafStore::open(dir).unwrap())
        .unwrap()
        .tree()
        .len()
}

/// The bytes at `index` on disk, through a handle that never saw the append.
fn leaf_from_disk(dir: &Path, index: u64) -> Vec<u8> {
    let log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    log.leaf_bytes(index)
        .expect("the leaf must be on disk")
        .to_vec()
}

#[test]
fn appending_reaches_past_the_genesis_leaf_and_keeps_going() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor_with(dir, AcceptAll);

    // The defect this file exists for: before `append` was ungated, a
    // default-features anchor stopped here, at size 1.
    assert_eq!(anchor.tree_size(), 1);

    let statements: [&[u8]; 3] = [b"first", b"second", b"third"];
    let mut appended = Vec::new();
    for statement in statements {
        appended.push(
            anchor
                .append(Submission { statement }, SubmitterContext::Unidentified)
                .unwrap(),
        );
    }

    // Count what fired: three appends, three distinct indices, each one past
    // genesis, and the tree grew by exactly three.
    assert_eq!(appended.len(), 3);
    let indices: Vec<u64> = appended.iter().map(|outcome| outcome.leaf_index).collect();
    assert_eq!(indices, vec![1, 2, 3]);
    let sizes: Vec<u64> = appended.iter().map(|outcome| outcome.tree_size).collect();
    assert_eq!(sizes, vec![2, 3, 4]);

    // And the disk agrees, through a handle that never saw any of it.
    assert_eq!(extent_from_disk(dir), 4);
    for (offset, statement) in statements.iter().enumerate() {
        let index = u64::try_from(offset).unwrap() + 1;
        assert_eq!(leaf_from_disk(dir, index), statement.to_vec());
    }
}

#[test]
fn the_leaf_hash_is_sha256_of_the_tag_byte_and_the_statement() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor_with(tmp.path(), AcceptAll);
    let statement: &[u8] = b"lys-anchor append gate statement fixture";

    let outcome = anchor
        .append(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap();

    // Driven by this file, not by the helper the append path used.
    let mut hasher = Sha256::new();
    hasher.update([0x00_u8]);
    hasher.update(statement);
    let expected: [u8; 32] = hasher.finalize().into();
    assert_eq!(outcome.leaf_hash, expected);
}

#[test]
fn the_statement_is_stored_verbatim() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor_with(dir, AcceptAll);

    // Not valid UTF-8, contains a NUL, and would be mangled by any
    // canonicalization or text handling on the way to storage.
    let statement: &[u8] = &[0xff, 0x00, 0x0a, b'{', 0xc3, 0x28, b'}', 0x0d];
    let outcome = anchor
        .append(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap();

    assert_eq!(leaf_from_disk(dir, outcome.leaf_index), statement.to_vec());
}

#[test]
fn identical_bytes_appended_twice_are_two_leaves_at_two_indices() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor_with(dir, AcceptAll);
    let statement: &[u8] = b"the same bytes, twice";

    let first = anchor
        .append(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap();
    let second = anchor
        .append(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap();

    assert_ne!(first.leaf_index, second.leaf_index);
    // The same bytes hash the same; that is not de-duplication, it is a hash.
    assert_eq!(first.leaf_hash, second.leaf_hash);
    assert_eq!(extent_from_disk(dir), 3);
}

#[test]
fn the_admission_policy_runs_on_the_ungated_path_and_a_refusal_appends_nothing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    // Admits the empty statement and nothing longer, so this file supplies
    // both an admitted case and a refused one under one rule.
    let mut anchor = create_anchor_with(dir, MaxSize::new(0));

    let refused: [&[u8]; 3] = [b"a", b"ab", b"abc"];
    let mut refusals = 0_u32;
    for statement in refused {
        match anchor.append(Submission { statement }, SubmitterContext::Unidentified) {
            Err(AnchorError::NotAdmitted) => refusals += 1,
            other => panic!("a refusal was expected, got {other:?}"),
        }
    }

    // Count what fired. A policy that was never consulted and one that admitted
    // everything are the same thing to a test that only counts successes.
    assert_eq!(refusals, 3);

    // A refusal leaves no trace, and the disk is what says so.
    assert_eq!(anchor.tree_size(), 1);
    assert_eq!(extent_from_disk(dir), 1);

    // The positive leg, so the refusals above are not a policy that refuses
    // everything nor an `append` that is simply broken.
    let admitted = anchor
        .append(
            Submission { statement: b"" },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    assert_eq!(admitted.leaf_index, 1);
    assert_eq!(extent_from_disk(dir), 2);
}

#[test]
fn what_the_policy_is_told_about_the_submitter_reaches_it_and_no_leaf() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor_with(dir, AcceptAll);
    let statement: &[u8] = b"a statement with a credential beside it";
    let credential: &[u8] = b"a credential the log must never hold";

    let outcome = anchor
        .append(
            Submission { statement },
            SubmitterContext::AssertedBySubmitter(credential),
        )
        .unwrap();

    // The leaf is the statement, verbatim, and nothing about who sent it.
    let leaf = leaf_from_disk(dir, outcome.leaf_index);
    assert_eq!(leaf, statement.to_vec());
    assert!(
        !leaf
            .windows(credential.len())
            .any(|window| window == credential),
        "the credential must not reach the leaf"
    );
}
