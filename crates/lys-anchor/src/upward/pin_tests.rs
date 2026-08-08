#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`pin()`] — one rule per case, and the DP14 claim asserted
//! structurally rather than by comparing outputs.
//!
//! # Why "the outcome type is the core's" is the assertion, and not "the two
//! paths agree"
//!
//! DP14 requires that cascading and witnessing be *the same mechanism*, and
//! §7.5 is specific about how that is to be checked: the test asserts the
//! outcome type is the core's, **not** that two paths produce equivalent
//! results. The difference is not pedantry. Two separate implementations that
//! happen to agree today satisfy an equivalence assertion for as long as they
//! keep agreeing, and they stop agreeing silently — that is the drift a
//! same-mechanism claim exists to rule out. A type identity cannot drift: if
//! `pin` ever grew a federation twin of [`SubmissionOutcome`],
//! [`the_outcome_is_the_cores_submission_outcome`] would fail to **compile**,
//! and a failure to compile is not a value that can be nearly right.
//!
//! # One rule per case
//!
//! | rule | the only case that may fail when it is drifted |
//! |---|---|
//! | the outcome is the core's type, not a twin | [`the_outcome_is_the_cores_submission_outcome`] |
//! | the leaf is the child's note verbatim — no new format | [`the_parent_records_the_child_note_verbatim`] |
//! | the checkpoint returned is the one that was submitted | [`the_returned_checkpoint_is_the_one_that_was_pinned`] |
//! | the parent cannot tell a cascade from a submission | [`the_parents_receipt_is_what_a_plain_submission_would_have_got`] |
//! | pinning appends nothing to the child | [`pinning_appends_nothing_to_the_childs_own_log`] |
//! | a parent's refusal records nothing anywhere | [`a_parent_that_refuses_leaves_both_logs_untouched`] |
//!
//! # Where the second party comes from
//!
//! - **`lys-core`'s `verify_checkpoint` and `verify_receipt_bytes`**, written
//!   before this crate existed and knowing nothing about a cascade, judge the
//!   two signed artifacts a pin produces.
//! - **A parallel parent driven by the plain submit path** is the second party
//!   for the "cannot tell" claim: two independently driven anchors, one reached
//!   through `pin` and one through `Anchor::submit`, must produce the same
//!   bytes.
//! - **The child's own storage**, read back through a value the fixture did not
//!   substitute, is the second party for "pinning appends nothing".
//!
//! The independence axis is *implementation*, not platform: one machine, one
//! toolchain, one dependency resolution.

use lys_core::checkpoint::verify_checkpoint;
use lys_core::receipt::verify_receipt_bytes;

use crate::admission::{MaxSize, SubmitterContext};
use crate::error::AnchorError;
use crate::wire::{Submission, SubmissionOutcome};

use super::fixture::{
    CHILD_ORIGIN, Node, PARENT_ORIGIN, PARENT_SEED, child, child_with_statement, parent,
};
use super::*;

#[test]
fn the_outcome_is_the_cores_submission_outcome() {
    let (child, _index) = child_with_statement();
    let mut parent = parent();

    let pinned = pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();
    let direct = parent
        .anchor
        .submit(
            Submission {
                statement: b"an ordinary statement from an ordinary submitter",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    // THE ASSERTION, and it is the type annotation rather than any `assert!`
    // below it: both values go into one `Vec<SubmissionOutcome>`. A federation
    // twin carrying the same fields would make this line fail to compile, which
    // is the one failure mode a diverging implementation cannot pass by
    // continuing to agree.
    let outcomes: Vec<SubmissionOutcome> = vec![pinned.recorded, direct];

    // Count what fired: an empty vector would satisfy any assertion phrased
    // over its contents, and this one is here to say two distinct values really
    // did arrive through the two paths.
    assert_eq!(outcomes.len(), 2);
    assert_ne!(
        outcomes[0].leaf_index, outcomes[1].leaf_index,
        "the two submissions must be two events at two indices"
    );
}

#[test]
fn the_parent_records_the_child_note_verbatim() {
    let (child, _index) = child_with_statement();
    let mut parent = parent();

    let pinned = pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();

    // Keyed on what the parent stored, read back out of the parent's log at the
    // index the parent reported — never on the value handed in.
    let stored = parent
        .anchor
        .leaf_bytes(pinned.recorded.leaf_index)
        .expect("the parent reported an index it stored a leaf at");
    assert_eq!(
        stored,
        pinned.checkpoint.note.as_bytes(),
        "the leaf must be the child's note verbatim: any wrapping would be a \
         new wire format, frozen by this very append"
    );

    // The second party: `lys-core`'s checkpoint verifier reads the stored bytes
    // as an ordinary C2SP note under the child's own key. A wrapped or
    // re-encoded leaf would not parse as one at all, so this is the claim "no
    // new format" made executable rather than asserted.
    let body = verify_checkpoint(stored, &child.verifier())
        .expect("the recorded leaf is an ordinary signed checkpoint note");
    assert_eq!(body.origin(), CHILD_ORIGIN);
    assert_eq!(body.tree_size(), child.anchor.tree_size());
}

#[test]
fn the_returned_checkpoint_is_the_one_that_was_pinned() {
    let (child, _index) = child_with_statement();
    let mut parent = parent();

    let pinned = pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();

    // The parent's receipt is over a root it derived from the leaf bytes it
    // was given, so a receipt that verifies against `checkpoint.note` is proof
    // the returned note is the submitted one. `verify_receipt_bytes` is
    // `lys-core`'s, and it is handed the parent's advertised key rather than
    // anything the pin reported.
    let receipt_bytes = pinned.recorded.receipt.to_cose_bytes();
    let verified = verify_receipt_bytes(
        &receipt_bytes,
        pinned.checkpoint.note.as_bytes(),
        &parent.verifier().public_key(),
    )
    .expect("the parent's receipt proves the returned note as its leaf");
    assert_eq!(verified.leaf_index, pinned.recorded.leaf_index);

    // And a note the child did not publish must not verify, so the check above
    // is not one a receipt over anything at all would pass.
    let mut tampered = pinned.checkpoint.note.as_bytes().to_vec();
    tampered[0] ^= 0xff;
    assert!(
        verify_receipt_bytes(&receipt_bytes, &tampered, &parent.verifier().public_key()).is_err(),
        "a receipt that verified against altered note bytes would prove nothing \
         about which note was pinned"
    );
}

#[test]
fn the_parents_receipt_is_what_a_plain_submission_would_have_got() {
    let (child, _index) = child_with_statement();
    let note = child.anchor.publish_checkpoint().unwrap().note;

    // Two parents, same origin, same key, same genesis, driven to the same tree
    // state by two different code paths.
    let mut pinned_parent = parent();
    let mut plain_parent = Node::new(PARENT_ORIGIN, PARENT_SEED);

    let via_pin = pin(
        &child.anchor,
        &mut pinned_parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();
    let via_submit = plain_parent
        .anchor
        .submit(
            Submission {
                statement: note.as_bytes(),
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    assert_eq!(
        via_pin.recorded.receipt.to_cose_bytes(),
        via_submit.receipt.to_cose_bytes(),
        "a parent's receipt must not distinguish a cascade from an ordinary \
         submission: there is no field for 'this came from an anchor' and there \
         must be no byte for it either"
    );
    assert_eq!(via_pin.recorded.leaf_index, via_submit.leaf_index);
    assert_eq!(via_pin.recorded.leaf_hash, via_submit.leaf_hash);
}

#[test]
fn pinning_appends_nothing_to_the_childs_own_log() {
    let (child, index) = child_with_statement();
    let mut parent = parent();

    let before_size = child.anchor.tree_size();
    let before_root = child.anchor.root();
    let before_leaf = child.anchor.leaf_bytes(index).unwrap().to_vec();

    let pinned = pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();

    assert_eq!(
        child.anchor.tree_size(),
        before_size,
        "publishing is not an append, and the parent's leaf lands in the \
         parent's log"
    );
    assert_eq!(child.anchor.root(), before_root);
    assert_eq!(child.anchor.leaf_bytes(index).unwrap(), before_leaf);

    // The positive control this negative needs: the *parent* did grow, so the
    // three assertions above are reporting an unchanged child rather than a
    // pin that quietly did nothing at all.
    assert_eq!(parent.anchor.tree_size(), 2);
    assert_eq!(pinned.recorded.leaf_index, 1);
}

#[test]
fn a_parent_that_refuses_leaves_both_logs_untouched() {
    let (child, _index) = child_with_statement();
    let note_len = child.anchor.publish_checkpoint().unwrap().note.len();

    // A parent whose policy admits nothing this large. The threshold is derived
    // from the note the child actually produces, so the refusal is keyed on the
    // other side's value rather than on a number chosen to fit.
    let mut strict = Node::with_policy(
        PARENT_ORIGIN,
        PARENT_SEED,
        MaxSize::new(note_len.saturating_sub(1)),
    );

    // Positive control first: this parent admits *something*, so the refusal
    // below is the size rule firing and not a policy that declines everything.
    let admitted = strict
        .anchor
        .submit(
            Submission {
                statement: b"short",
            },
            SubmitterContext::Unidentified,
        )
        .expect("the control submission is under the threshold and must be admitted");
    assert_eq!(admitted.leaf_index, 1);

    let child_size_before = child.anchor.tree_size();
    let parent_size_before = strict.anchor.tree_size();

    let refused = pin(
        &child.anchor,
        &mut strict.anchor,
        SubmitterContext::Unidentified,
    );
    assert!(
        matches!(refused, Err(AnchorError::NotAdmitted)),
        "a parent's refusal arrives as the one value carrying nothing, and a \
         cascade is not exempt from that"
    );
    assert_eq!(child.anchor.tree_size(), child_size_before);
    assert_eq!(strict.anchor.tree_size(), parent_size_before);
}

#[test]
fn a_bare_child_can_still_be_pinned() {
    // A child holding only genesis has a size-1 tree, which its *own*
    // `receipt_for` would refuse — and pinning it must still work, because the
    // artifact a pin needs from the child is a checkpoint, not a receipt.
    // Without this case the suite would only ever exercise children large
    // enough for both, and could not tell the two constraints apart.
    let child = child();
    let mut parent = parent();
    assert_eq!(child.anchor.tree_size(), 1);

    let pinned = pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();
    assert_eq!(pinned.checkpoint.body.tree_size(), 1);
    assert_eq!(
        parent
            .anchor
            .leaf_bytes(pinned.recorded.leaf_index)
            .unwrap(),
        pinned.checkpoint.note.as_bytes()
    );
}
