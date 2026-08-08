#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`Anchor::status`], [`AnchorStatus`] and [`WitnessPosture`].
//!
//! # Which assertion here is evidence and which is not, said before either
//!
//! [`the_posture_of_a_fresh_anchor_is_unwitnessed`] **cannot fail** while
//! `WitnessPosture` has one variant. It is written because BUILD-PLAN §7.2 asks
//! for it and because it becomes load-bearing the instant increment 8 adds a
//! second variant — but a check that cannot fire is indistinguishable from one
//! that passed, and pretending otherwise is worse than not writing it. It is
//! not evidence today and is labelled so in place.
//!
//! The assertions that *are* live today are the ones a person editing this
//! crate can break:
//!
//! - **The disclosure's wording**, checked against phrases written out in this
//!   file rather than against the constant it is rendered from. Comparing
//!   `WitnessPosture::Unwitnessed.to_string()` with [`STANDALONE_DISCLOSURE`]
//!   would be the implementation agreeing with itself: both sides are the same
//!   `const`, and deleting the sentence from that `const` would keep them
//!   equal. So the phrases below are the second party, and the equality against
//!   the constant is asserted only *after* them, to catch a `Display` that
//!   renders something other than the whole constant.
//! - **The root**, held against the root inside a signed checkpoint. Two
//!   different routes out of the tree — the accessor, and base64 through a note
//!   that `verify_checkpoint` parses back — so a status that reported a root
//!   from anywhere but the log is caught.
//! - **The size**, held against a `Log` opened fresh over the same directory.
//! - **The origin**, held against the literal this file supplied to the store,
//!   never against what the anchor reported back.

use std::path::Path;

use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_log_store::{FileLeafStore, Log};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{AcceptAll, SubmitterContext};
use crate::keys::{FileSigner, Signer};
use crate::wire::Submission;

use super::*;

/// The origin this test supplies to the store. The verifier is built from
/// *this* literal, never from what the anchor reports back.
const ORIGIN: &str = "example.com/lys/anchor-status-test";

/// The genesis bytes for every anchor built here.
const GENESIS: &[u8] = b"lys-anchor status gate genesis fixture";

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

#[test]
fn the_posture_of_a_fresh_anchor_is_unwitnessed() {
    let tmp = TempDir::new().unwrap();
    let anchor = create_anchor(tmp.path());

    // ⚠️ Vacuous today: `WitnessPosture` has one variant, so this assertion
    // cannot fail and proves nothing. BUILD-PLAN §7.2 asks for it, and it
    // becomes real the moment increment 8's upward pin makes a second variant
    // expressible. The live checks in this file are the ones below.
    assert_eq!(anchor.status().posture, WitnessPosture::Unwitnessed);
}

#[test]
fn displaying_the_posture_states_the_equivocation_limit_in_full() {
    let rendered = WitnessPosture::Unwitnessed.to_string();

    // Written out here rather than read from the constant: a check against
    // `STANDALONE_DISCLOSURE` would survive the sentence being deleted from
    // `STANDALONE_DISCLOSURE`.
    let required = [
        "no witnesses",
        "equivocate undetectably",
        "hold two histories",
        "no local check catches that",
        "one external party keeping its own durable memory",
    ];
    let mut found = 0_u32;
    for phrase in required {
        assert!(
            rendered.contains(phrase),
            "the disclosure must contain {phrase:?}, got {rendered:?}"
        );
        found += 1;
    }
    assert_eq!(found, 5, "every required phrase must have been checked");

    // And the whole constant, so a `Display` that renders a shortened form of
    // it — the exact way a disclosure gets quietly dropped — is caught too.
    assert_eq!(rendered, STANDALONE_DISCLOSURE);
}

#[test]
fn the_status_root_is_the_root_the_anchor_signs_a_checkpoint_over() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());
    anchor
        .append(
            Submission {
                statement: b"a statement, so the tree is not the trivial one",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    let status = anchor.status();
    let published = anchor.publish_checkpoint().unwrap();

    // The second party: the root that travelled through base64 into a signed
    // note, parsed back out by `lys-core`'s verifier under a key built from the
    // origin literal this file supplied.
    let verifier = NoteVerifierKey::new(ORIGIN, anchor.signer().public_key()).unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier).unwrap();

    assert_eq!(status.root.as_bytes(), body.root_hash());
    assert_eq!(status.tree_size(), body.tree_size());
}

#[test]
fn the_status_size_and_origin_come_from_the_log_and_not_from_this_process() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor(dir);
    for statement in [b"one".as_slice(), b"two".as_slice()] {
        anchor
            .append(Submission { statement }, SubmitterContext::Unidentified)
            .unwrap();
    }

    let status = anchor.status();

    // Against a handle that never saw the appends.
    let reopened = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    assert_eq!(status.tree_size(), reopened.tree().len());
    assert_eq!(status.tree_size(), 3);

    // Against the literal this file handed the store, not against
    // `anchor.origin()`.
    assert_eq!(status.origin, ORIGIN);

    // A clean open reports no repair, and the field is present rather than
    // absent so a consumer can tell "no repair" from "not reported".
    assert_eq!(status.recovered_to, None);
    assert_eq!(status.recovered_to, anchor.recovered_to());
}

#[test]
fn a_status_is_a_snapshot_and_does_not_follow_the_anchor() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    let before = anchor.status();
    anchor
        .append(
            Submission {
                statement: b"appended after the snapshot was taken",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    let after = anchor.status();

    assert_eq!(before.tree_size(), 1);
    assert_eq!(after.tree_size(), 2);
    assert_ne!(before.root, after.root);
}
