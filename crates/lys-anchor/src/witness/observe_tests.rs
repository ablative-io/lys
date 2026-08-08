#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`observe`] — one rule per case, and **every negative case opens
//! with a positive control as its first assertion**.
//!
//! # The vacuity trap this file is shaped around, named
//!
//! This repo has already shipped and caught the defect these controls exist
//! for: drifting a Go content-type constant by one character left a whole
//! conformance test green, because every assertion in it said the tool
//! *refused* something — and a verifier that refuses everything satisfies all
//! of them at once. The witness equivalent is a `relate` that answers one
//! relation to everything: a suite made only of `assert_eq!(…, Conflicting)`
//! would be perfectly green against it.
//!
//! So each case below first asserts that the machinery **does not** answer its
//! target relation to an input that should not get it, and only then asserts
//! that the input built for it does. The controls are written as `assert_ne!`
//! against the case's own target rather than `assert_eq!` against some other
//! relation, deliberately: an `assert_eq!` control would be a second assertion
//! of *another* case's rule, and a drift of that rule would then fail two tests
//! instead of one.
//!
//! # One rule per case
//!
//! | rule | case |
//! |---|---|
//! | the record precedes the check | [`a_conflicting_checkpoint_is_still_appended`] |
//! | identical resubmission is a no-op | [`resubmitting_the_identical_checkpoint_is_identical`] |
//! | rollback observed | [`a_smaller_tree_size_is_rollback`] |
//! | equivocation observed | [`the_same_size_with_a_different_root_is_conflicting`] |
//! | consistency actually checked | [`a_forged_consistency_path_is_unrelated`] |
//! | no prior record implies nothing | [`a_first_sighting_reports_previous_none`] |
//! | the check never reaches the artifact | [`the_receipt_is_byte_identical_whatever_the_relation`] |
//!
//! [`a_conflicting_checkpoint_is_still_appended`] deliberately **does not**
//! assert the relation, and [`the_receipt_is_byte_identical_whatever_the_relation`]
//! deliberately does not either. Both would otherwise be second assertions of
//! rules that already have a case, and a rule with two guards is proven by
//! neither.
//!
//! # Where the second party comes from
//!
//! - **A real second log with its own key.** Every note observed here was
//!   produced by `lys-core`'s `sign_note` over a `CheckpointBody` built from a
//!   `Log`'s own root — including the equivocating and rolled-back ones, which
//!   are genuinely signed by the child. A witness cannot dismiss those as
//!   forgeries, which is exactly the case that matters.
//! - **`lys-core`'s consistency verifier**, which was written before this crate
//!   existed and does not know what a witness is.
//! - **A parallel anchor driven by the plain submit path**, for the byte
//!   identity claim: two independently driven logs, one witnessing and one not,
//!   must produce the same bytes.
//!
//! The independence axis is *implementation*, not platform: one machine, one
//! toolchain, one dependency resolution.

use lys_log_store::FileLeafStore;
use tempfile::TempDir;

use crate::admission::{AcceptAll, SubmitterContext};
use crate::anchor::Anchor;
use crate::keys::FileSigner;
use crate::wire::Submission;

use super::super::report::Relation;
use super::super::report::fixture::{
    CHILD_ORIGIN, Child, OTHER_CHILD_ORIGIN, flip_root, witness_anchor,
};
use super::*;

/// A witness anchor over a fresh temp directory, with the directory returned so
/// it outlives the anchor.
fn staged() -> (TempDir, Anchor<FileLeafStore, FileSigner, AcceptAll>) {
    let dir = TempDir::new().unwrap();
    let anchor = witness_anchor(dir.path());
    (dir, anchor)
}

/// Observes `note` with no consistency proof, under no established identity.
fn see(
    anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>,
    note: &[u8],
) -> super::super::report::Observation {
    observe(anchor, note, None, SubmitterContext::Unidentified).unwrap()
}

/// Observes `note` with `proof`.
fn see_with(
    anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>,
    note: &[u8],
    proof: &[u8],
) -> super::super::report::Observation {
    observe(anchor, note, Some(proof), SubmitterContext::Unidentified).unwrap()
}

// ---------------------------------------------------------------------------
// rule: the record precedes the check
// ---------------------------------------------------------------------------

#[test]
fn a_conflicting_checkpoint_is_still_appended() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(3);

    // Positive control, first: the instrument this case reads — tree size, and
    // the leaf read back off the anchor — actually moves when something is
    // recorded. A size that never changed would satisfy the claim below by
    // being wrong in both places at once.
    let before = anchor.tree_size();
    let seen = see(&mut anchor, &child.checkpoint());
    assert_eq!(
        anchor.tree_size(),
        before + 1,
        "positive control: recording does not move the tree size, so 'it was appended' is unmeasurable here"
    );
    assert_eq!(
        anchor.leaf_bytes(seen.recorded.leaf_index).unwrap(),
        child.checkpoint().as_slice()
    );

    // The claim: a checkpoint the witness's own memory contradicts is recorded
    // anyway. Same size, different root, and genuinely signed by the child.
    let equivocation = child.checkpoint_stating(child.tree_size(), flip_root(child.root()));
    let size_before = anchor.tree_size();
    let observed = see(&mut anchor, &equivocation);

    assert_eq!(
        anchor.tree_size(),
        size_before + 1,
        "the record must precede the check: an equivocating checkpoint is appended, not refused"
    );
    assert_eq!(
        anchor.leaf_bytes(observed.recorded.leaf_index).unwrap(),
        equivocation.as_slice(),
        "the leaf is the submitted note verbatim"
    );
    // A comparison did happen — so the append was not merely the check being
    // skipped — but which relation it produced is another case's rule and is
    // deliberately not asserted here.
    assert!(observed.previous.is_some());
    assert!(observed.relation.is_some());
}

// ---------------------------------------------------------------------------
// rule: identical resubmission is a no-op
// ---------------------------------------------------------------------------

#[test]
fn resubmitting_the_identical_checkpoint_is_identical() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(3);
    see(&mut anchor, &child.checkpoint());

    // Positive control, first: a checkpoint that is *not* a repeat does not
    // come back `Identical`, so `Identical` is not this path's only answer.
    child.grow(2);
    let grown = child.checkpoint();
    let proof = child.consistency_from(child.tree_size() - 2);
    let control = see_with(&mut anchor, &grown, &proof);
    assert_ne!(
        control.relation,
        Some(Relation::Identical),
        "positive control: every observation reports Identical, so the claim below is vacuous"
    );

    // The claim: the same bytes again, against a memory that now holds exactly
    // them, is a no-op rather than an error or a conflict.
    let repeat = see(&mut anchor, &grown);
    assert_eq!(repeat.relation, Some(Relation::Identical));
    let previous = repeat.previous.unwrap();
    assert_eq!(previous.tree_size, child.tree_size());
    assert_eq!(previous.root, child.root());
}

// ---------------------------------------------------------------------------
// rule: rollback observed
// ---------------------------------------------------------------------------

#[test]
fn a_smaller_tree_size_is_rollback() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(5);
    let remembered_size = child.tree_size();
    see(&mut anchor, &child.checkpoint());

    // Positive control, first: a *larger* size does not come back `Rollback`.
    child.grow(2);
    let proof = child.consistency_from(remembered_size);
    let control = see_with(&mut anchor, &child.checkpoint(), &proof);
    assert_ne!(
        control.relation,
        Some(Relation::Rollback),
        "positive control: every observation reports Rollback, so the claim below is vacuous"
    );

    // The claim: a genuinely signed checkpoint stating a smaller size than the
    // witness's own memory is reported as a rollback.
    let remembered_now = child.tree_size();
    let shrunk = child.checkpoint_stating(2, flip_root(child.root()));
    let observed = see(&mut anchor, &shrunk);
    assert_eq!(observed.relation, Some(Relation::Rollback));
    assert_eq!(observed.previous.unwrap().tree_size, remembered_now);
}

// ---------------------------------------------------------------------------
// rule: equivocation observed
// ---------------------------------------------------------------------------

#[test]
fn the_same_size_with_a_different_root_is_conflicting() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(4);
    let remembered_size = child.tree_size();
    see(&mut anchor, &child.checkpoint());

    // Positive control, first: an extending checkpoint does not come back
    // `Conflicting`, so `Conflicting` is not this path's only answer.
    child.grow(1);
    let proof = child.consistency_from(remembered_size);
    let control = see_with(&mut anchor, &child.checkpoint(), &proof);
    assert_ne!(
        control.relation,
        Some(Relation::Conflicting),
        "positive control: every observation reports Conflicting, so the claim below is vacuous"
    );

    // The claim: one size, two roots, both genuinely signed by the child — the
    // observation this whole mechanism exists to produce.
    let size = child.tree_size();
    let honest_root = child.root();
    let other_root = flip_root(honest_root);
    assert_ne!(honest_root, other_root);
    let equivocation = child.checkpoint_stating(size, other_root);
    let observed = see(&mut anchor, &equivocation);

    assert_eq!(observed.relation, Some(Relation::Conflicting));
    let previous = observed.previous.unwrap();
    assert_eq!(previous.tree_size, size, "same size");
    assert_eq!(previous.root, honest_root, "different root");
}

// ---------------------------------------------------------------------------
// rule: consistency actually checked
// ---------------------------------------------------------------------------

#[test]
fn a_forged_consistency_path_is_unrelated() {
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(3);
    let remembered = child.checkpoint();
    let remembered_size = child.tree_size();
    child.grow(4);
    let grown = child.checkpoint();
    let genuine = child.consistency_from(remembered_size);

    // A forgery, not a malformed blob: same length, still a whole number of
    // digests, so `ConsistencyProof::try_from_bytes` accepts it and the only
    // thing that can reject it is the verification itself.
    let mut forged = genuine.clone();
    forged[0] ^= 0xff;
    assert_eq!(forged.len(), genuine.len());
    assert_eq!(forged.len() % 32, 0);
    assert_ne!(forged, genuine);

    // Positive control, first, on its own witness: the genuine path does not
    // come back `Unrelated`, so `Unrelated` is not this path's only answer.
    let (_control_dir, mut control_anchor) = staged();
    see(&mut control_anchor, &remembered);
    let control = see_with(&mut control_anchor, &grown, &genuine);
    assert_ne!(
        control.relation,
        Some(Relation::Unrelated),
        "positive control: even a genuine consistency proof reports Unrelated, so the claim below is vacuous"
    );

    // The claim: the forged path is not accepted as an extension.
    let (_dir, mut anchor) = staged();
    see(&mut anchor, &remembered);
    let observed = see_with(&mut anchor, &grown, &forged);
    assert_eq!(observed.relation, Some(Relation::Unrelated));
    assert_eq!(observed.previous.unwrap().tree_size, remembered_size);
}

#[test]
fn a_size_increase_with_no_proof_offered_is_unrelated() {
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(3);
    let remembered = child.checkpoint();
    let remembered_size = child.tree_size();
    child.grow(4);
    let grown = child.checkpoint();
    let genuine = child.consistency_from(remembered_size);

    // Positive control, first: with a proof, the same growth does not come
    // back `Unrelated`.
    let (_control_dir, mut control_anchor) = staged();
    see(&mut control_anchor, &remembered);
    let control = see_with(&mut control_anchor, &grown, &genuine);
    assert_ne!(
        control.relation,
        Some(Relation::Unrelated),
        "positive control: every size increase reports Unrelated, so the claim below is vacuous"
    );

    // The claim: a witness offered no proof holds no proof, and says so.
    let (_dir, mut anchor) = staged();
    see(&mut anchor, &remembered);
    let observed = see(&mut anchor, &grown);
    assert_eq!(observed.relation, Some(Relation::Unrelated));
}

// ---------------------------------------------------------------------------
// rule: no prior record implies nothing
// ---------------------------------------------------------------------------

#[test]
fn a_first_sighting_reports_previous_none() {
    let (_dir, mut anchor) = staged();
    let mut known = Child::new(CHILD_ORIGIN);
    known.grow(2);

    let first = see(&mut anchor, &known.checkpoint());
    assert!(first.previous.is_none());
    assert!(first.relation.is_none());

    // Positive control, first — for the claim below: `previous` is not always
    // `None`, so a `None` means something. A second sighting of an origin this
    // witness has recorded reports what it recorded.
    known.grow(1);
    let second = see(&mut anchor, &known.checkpoint());
    assert!(
        second.previous.is_some(),
        "positive control: `previous` is always None, so the claim below is vacuous"
    );
    assert!(second.relation.is_some());

    // The claim: an origin this witness has never recorded gets no comparison
    // and no relation — not one borrowed from an origin it has recorded.
    let mut stranger = Child::with_seed(OTHER_CHILD_ORIGIN, b"lys-anchor-increment-7-other-key");
    stranger.grow(6);
    let unseen = see(&mut anchor, &stranger.checkpoint());
    assert!(
        unseen.previous.is_none(),
        "an origin never recorded must not be compared against another origin's memory"
    );
    assert!(unseen.relation.is_none());
    // It was still recorded: a witness with nothing to say still remembers.
    assert_eq!(
        anchor.leaf_bytes(unseen.recorded.leaf_index).unwrap(),
        stranger.checkpoint().as_slice()
    );
}

#[test]
fn a_leaf_that_is_not_a_checkpoint_is_recorded_and_compared_against_nothing() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(2);

    // Positive control, first: this path does produce a comparison for
    // something, so `None` below is the input being unreadable rather than the
    // comparison never running.
    see(&mut anchor, &child.checkpoint());
    let control = see(&mut anchor, &child.checkpoint());
    assert!(
        control.relation.is_some(),
        "positive control: no observation ever compares anything, so the claim below is vacuous"
    );

    let statement = b"this is not a checkpoint note";
    let observed = see(&mut anchor, statement);
    assert!(observed.previous.is_none());
    assert!(observed.relation.is_none());
    assert_eq!(
        anchor.leaf_bytes(observed.recorded.leaf_index).unwrap(),
        statement.as_slice(),
        "an unreadable submission is still recorded verbatim"
    );
}

// ---------------------------------------------------------------------------
// rule: the check never reaches the artifact
// ---------------------------------------------------------------------------

/// One step of the byte-identity script: the bytes offered, the consistency
/// proof offered with them, and the `(size, root)` memory *the test* computes
/// this step must be compared against.
type Step<'a> = (&'a [u8], Option<&'a [u8]>, Option<(u64, [u8; 32])>);

#[test]
fn the_receipt_is_byte_identical_whatever_the_relation() {
    // Two anchors from the same seed and the same genesis, fed the same bytes
    // in the same order. One is driven through the witness path, which computes
    // a relation at every step; the other through the plain submit path, which
    // has never heard of a relation. Every receipt must match byte-for-byte.
    //
    // The relations the seven inputs produce are asserted by the dedicated
    // cases above and are deliberately **not** re-asserted here: a relation
    // drift must fail exactly one case, and this one is about the artifact.
    let witness_dir = TempDir::new().unwrap();
    let plain_dir = TempDir::new().unwrap();
    let mut witness = witness_anchor(witness_dir.path());
    let mut plain = witness_anchor(plain_dir.path());

    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(3);
    let (size_one, root_one) = (child.tree_size(), child.root());
    let first = child.checkpoint();
    let other_root = flip_root(root_one);
    let equivocation = child.checkpoint_stating(size_one, other_root);
    let rollback_root = flip_root(other_root);
    let rolled_back = child.checkpoint_stating(2, rollback_root);
    child.grow(2);
    let (size_two, root_two) = (child.tree_size(), child.root());
    let grown = child.checkpoint();
    let mut forged = child.consistency_from(size_one);
    forged[0] ^= 0xff;
    child.grow(2);
    let grown_again = child.checkpoint();
    let genuine = child.consistency_from(size_two);

    // The script, and the memory each step must find. `expect_previous` is what
    // the *test* computes the witness should be holding — the stated
    // `(size, root)` of the last checkpoint the script recorded — so it is a
    // second party for the memory chain rather than a read-back of it.
    //
    // Coverage is pinned by these two columns and by nothing else. Held against
    // the stated `(size, root)` of each note, they say what each step is:
    // unreadable; a first sighting; a repeat of the memory; the same size with a
    // different root; a smaller size; a larger size with a proof that fails; a
    // larger size with a proof that verifies. **No relation is asserted here**,
    // deliberately — the five relation rules each have exactly one case above,
    // and re-asserting one of them here would leave its drift failing two tests
    // so that neither was the case that proved it. Writing the coverage as
    // memory state instead is what keeps this case blind to every relation
    // drift while still refusing to degrade silently: an earlier draft of this
    // script never reached the same-size-different-root shape at all, and no
    // assertion in it noticed.
    //
    // For a reader checking the coverage claim, the relations the seven shapes
    // below imply are, in order: none (unreadable), none (first sighting),
    // `Identical`, `Conflicting`, `Rollback`, `Unrelated`, `Extends`. They are
    // written here as a derivation to read, not as an assertion to run.
    let script: [Step<'_>; 7] = [
        (b"an ordinary statement", None, None),
        (&first, None, None),
        (&first, None, Some((size_one, root_one))),
        (&equivocation, None, Some((size_one, root_one))),
        (&rolled_back, None, Some((size_one, other_root))),
        (&grown, Some(&forged), Some((2, rollback_root))),
        (&grown_again, Some(&genuine), Some((size_two, root_two))),
    ];
    assert_ne!(root_one, other_root, "the equivocation must differ in root");
    assert_ne!(size_one, size_two, "the extension must differ in size");

    // Positive control, first: the comparison instrument can register a
    // difference. Two receipts from one anchor at two tree sizes must not be
    // byte-equal, or `assert_eq!` on receipt bytes below proves nothing.
    let control_a = plain
        .submit(
            Submission {
                statement: b"control leaf one",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    let control_b = plain
        .submit(
            Submission {
                statement: b"control leaf two",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    assert_ne!(
        control_a.receipt.to_cose_bytes(),
        control_b.receipt.to_cose_bytes(),
        "positive control: receipt bytes compare equal regardless, so the claim below is vacuous"
    );
    // Keep the two logs in step: the witness gets the same two control leaves.
    for statement in [
        b"control leaf one".as_slice(),
        b"control leaf two".as_slice(),
    ] {
        witness
            .submit(Submission { statement }, SubmitterContext::Unidentified)
            .unwrap();
    }

    let mut compared = 0;
    let mut with_a_memory = 0;
    for (step, (bytes, proof, expect_previous)) in script.into_iter().enumerate() {
        let witnessed =
            observe(&mut witness, bytes, proof, SubmitterContext::Unidentified).unwrap();
        let recorded = plain
            .submit(
                Submission { statement: bytes },
                SubmitterContext::Unidentified,
            )
            .unwrap();
        assert_eq!(
            witnessed.recorded.leaf_index, recorded.leaf_index,
            "the two logs must stay in step for the comparison to mean anything"
        );
        assert_eq!(witnessed.recorded.tree_size, recorded.tree_size);
        assert_eq!(
            witnessed.recorded.receipt.to_cose_bytes(),
            recorded.receipt.to_cose_bytes(),
            "a witness receipt differs from a plain submission's, so witnessing is encodable in the artifact"
        );
        // Coverage, not a relation: the memory this step was compared against.
        // Steps with no expectation are the two that must find none, and they
        // are deliberately left unasserted so this case stays blind to the
        // no-prior-record rule as well.
        if let Some(expected) = expect_previous {
            let previous = witnessed
                .previous
                .as_ref()
                .unwrap_or_else(|| panic!("step {step} must have found a memory"));
            assert_eq!(
                (previous.tree_size, previous.root),
                expected,
                "step {step} was compared against the wrong memory, so it is not the shape this script claims"
            );
            with_a_memory += 1;
        }
        compared += 1;
    }
    assert_eq!(compared, 7, "every scripted input must have been compared");
    assert_eq!(with_a_memory, 5, "five steps must have found a memory");
}
