#![cfg(test)]
//! What a delegated genesis writes: leaf 0 is a delegation from the root key
//! to the operational key for the store's origin, it is written whatever the
//! admission policy, and the anchor still works.

use super::*;

#[test]
fn leaf_zero_is_a_delegation_from_the_root_key_to_the_operational_key_for_the_stores_origin() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let anchor = create_delegated(dir, ORIGIN).unwrap();
    assert_eq!(anchor.tree_size(), 1);
    drop(anchor);

    // The two keys must differ, or the delegated-key assertion below would hold
    // for an implementation that delegated to the wrong one.
    let root_key = root(dir).public_key();
    let operational_key = operational(dir).public_key();
    assert_ne!(root_key, operational_key);

    // `verify_delegation` is `lys-core`'s third-party entry point. Both of the
    // arguments it requires are values this file chose: the root signer's own
    // public key, and the origin literal handed to `FileLeafStore::create`.
    let delegation = verify_delegation(
        &leaf_zero_from_disk(dir),
        &root_key,
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .expect("leaf 0 must verify as a delegation from the root key for the store's origin");

    assert_eq!(delegation.root_public_key, root_key);
    assert_eq!(delegation.claim.delegated_public_key, operational_key);
    assert_ne!(delegation.claim.delegated_public_key, root_key);
    assert_eq!(delegation.claim.role, DelegationRole::Operational);
    assert_eq!(delegation.claim.not_before_unix_ms, NOT_BEFORE);

    // The genesis sequence, as a literal. Read from `GENESIS_SEQUENCE` it would
    // be the constant agreeing with itself, and the convention is what is being
    // pinned.
    assert_eq!(delegation.claim.sequence, 0);

    // And the origin arrived in the signed payload as the subject VALUE, not
    // merely as the argument that was compared against it.
    assert_eq!(delegation.claim.subject_value, ORIGIN);

    // ⛔ The subject KIND. An anchor's genesis must be a DOMAIN delegation, and
    // this is the one leaf that can never be corrected — so a constructor that
    // wrote a seat here would produce a perfectly signed, internally valid
    // artifact (`(seat, speaks-for)` is in `lys-core`'s pair table) that no
    // verifier would flag. Asserted as the enum variant this file names, not read
    // back from whatever the constructor chose.
    assert_eq!(delegation.claim.subject_kind, DelegationSubjectKind::Domain);

    // And the seat verifier refuses it, over the same subject string — so the kind
    // is doing work here rather than merely being present. `verify_delegation`
    // requires the kind as an argument precisely because a value-only check would
    // accept a seat delegation whose identifier equalled this origin.
    assert!(
        verify_delegation(
            &leaf_zero_from_disk(dir),
            &root_key,
            DelegationSubjectKind::Seat,
            ORIGIN
        )
        .is_err(),
        "an anchor's genesis must not verify as a seat delegation"
    );
}

#[test]
fn an_anchor_created_under_one_root_key_does_not_verify_against_another() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_delegated(dir, ORIGIN).unwrap();
    let leaf = leaf_zero_from_disk(dir);

    let root_key = root(dir).public_key();
    let other_root_key = signer_from(dir, "other-root.key", OTHER_ROOT_SEED).public_key();
    assert_ne!(root_key, other_root_key);

    // Positive control: the instrument accepts the key that actually signed.
    assert!(verify_delegation(&leaf, &root_key, DelegationSubjectKind::Domain, ORIGIN).is_ok());

    // The one difference: a different root key is named.
    assert!(
        verify_delegation(
            &leaf,
            &other_root_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_err(),
        "a genesis delegation must not verify against a root key that did not sign it"
    );

    // And for the same reason the format's `kid` is a claim rather than an
    // authority: the artifact still *names* the key that signed it, so an
    // implementation that had written the wrong key into `kid` would have failed
    // the positive control above rather than this line.
    assert_eq!(
        verify_delegation(&leaf, &root_key, DelegationSubjectKind::Domain, ORIGIN)
            .unwrap()
            .root_public_key,
        root_key
    );
}

#[test]
fn the_delegations_origin_is_the_stores_and_two_stores_produce_two_delegations() {
    // Two origins, so a constant or a fallback would have to be right for both
    // inputs at once.
    let origins = [
        "example.com/lys/genesis-alpha",
        "somewhere.else.invalid/a/different/log",
    ];
    assert_ne!(origins[0], origins[1]);

    let mut artifacts = Vec::new();
    for origin in origins {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        create_delegated(dir, origin).unwrap();
        let leaf = leaf_zero_from_disk(dir);
        let root_key = root(dir).public_key();

        // Positive control: it verifies under its own store's origin.
        let delegation = verify_delegation(&leaf, &root_key, DelegationSubjectKind::Domain, origin)
            .expect("a delegation must verify under the origin its store was created with");
        assert_eq!(delegation.claim.subject_value, origin);

        artifacts.push((leaf, root_key));
    }

    // Count what fired: a loop that ran once satisfies every assertion above
    // without ever comparing two origins.
    assert_eq!(artifacts.len(), origins.len());

    // The two artifacts differ, and each is refused under the other's origin.
    // Both keys are identical across the two anchors — the seeds are fixed — so
    // the origin is the only thing that can be making the difference.
    assert_eq!(artifacts[0].1, artifacts[1].1);
    assert_ne!(artifacts[0].0, artifacts[1].0);
    assert!(
        verify_delegation(
            &artifacts[0].0,
            &artifacts[0].1,
            DelegationSubjectKind::Domain,
            origins[1]
        )
        .is_err(),
        "a delegation for one origin must not verify for another"
    );
    assert!(
        verify_delegation(
            &artifacts[1].0,
            &artifacts[1].1,
            DelegationSubjectKind::Domain,
            origins[0]
        )
        .is_err(),
        "a delegation for one origin must not verify for another"
    );
}

#[test]
fn a_delegated_genesis_is_written_even_under_a_policy_that_would_refuse_it() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // The control that makes this mean anything: this policy really does refuse
    // everything, so the assertion below would not pass for a permissive one.
    let refusing = MaxSize::new(0);
    assert_eq!(
        refusing.admit(
            &Submission {
                statement: b"anything at all"
            },
            &SubmitterContext::Unidentified
        ),
        Err(NotAdmitted),
        "the fixture policy must refuse"
    );

    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let anchor = Anchor::create_with_delegated_genesis(
        store,
        &root(dir),
        NOT_BEFORE,
        operational(dir),
        refusing,
        AnchorConfig::unconfigured(),
    )
    .expect("creating an anchor must not consult its admission policy");
    assert_eq!(anchor.tree_size(), 1);
    drop(anchor);

    // A delegation is far larger than the policy's zero-byte limit, so this also
    // establishes that genesis did not slip past the policy by being small.
    let leaf = leaf_zero_from_disk(dir);
    assert!(leaf.len() > 100);
    verify_delegation(
        &leaf,
        &root(dir).public_key(),
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .unwrap();
}

#[test]
fn an_anchor_whose_leaf_zero_is_a_delegation_is_still_a_working_anchor() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Leaf 0 is now a ~180-byte COSE artifact where it used to be whatever the
    // caller passed. Nothing in the log cares — a leaf is bytes — but "the
    // delegation parses" is not the same claim as "the anchor works", and only
    // the first of those is asserted anywhere else in this file. Without this
    // case, genesis-as-delegation could have broken publishing and every test
    // above would still be green.
    let mut anchor = create_delegated(dir, ORIGIN).unwrap();

    let statement: &[u8] = b"the first statement an anchor with a delegated genesis admitted";
    let appended = anchor
        .append(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap();
    // Keyed on the index this file can count, not on one the anchor volunteered.
    assert_eq!(appended.leaf_index, 1);
    assert_eq!(anchor.tree_size(), 2);

    // A checkpoint, verified as a stranger holding only the origin literal this
    // file supplied and the operational key the delegation names.
    let published = anchor.publish_checkpoint().unwrap();
    let verifier = NoteVerifierKey::new(ORIGIN, operational(dir).public_key()).unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier).unwrap();
    assert_eq!(body.origin(), ORIGIN);
    assert_eq!(body.tree_size(), 2);

    // ⭐ The join that makes the two-key model checkable end to end: the key the
    // ROOT key delegated to in leaf 0 is the key that signed this checkpoint.
    // The verifier above was built from `operational(dir)`, and the delegation is
    // read back off disk, so the two values reach this line by different routes.
    let delegation = verify_delegation(
        &leaf_zero_from_disk(dir),
        &root(dir).public_key(),
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .unwrap();
    assert_eq!(
        delegation.claim.delegated_public_key,
        operational(dir).public_key(),
        "the checkpoint verified under the very key leaf 0 delegates to"
    );
}
