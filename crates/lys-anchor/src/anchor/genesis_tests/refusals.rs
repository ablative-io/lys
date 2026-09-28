#![cfg(test)]
//! Refusals when creating a delegated genesis, each without writing: a root
//! signer that is the operational signer, a log that already has leaves, a
//! root signer that declines, and one whose advertised key is not its own.

use super::*;

/// ⛔ The root key and the operational key may not be the same key, in either
/// order, and nothing is written when they are.
///
/// # Why this needs a test rather than a comment
///
/// The artifact this would produce is **valid**. Not malformed, not unsigned,
/// not mispaired — a correct `lys/delegation/v1` domain delegation that
/// `verify_delegation` accepts, saying the offline root key has delegated the
/// operational role to itself. DP16's two-key model would be void and **no
/// verifier anywhere could tell**, because there is nothing wrong with the
/// bytes. It would sit at leaf 0, which `LeafStore` can never correct.
///
/// So the only place the fault is visible is the constructor, and the only
/// evidence that it is visible is this test. Both orders are exercised because
/// "the root signer is also the operational signer" and "the operational signer
/// is also the root signer" are the same defect reached by two different
/// operator mistakes, and a check written against one variable would catch one.
#[test]
fn a_root_signer_that_is_also_the_operational_signer_is_refused_without_writing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // The control, in its own directory: two DISTINCT signers over the same
    // seeds are accepted, so the refusals below are about the collision and not
    // about this setup being unable to create an anchor.
    let control = TempDir::new().unwrap();
    assert!(create_delegated(control.path(), ORIGIN).is_ok());
    assert_ne!(
        root(dir).public_key(),
        operational(dir).public_key(),
        "the honest fixture must use two different keys, or the control is the \
         very case under test"
    );

    // Both orders: the operational key used as root, and the root key used as
    // operational. Each is a different operator mistake reaching one defect.
    let mut refused = 0;
    for (label, root_seed, operational_seed) in [
        (
            "the operational key used as the root key",
            OPERATIONAL_SEED,
            OPERATIONAL_SEED,
        ),
        (
            "the root key used as the operational key",
            ROOT_SEED,
            ROOT_SEED,
        ),
    ] {
        let case = TempDir::new().unwrap();
        let case_dir = case.path();
        let root_signer = signer_from(case_dir, "r.key", root_seed);
        let operational_signer = signer_from(case_dir, "o.key", operational_seed);
        assert_eq!(
            root_signer.public_key(),
            operational_signer.public_key(),
            "{label}: the fixture must actually collide"
        );

        let store = FileLeafStore::create(case_dir, ORIGIN).unwrap();
        match Anchor::create_with_delegated_genesis(
            store,
            &root_signer,
            NOT_BEFORE,
            operational_signer,
            AcceptAll,
            AnchorConfig::unconfigured(),
        ) {
            Err(AnchorError::Genesis(GenesisError::GenesisRootKeyIsOperationalKey { origin })) => {
                assert_eq!(origin, ORIGIN, "{label}");
            }
            other => panic!("{label}: expected GenesisRootKeyIsOperationalKey, got {other:?}"),
        }

        // Nothing was appended, so the store can still be given a correct
        // genesis — the same invariant the declining-signer case asserts, and
        // the reason the comparison happens before anything is built.
        assert_eq!(
            FileLeafStore::open(case_dir).unwrap().extent(),
            0,
            "{label}"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both orders must have been tried");

    // And the log left empty by a refusal still accepts an honest genesis, so
    // the refusal cost the operator nothing but the mistake.
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let anchor = Anchor::create_with_delegated_genesis(
        store,
        &root(dir),
        NOT_BEFORE,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .expect("two distinct signers must still be accepted");
    assert_eq!(anchor.tree_size(), 1);
    drop(anchor);
    let delegation = verify_delegation(
        &leaf_zero_from_disk(dir),
        &root(dir).public_key(),
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .unwrap();
    assert_ne!(
        delegation.claim.delegated_public_key,
        root(dir).public_key(),
        "an honest genesis delegates to a key that is NOT the root key"
    );
}

#[test]
fn delegated_genesis_over_a_log_that_already_has_leaves_is_refused_without_writing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Positive control: the first create is accepted, so the refusal below is
    // about the log's state and not about this constructor refusing everything.
    assert_eq!(create_delegated(dir, ORIGIN).unwrap().tree_size(), 1);

    // Recorded rather than re-asserted: what leaf 0 holds is another case's rule,
    // so a failure here is attributable to the refusal having written something.
    let before = leaf_zero_from_disk(dir);

    match Anchor::create_with_delegated_genesis(
        FileLeafStore::open(dir).unwrap(),
        &root(dir),
        NOT_BEFORE,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    ) {
        Err(AnchorError::Genesis(GenesisError::GenesisAlreadyWritten { origin, tree_size })) => {
            assert_eq!(origin, ORIGIN);
            assert_eq!(tree_size, 1);
        }
        other => panic!("expected GenesisAlreadyWritten over an occupied log, got {other:?}"),
    }

    let after = FileLeafStore::open(dir).unwrap();
    assert_eq!(after.extent(), 1);
    assert_eq!(after.leaf(0).unwrap().as_deref(), Some(before.as_slice()));
}

#[test]
fn a_root_signer_that_declines_leaves_a_log_that_can_still_be_given_genesis() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // The control: the same store, the same operational signer and a root signer
    // that does sign, in a separate directory — so "it failed" below cannot be
    // this setup being unable to create an anchor at all.
    let control = TempDir::new().unwrap();
    assert!(create_delegated(control.path(), ORIGIN).is_ok());

    let declining = DecliningSigner {
        public_key: root(dir).public_key(),
    };
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    match Anchor::create_with_delegated_genesis(
        store,
        &declining,
        NOT_BEFORE,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    ) {
        // Keyed on the sentinel this file put in the signer: the signer's own
        // error reached the caller, rather than being replaced.
        Err(AnchorError::Signing(SigningError::SignerDeclined { reason })) => {
            assert_eq!(reason, DECLINED_REASON);
        }
        other => panic!("expected the root signer's own error to propagate, got {other:?}"),
    }

    // The invariant `genesis`'s docs state: nothing was appended. Leaf 0 cannot
    // be replaced, so a declined signature must leave a log that can still be
    // given a genesis delegation.
    assert_eq!(FileLeafStore::open(dir).unwrap().extent(), 0);

    let anchor = Anchor::create_with_delegated_genesis(
        FileLeafStore::open(dir).unwrap(),
        &root(dir),
        NOT_BEFORE,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .expect("a log left empty by a declined signature must still accept genesis");
    assert_eq!(anchor.tree_size(), 1);
    drop(anchor);

    // And what it ended up with is a real delegation, so the retry produced an
    // anchor rather than merely a `tree_size` of 1.
    verify_delegation(
        &leaf_zero_from_disk(dir),
        &root(dir).public_key(),
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .unwrap();
}

#[test]
fn a_root_signer_whose_advertised_key_is_not_the_one_it_signs_with_is_refused_before_the_append() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // The control, again in its own directory: an honest signer over the same
    // seeds is accepted.
    let control = TempDir::new().unwrap();
    assert!(create_delegated(control.path(), ORIGIN).is_ok());

    // The one difference: `public_key` reports the root key while `sign` uses the
    // operational one. `Signer`'s contract forbids this; the point of the case is
    // that the contract is enforced rather than assumed.
    let signing = operational(dir);
    let advertised = root(dir).public_key();
    assert_ne!(signing.public_key(), advertised);
    let misadvertising = MisadvertisingSigner {
        signing,
        advertised,
    };

    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    match Anchor::create_with_delegated_genesis(
        store,
        &misadvertising,
        NOT_BEFORE,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    ) {
        Err(AnchorError::Genesis(GenesisError::GenesisDelegation { origin, source })) => {
            assert_eq!(origin, ORIGIN);
            assert!(
                matches!(source, TrustError::DelegationVerification),
                "a signature that does not match the advertised key is a verification failure, got {source:?}"
            );
        }
        other => panic!("expected GenesisDelegation for a mis-advertised root key, got {other:?}"),
    }

    // Nothing was appended, so the store can still be given a correct genesis.
    assert_eq!(FileLeafStore::open(dir).unwrap().extent(), 0);
}
