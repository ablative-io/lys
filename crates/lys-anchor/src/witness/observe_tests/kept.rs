#![cfg(test)]
//! A kept projection costs only the leaves recorded since the last call.

use super::*;

// ---------------------------------------------------------------------------
// rule: a kept projection costs only the leaves recorded since the last call
// ---------------------------------------------------------------------------

/// Submits `bytes` through the plain path, as a statement nobody witnesses.
fn submit(anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>, bytes: &[u8]) {
    anchor
        .submit(
            Submission { statement: bytes },
            SubmitterContext::Unidentified,
        )
        .unwrap();
}

/// Observes `note` with no proof through the kept `projection`, returning the
/// observation and how many parses it cost.
fn see_kept(
    anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>,
    projection: &mut WitnessProjection,
    note: &[u8],
) -> (crate::witness::report::Observation, u64) {
    let before = parse_count();
    let observed = observe(
        anchor,
        projection,
        note,
        None,
        SubmitterContext::Unidentified,
    )
    .unwrap();
    (observed, parse_count() - before)
}

/// Asserts that `projection` has folded the whole log and holds what a rebuild
/// over the same anchor holds for both child origins.
fn assert_matches_a_rebuild(
    anchor: &Anchor<FileLeafStore, FileSigner, AcceptAll>,
    projection: &WitnessProjection,
) {
    assert_eq!(projection.folded(), anchor.tree_size());
    let rebuilt = WitnessProjection::rebuild(anchor);
    for origin in [CHILD_ORIGIN, OTHER_CHILD_ORIGIN] {
        assert_eq!(
            projection.latest(origin),
            rebuilt.latest(origin),
            "{origin}"
        );
    }
}

#[test]
fn a_kept_projection_parses_only_what_was_recorded_since_the_last_observe() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    let other = Child::with_seed(OTHER_CHILD_ORIGIN, b"lys-anchor-increment-7-other-key");
    child.grow(1);
    assert_eq!(child.tree_size(), 2);
    submit(&mut anchor, &child.checkpoint());
    for n in 0..254 {
        submit(&mut anchor, format!("ordinary statement {n}").as_bytes());
    }
    assert_eq!(anchor.tree_size(), 256);

    let before = parse_count();
    let mut projection = WitnessProjection::rebuild(&anchor);
    assert_eq!(
        parse_count() - before,
        256,
        "positive control: the counter sees every parse of a rebuild"
    );

    child.grow(1);
    let (seen, parses) = see_kept(&mut anchor, &mut projection, &child.checkpoint());
    assert_eq!(parses, 1, "only the note itself");
    assert_eq!(seen.previous.unwrap().tree_size, 2);
    assert_matches_a_rebuild(&anchor, &projection);

    let (seen, parses) = see_kept(&mut anchor, &mut projection, &other.checkpoint());
    assert_eq!(parses, 1, "only the note itself");
    assert!(seen.previous.is_none());
    assert_matches_a_rebuild(&anchor, &projection);

    for n in 0..3 {
        submit(&mut anchor, format!("later statement {n}").as_bytes());
    }
    child.grow(1);
    let (seen, parses) = see_kept(&mut anchor, &mut projection, &child.checkpoint());
    assert_eq!(parses, 4, "the three statements since, and the note");
    assert_eq!(seen.previous.unwrap().tree_size, 3);
    assert_matches_a_rebuild(&anchor, &projection);
}

#[test]
fn a_kept_projection_follows_a_rollback() {
    let (_dir, mut anchor) = staged();
    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(4);
    assert_eq!(child.tree_size(), 5);
    let mut projection = WitnessProjection::default();

    let (first, first_parses) = see_kept(&mut anchor, &mut projection, &child.checkpoint());
    assert_eq!(first_parses, 2, "the genesis leaf, then the note");
    assert_ne!(
        first.relation,
        Some(Relation::Rollback),
        "positive control: a first sighting is not a rollback"
    );
    let rolled_back = child.checkpoint_stating(3, flip_root(child.root()));
    let (second, second_parses) = see_kept(&mut anchor, &mut projection, &rolled_back);
    assert_eq!(second.relation, Some(Relation::Rollback));
    assert_eq!(second_parses, 1, "only the note itself");
    assert_eq!(projection.latest(CHILD_ORIGIN).unwrap().tree_size, 3);
    assert_matches_a_rebuild(&anchor, &projection);
}

#[test]
fn a_kept_projection_folds_past_a_note_that_is_not_a_checkpoint() {
    let (_dir, mut anchor) = staged();
    let child = Child::new(CHILD_ORIGIN);
    submit(&mut anchor, &child.checkpoint());
    let mut projection = WitnessProjection::rebuild(&anchor);
    assert!(
        projection.latest(CHILD_ORIGIN).is_some(),
        "positive control: the projection holds a memory the statement could be compared to"
    );

    let (seen, parses) = see_kept(&mut anchor, &mut projection, b"not a checkpoint note");
    assert_eq!(parses, 1);
    assert!(seen.previous.is_none());
    assert_eq!(projection.folded(), anchor.tree_size());
}

#[test]
fn a_refused_observe_leaves_the_kept_projection_unchanged() {
    let dir = TempDir::new().unwrap();
    let key = dir.path().join("witness.key");
    std::fs::write(&key, b"lys-anchor-kept-projection-test!").unwrap();
    let store = FileLeafStore::create(dir.path(), WITNESS_ORIGIN).unwrap();
    let mut anchor = Anchor::create(
        store,
        WITNESS_GENESIS,
        FileSigner::load(&key).unwrap(),
        MaxSize::new(8),
        AnchorConfig::unconfigured(),
    )
    .unwrap();
    let note = Child::new(CHILD_ORIGIN).checkpoint();
    assert!(note.len() > 8, "the note must be longer than the limit");
    let short: &[u8] = b"short";
    anchor
        .submit(
            Submission { statement: short },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    let mut projection = WitnessProjection::default();
    projection.fold_to(&anchor, 1);
    let folded = projection.folded();
    assert_ne!(
        folded,
        anchor.tree_size(),
        "positive control: the projection is behind the log, so an unchanged position is not a fold that ran and stopped"
    );

    let refused = observe(
        &mut anchor,
        &mut projection,
        &note,
        None,
        SubmitterContext::Unidentified,
    );
    assert!(matches!(refused, Err(AnchorError::NotAdmitted)));
    assert_eq!(projection.folded(), folded);
}
