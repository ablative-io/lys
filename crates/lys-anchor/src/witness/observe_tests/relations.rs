#![cfg(test)]
//! Each relation an observation can report: conflicting, identical,
//! rollback, unrelated, a first sighting, and a leaf that is not a
//! checkpoint.

use super::*;

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
