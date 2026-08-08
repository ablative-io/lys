#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`WitnessProjection`] and [`checkpoint_in_leaf`].
//!
//! # Where the second party comes from
//!
//! - **`lys-core`'s `verify_note`.** The body split here is this crate's own
//!   reading of the C2SP signed-note structure, and a reader that only ever
//!   consumes bytes this crate produced would agree with itself forever.
//!   `verify_note` *returns the body it verified*, and it was written before
//!   this crate existed by a party that does not know how the split here is
//!   implemented. [`the_split_agrees_with_the_body_lys_core_verified`] holds
//!   the two against each other, on notes signed by a key this module never
//!   sees.
//! - **The disk, twice.** [`a_projection_is_rebuilt_from_leaves_not_remembered`]
//!   reopens the *witness's* store through a fresh handle that never saw an
//!   append and requires the same projection out of it;
//!   [`only_leaves_that_read_as_checkpoints_enter_the_projection`] reopens the
//!   *child's* store and checks the remembered `(size, root)` against the tree
//!   rebuilt from those bytes, rather than against the in-memory tree that
//!   produced the note.
//! - **A second log with its own key**, so "this origin" can be told from "any
//!   origin".

use lys_core::checkpoint::{NoteVerifierKey, verify_note};
use lys_log_store::{FileLeafStore, Log};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{AcceptAll, SubmitterContext};
use crate::anchor::Anchor;
use crate::keys::FileSigner;
use crate::wire::Submission;

use super::super::report::fixture::{
    CHILD_ORIGIN, Child, OTHER_CHILD_ORIGIN, flip_root, witness_anchor,
};
use super::*;

/// Appends `bytes` to `anchor` as an ordinary submission.
fn record(anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>, bytes: &[u8]) {
    anchor
        .submit(
            Submission { statement: bytes },
            SubmitterContext::Unidentified,
        )
        .unwrap();
}

#[test]
fn the_split_agrees_with_the_body_lys_core_verified() {
    let child = Child::new(CHILD_ORIGIN);
    let note = child.checkpoint();
    let verifier = NoteVerifierKey::new(child.origin(), child.public_key()).unwrap();

    // Positive control, first: the comparison instrument can register a
    // difference. Two different notes must not compare equal, or "the split
    // agrees" would be satisfied by any pair of strings.
    let other = Child::with_seed(OTHER_CHILD_ORIGIN, b"lys-anchor-increment-7-other-key");
    let other_verifier = NoteVerifierKey::new(other.origin(), other.public_key()).unwrap();
    let other_body = verify_note(&other.checkpoint(), &other_verifier).unwrap();
    let ours = checkpoint_in_leaf(&note).unwrap().encode();
    assert_ne!(
        ours, other_body,
        "positive control: the comparison cannot tell two different bodies apart"
    );

    // The claim: what this module split out is byte-for-byte what `lys-core`
    // verified a signature over.
    let body_lys_core_signed_over = verify_note(&note, &verifier).unwrap();
    assert_eq!(ours, body_lys_core_signed_over);
}

#[test]
fn a_leaf_that_is_not_a_checkpoint_note_is_not_read_as_one() {
    // Positive control: the reader does return `Some` for something.
    let child = Child::new(CHILD_ORIGIN);
    assert!(
        checkpoint_in_leaf(&child.checkpoint()).is_some(),
        "positive control: the reader never reads anything, so its `None`s prove nothing"
    );

    let mut refused = 0;
    // A bare statement; a checkpoint *body* with no signature block, so no
    // blank line; a note whose body is not a checkpoint; and invalid UTF-8.
    let not_notes: [&[u8]; 4] = [
        b"an ordinary statement",
        b"example.com/lys/child-log-test\n2\nAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
        b"not a checkpoint body\n\n\xe2\x80\x94 name AAAA\n",
        &[0xff, 0xfe, 0x0a, 0x0a, 0xfd],
    ];
    for candidate in not_notes {
        assert!(checkpoint_in_leaf(candidate).is_none());
        refused += 1;
    }
    assert_eq!(refused, 4, "every case must have been reached");
}

#[test]
fn only_leaves_that_read_as_checkpoints_enter_the_projection() {
    let dir = TempDir::new().unwrap();
    let mut anchor = witness_anchor(dir.path());
    let child = Child::new(CHILD_ORIGIN);

    record(&mut anchor, b"an ordinary statement");
    record(&mut anchor, b"another ordinary statement");
    assert!(
        WitnessProjection::rebuild(&anchor).is_empty(),
        "statements that are not checkpoints must leave the memory empty"
    );

    record(&mut anchor, &child.checkpoint());
    let projection = WitnessProjection::rebuild(&anchor);
    assert_eq!(projection.len(), 1);
    assert_eq!(projection.origins().collect::<Vec<_>>(), vec![CHILD_ORIGIN]);
    // Keyed on what the child supplied, and checked against the child's log
    // reopened from disk through a handle that never saw an append — not
    // against the in-memory tree that produced the note, and never against a
    // value this crate substituted.
    let held = projection.latest(CHILD_ORIGIN).unwrap();
    let child_from_disk = Log::open(FileLeafStore::open(child.dir()).unwrap()).unwrap();
    let (root_from_disk, size_from_disk) = child_from_disk.tree().root().to_parts();
    assert_eq!(held.tree_size, size_from_disk);
    assert_eq!(held.root, root_from_disk);
    // That the memory is *keyed* by origin is a separate rule, and
    // [`two_origins_are_two_memories`] is its only case — asserting it here as
    // well would leave a drift of it failing two tests, so neither would be the
    // one that proved it.
}

#[test]
fn the_last_recorded_wins_and_it_is_not_the_largest() {
    let dir = TempDir::new().unwrap();
    let mut anchor = witness_anchor(dir.path());
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(4);

    let large_size = child.tree_size();
    let large_root = child.root();
    record(&mut anchor, &child.checkpoint());
    assert_eq!(
        WitnessProjection::rebuild(&anchor)
            .latest(CHILD_ORIGIN)
            .unwrap()
            .tree_size,
        large_size,
        "positive control: the memory holds what was recorded before the rollback"
    );

    // A smaller, genuinely signed checkpoint recorded afterwards.
    let small_root = flip_root(large_root);
    record(&mut anchor, &child.checkpoint_stating(2, small_root));

    let held = WitnessProjection::rebuild(&anchor)
        .latest(CHILD_ORIGIN)
        .unwrap()
        .clone();
    assert_eq!(held.tree_size, 2, "the last recorded wins, not the largest");
    assert_eq!(held.root, small_root);
}

#[test]
fn a_prefix_fold_stops_below_the_bound() {
    let dir = TempDir::new().unwrap();
    let mut anchor = witness_anchor(dir.path());
    let mut child = Child::new(CHILD_ORIGIN);

    record(&mut anchor, &child.checkpoint());
    let first_size = child.tree_size();
    child.grow(3);
    record(&mut anchor, &child.checkpoint());
    let second_index = anchor.tree_size() - 1;

    // Positive control: the whole-log fold does see the later checkpoint, so a
    // prefix that does not see it is a bound taking effect and not a fold that
    // never worked.
    assert_eq!(
        WitnessProjection::rebuild(&anchor)
            .latest(CHILD_ORIGIN)
            .unwrap()
            .tree_size,
        child.tree_size()
    );

    let before = WitnessProjection::rebuild_prefix(&anchor, second_index);
    assert_eq!(before.latest(CHILD_ORIGIN).unwrap().tree_size, first_size);

    // The bound is exclusive at both ends of its range: zero leaves is empty.
    assert!(WitnessProjection::rebuild_prefix(&anchor, 0).is_empty());
    // And a bound past the end is the log, not an error.
    assert_eq!(
        WitnessProjection::rebuild_prefix(&anchor, u64::MAX)
            .latest(CHILD_ORIGIN)
            .unwrap()
            .tree_size,
        child.tree_size()
    );
}

#[test]
fn a_projection_is_rebuilt_from_leaves_not_remembered() {
    let dir = TempDir::new().unwrap();
    let recorded = {
        let mut anchor = witness_anchor(dir.path());
        let mut child = Child::new(CHILD_ORIGIN);
        child.grow(2);
        record(&mut anchor, &child.checkpoint());
        WitnessProjection::rebuild(&anchor)
            .latest(CHILD_ORIGIN)
            .unwrap()
            .clone()
    };

    // A handle that never saw the append, over the same directory.
    let store = FileLeafStore::open(dir.path()).unwrap();
    let signer = FileSigner::load(&dir.path().join("witness.key")).unwrap();
    let reopened = Anchor::open(store, signer, AcceptAll, AnchorConfig::unconfigured()).unwrap();

    let rebuilt = WitnessProjection::rebuild(&reopened);
    assert_eq!(rebuilt.len(), 1);
    assert_eq!(rebuilt.latest(CHILD_ORIGIN).unwrap(), &recorded);
}

#[test]
fn two_origins_are_two_memories() {
    let dir = TempDir::new().unwrap();
    let mut anchor = witness_anchor(dir.path());
    let mut first = Child::new(CHILD_ORIGIN);
    let mut second = Child::with_seed(OTHER_CHILD_ORIGIN, b"lys-anchor-increment-7-other-key");
    first.grow(1);
    second.grow(5);

    record(&mut anchor, &first.checkpoint());
    record(&mut anchor, &second.checkpoint());

    let projection = WitnessProjection::rebuild(&anchor);
    assert_eq!(projection.len(), 2);
    assert_eq!(
        projection.latest(CHILD_ORIGIN).unwrap().tree_size,
        first.tree_size()
    );
    assert_eq!(
        projection.latest(OTHER_CHILD_ORIGIN).unwrap().tree_size,
        second.tree_size()
    );
    assert_ne!(first.tree_size(), second.tree_size());
}
