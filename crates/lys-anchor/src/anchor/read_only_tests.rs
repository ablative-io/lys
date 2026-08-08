#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`Anchor::open_read_only`] and [`Anchor::root`].
//!
//! # What is checked where
//!
//! **That a read-only anchor cannot sign is not checkable in this file**, and
//! saying so is the point: it is a resolution failure, so the only thing that
//! can observe it is a compiler run over a snippet that is expected not to
//! build. That gate is the `compile_fail` doctest pair on
//! [`Anchor::open_read_only`], which carries its own positive control — a
//! snippet identical but for the one line that signs, so a `compile_fail` that
//! passes because of a typo is told apart from one that passes because the
//! method is absent.
//!
//! What is checkable here is that the reader is a *faithful* reader: it sees
//! the same origin, extent, root and bytes a writing anchor over the same
//! directory sees, and it refuses the same logs the write path refuses. A
//! read-only handle that quietly accepted a log without genesis would let a
//! status command report happily on a log that can never issue a receipt.
//!
//! # Where the second party comes from
//!
//! The **writing** anchor, and the **disk**. Every value the reader reports is
//! held against one produced by a different handle over the same directory —
//! and the root is additionally held against the one inside a signed
//! checkpoint, which reaches the tree by another route entirely.

use std::path::Path;

use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_log_store::FileLeafStore;
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{AcceptAll, SubmitterContext};
use crate::error::AnchorError;
use crate::keys::{FileSigner, Signer};
use crate::wire::Submission;

use super::*;

/// The origin this test supplies to the store.
const ORIGIN: &str = "example.com/lys/anchor-read-only-test";

/// The genesis bytes for every anchor built here.
const GENESIS: &[u8] = b"lys-anchor read-only gate genesis fixture";

/// `lys-core`'s conformance fixture seed, so key material is deterministic.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a store at `dir` under [`ORIGIN`] and an anchor over it.
fn create_anchor(dir: &Path) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(
        store,
        GENESIS,
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// Opens the anchor at `dir` for reading, with no key and no policy.
fn read_only(dir: &Path) -> ReadOnlyAnchor<FileLeafStore> {
    Anchor::open_read_only(
        FileLeafStore::open(dir).unwrap(),
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

#[test]
fn a_reader_needs_neither_a_key_file_nor_a_policy_to_report_what_the_log_holds() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let statements: [&[u8]; 2] = [b"first statement", b"second statement"];
    let (writer_root, writer_size) = {
        let mut anchor = create_anchor(dir);
        for statement in statements {
            anchor
                .append(Submission { statement }, SubmitterContext::Unidentified)
                .unwrap();
        }
        (anchor.root(), anchor.tree_size())
    };

    // The key file is still on disk, but nothing below reads it: the reader is
    // constructed from the store and the config alone.
    let reader = read_only(dir);

    assert_eq!(reader.origin(), ORIGIN);
    assert_eq!(reader.tree_size(), writer_size);
    assert_eq!(reader.root(), writer_root);
    assert_eq!(reader.status().tree_size(), writer_size);
    assert_eq!(reader.recovered_to(), None);

    let mut read_back = 0_u32;
    for (offset, statement) in statements.iter().enumerate() {
        let index = u64::try_from(offset).unwrap() + 1;
        assert_eq!(reader.leaf_bytes(index).unwrap(), *statement);
        read_back += 1;
    }
    assert_eq!(read_back, 2, "both leaves must have been read back");
    assert_eq!(reader.leaf_bytes(writer_size), None);
}

#[test]
fn the_root_a_reader_reports_is_the_root_a_signed_checkpoint_commits_to() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor(dir);
    anchor
        .append(
            Submission {
                statement: b"a statement, so the tree is not the trivial one",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    let published = anchor.publish_checkpoint().unwrap();
    let public_key = anchor.signer().public_key();
    drop(anchor);

    // A handle that has no key at all, against a note that was signed by one.
    let reader = read_only(dir);
    let verifier = NoteVerifierKey::new(ORIGIN, public_key).unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier).unwrap();

    assert_eq!(reader.root().as_bytes(), body.root_hash());
    assert_eq!(reader.root().num_leaves(), body.tree_size());
}

#[test]
fn a_reader_refuses_a_log_with_no_genesis_leaf_exactly_as_open_does() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    // A store with no leaves at all — never handed to `Anchor::create`.
    FileLeafStore::create(dir, ORIGIN).unwrap();

    match Anchor::open_read_only(
        FileLeafStore::open(dir).unwrap(),
        AnchorConfig::unconfigured(),
    ) {
        Err(AnchorError::NoGenesisLeaf { origin }) => assert_eq!(origin, ORIGIN),
        other => panic!("a genesis-less log must be refused, got {other:?}"),
    }
}

#[test]
fn reading_an_anchor_does_not_change_it() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let before = {
        let anchor = create_anchor(dir);
        anchor.root()
    };

    // Every read on the surface a reader has, twice over.
    for _ in 0..2 {
        let reader = read_only(dir);
        let status = reader.status();
        assert_eq!(status.tree_size(), 1);
        assert_eq!(reader.origin(), ORIGIN);
        assert_eq!(reader.leaf_bytes(0).unwrap(), GENESIS);
    }

    assert_eq!(read_only(dir).root(), before);
}
