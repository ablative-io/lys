#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`Anchor::publish_checkpoint`].
//!
//! # Where the second party comes from
//!
//! - **`lys-core`'s `verify_checkpoint`**, written before this crate existed
//!   and knowing nothing about anchors, as the judge of every note emitted
//!   here. The axis is *implementation*, not language or toolchain: it is Rust
//!   in this workspace. The independence-of-algorithm claim belongs to
//!   `tests/checkpoint_note_conformance.rs`, which drives the same notes
//!   through Go `sumdb/note`, and is not claimed here.
//! - **The caller's own strings.** Verifiers are built from the origin the test
//!   handed `FileLeafStore::create`, never from `anchor.origin()`. Keying on a
//!   value this crate substituted would let a wrong origin pass, because both
//!   sides would be wrong together.
//! - **A hash computed outside Rust entirely.** [`SIZE_ONE_BODY`] is the exact
//!   body a one-leaf log must produce, and its root line was computed with a
//!   shell pipeline — `(printf '\x00'; printf '<genesis>') | openssl dgst
//!   -sha256 -binary | base64` — whose own correctness was first checked
//!   against two known values: `SHA-256("")` in base64 is
//!   `47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=`, and `SHA-256(0x00)` is
//!   `6e340b9c…afa01d`. The literal is written out here rather than recomputed,
//!   so no change to this crate can move it.
//!
//! # Every refusal opens with a positive control
//!
//! A verifier that rejects everything satisfies every "it was refused"
//! assertion at once. Each negative case below first asserts that the
//! unmodified note is accepted.

use std::path::Path;

use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_log_store::{FileLeafStore, LeafStore, Log};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::keys::{FileSigner, Signer};

use super::*;

/// The origin this test supplies to the store. Every verifier below is built
/// from *this* literal, not from what the anchor reports.
const ORIGIN: &str = "example.com/lys/anchor-checkpoint-test";

/// A second origin, so a constant or a fallback would have to be right twice.
const OTHER_ORIGIN: &str = "somewhere.else.invalid/another/log";

/// The genesis bytes whose RFC 6962 leaf hash is baked into [`SIZE_ONE_BODY`].
const GENESIS: &[u8] = b"lys-anchor increment 3 genesis fixture";

/// The complete checkpoint body a one-leaf log over [`GENESIS`] must produce,
/// origin line included. For a tree of size 1 the RFC 6962 root *is* the leaf
/// hash `SHA-256(0x00 ‖ leaf)`, so this whole string is checkable without any
/// lys code — see the module docs for the pipeline and its controls.
const SIZE_ONE_BODY: &str =
    "example.com/lys/anchor-checkpoint-test\n1\nsLS/MAYrS3mTIBQyIl4ugePbHY5SuN6Im7faYQtsLD0=\n";

/// `lys-core`'s Go-conformance fixture seed, reused so key material in tests is
/// deterministic and never generated.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a store at `dir` under `origin` and an anchor over it.
fn create_anchor(dir: &Path, origin: &str) -> Anchor<FileLeafStore, FileSigner> {
    let store = FileLeafStore::create(dir, origin).unwrap();
    Anchor::create(store, GENESIS, signer(dir), AnchorConfig::unconfigured()).unwrap()
}

/// Reopens the anchor at `dir` through a fresh store handle.
fn reopen(dir: &Path) -> Anchor<FileLeafStore, FileSigner> {
    Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        signer(dir),
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// The verifier a third party would build: the origin they were told, and the
/// public key they were given.
fn verifier_for(origin: &str, anchor: &Anchor<FileLeafStore, FileSigner>) -> NoteVerifierKey {
    NoteVerifierKey::new(origin, anchor.signer().public_key()).unwrap()
}

#[test]
fn a_published_checkpoint_verifies_under_the_origin_the_store_was_created_with() {
    let tmp = TempDir::new().unwrap();
    let anchor = create_anchor(tmp.path(), ORIGIN);

    let published = anchor.publish_checkpoint().unwrap();

    // The judge is lys-core's, and the verifier is built from the origin this
    // test supplied — not from anything the anchor reported back.
    let body = verify_checkpoint(published.note.as_bytes(), &verifier_for(ORIGIN, &anchor))
        .expect("a freshly published checkpoint must verify");

    // The `body` handed back with the note is the one inside it, not a second
    // computation of the same facts.
    assert_eq!(body, published.body);
    assert_eq!(body.origin(), ORIGIN);
    assert_eq!(body.tree_size(), 1);
}

#[test]
fn the_body_is_the_bytes_a_stranger_can_recompute_by_hand() {
    let tmp = TempDir::new().unwrap();
    let anchor = create_anchor(tmp.path(), ORIGIN);

    let published = anchor.publish_checkpoint().unwrap();

    // Origin line, size line and root line at once, against a literal whose
    // root was computed outside Rust. A symmetric change anywhere in this
    // crate's encode/decode pair cannot satisfy this.
    assert_eq!(published.body.encode(), SIZE_ONE_BODY);

    // And the note is that body, verbatim, followed by the signature block —
    // the signature covers the body including its trailing newline.
    assert!(
        published.note.starts_with(SIZE_ONE_BODY),
        "the note must open with the exact body bytes, got: {}",
        published.note
    );
}

#[test]
fn the_note_is_bound_to_the_origin_and_not_merely_to_the_key() {
    let tmp = TempDir::new().unwrap();
    let anchor = create_anchor(tmp.path(), ORIGIN);
    let published = anchor.publish_checkpoint().unwrap();

    // Positive control: the correct verifier accepts, so the refusal below is
    // about the name and not about a verifier that rejects everything.
    assert!(verify_checkpoint(published.note.as_bytes(), &verifier_for(ORIGIN, &anchor)).is_ok());

    // The one difference: the same public key, under a different name. A
    // checkpoint signed for one log must not be acceptable for another, and
    // that is what stops one anchor's key vouching for someone else's history.
    assert_ne!(ORIGIN, OTHER_ORIGIN);
    assert!(
        verify_checkpoint(
            published.note.as_bytes(),
            &verifier_for(OTHER_ORIGIN, &anchor)
        )
        .is_err(),
        "a checkpoint must not verify under a verifier named for a different log"
    );
}

#[test]
fn a_checkpoint_from_one_log_does_not_verify_for_a_differently_named_one() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();

    // Two anchors, two origins, one key — the confusion this binding exists to
    // refuse. Both are built the same way, so nothing but the origin differs.
    let a = create_anchor(first.path(), ORIGIN);
    let b = create_anchor(second.path(), OTHER_ORIGIN);
    assert_eq!(a.signer().public_key(), b.signer().public_key());

    let from_a = a.publish_checkpoint().unwrap();
    let from_b = b.publish_checkpoint().unwrap();

    // Positive control: each verifies under its own log's verifier.
    assert!(verify_checkpoint(from_a.note.as_bytes(), &verifier_for(ORIGIN, &a)).is_ok());
    assert!(verify_checkpoint(from_b.note.as_bytes(), &verifier_for(OTHER_ORIGIN, &b)).is_ok());

    // Crossed over, both are refused.
    assert!(verify_checkpoint(from_a.note.as_bytes(), &verifier_for(OTHER_ORIGIN, &b)).is_err());
    assert!(verify_checkpoint(from_b.note.as_bytes(), &verifier_for(ORIGIN, &a)).is_err());
}

#[test]
fn the_checkpoint_tracks_the_tree_it_is_published_over() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_anchor(dir, ORIGIN);

    // Grow the log underneath through `Log` directly: the anchor has no append
    // of its own until the submit path lands, and the point here is that
    // `publish_checkpoint` reads the tree rather than remembering a size.
    let mut roots = Vec::new();
    for size in 1..=4u64 {
        if size > 1 {
            let mut log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
            log.append(format!("entry {size}").as_bytes()).unwrap();
        }
        let anchor = reopen(dir);
        let published = anchor.publish_checkpoint().unwrap();

        let body = verify_checkpoint(published.note.as_bytes(), &verifier_for(ORIGIN, &anchor))
            .expect("every published checkpoint must verify");
        assert_eq!(body.tree_size(), size);
        roots.push(body.root_hash());
    }

    // Count what fired: a loop that ran zero or one times would satisfy every
    // assertion inside it while comparing nothing.
    assert_eq!(roots.len(), 4);
    for (earlier, later) in roots.iter().zip(roots.iter().skip(1)) {
        assert_ne!(
            earlier, later,
            "appending a leaf must change the root the checkpoint commits to"
        );
    }
}

#[test]
fn two_publications_of_an_unchanged_log_are_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let anchor = create_anchor(tmp.path(), ORIGIN);

    let first = anchor.publish_checkpoint().unwrap();
    let second = anchor.publish_checkpoint().unwrap();

    // No time enters a checkpoint, and Ed25519 signing is deterministic. Any
    // clock, counter or nonce that crept into the signed bytes would show up
    // here as two different artifacts for one unchanged log — and would also
    // mean two signatures over one state, which is a shape nobody downstream
    // can distinguish from carelessness.
    assert_eq!(first.note, second.note);
    assert_eq!(first.body, second.body);
}

#[test]
fn publishing_a_checkpoint_does_not_touch_the_log() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let anchor = create_anchor(dir, ORIGIN);

    let before = FileLeafStore::open(dir).unwrap().extent();
    let published = anchor.publish_checkpoint().unwrap();

    // Positive control: something was actually produced, so the equality below
    // is not the equality of two no-ops.
    assert!(!published.note.is_empty());

    assert_eq!(anchor.tree_size(), 1);
    // Read off the disk through a handle that never saw the call.
    assert_eq!(
        FileLeafStore::open(dir).unwrap().extent(),
        before,
        "an anchor must not log its own checkpoints"
    );
}
