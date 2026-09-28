#![cfg(test)]
//! The strict open: it accepts an anchor this crate created and refuses a
//! genesis that is uninterpreted, from another root key, for another origin
//! or subject, for another role, or at another sequence.

use super::*;

// ---------------------------------------------------------------------------
// The open-time check. Everything above is about what CREATION writes; these
// are about what OPENING is willing to accept, which was nothing until now.
// ---------------------------------------------------------------------------

/// A second origin, for the case where leaf 0 is a valid delegation for a
/// different one.
const OTHER_ORIGIN: &str = "example.com/lys/genesis-delegation-test-2";

/// Signs `claim` with `signer` acting as the root key and returns the artifact.
///
/// Written through the same two-phase pair `genesis.rs` uses, so every forgery
/// below is a **cryptographically perfect** artifact: correct content type,
/// canonical encoding, a pair in `lys-core`'s table, and a signature that
/// verifies against the key in its own `kid`. Each refusal is therefore a
/// refusal of something valid rather than of something broken, which is the only
/// kind of refusal that says anything about the rule under test.
fn signed_delegation(signer: &FileSigner, claim: &DelegationClaim) -> Vec<u8> {
    let key = signer.public_key();
    let signature = signer.sign(&delegation_preimage(&key, claim)).unwrap();
    assemble_delegation(&key, claim, &signature).unwrap()
}

/// The claim `create_with_delegated_genesis` builds for `origin` and
/// `operational(dir)`.
///
/// Every forgery below starts from this and changes exactly one field, so the
/// field under test is the only difference between the artifact being refused
/// and one the strict open accepts.
fn genesis_claim(dir: &Path, origin: &str) -> DelegationClaim {
    DelegationClaim {
        subject_kind: DelegationSubjectKind::Domain,
        subject_value: origin.to_string(),
        delegated_public_key: operational(dir).public_key(),
        role: DelegationRole::Operational,
        not_before_unix_ms: NOT_BEFORE,
        sequence: GENESIS_SEQUENCE,
    }
}

/// Creates a store under `origin` holding `leaf` verbatim at index 0, through
/// the **uninterpreted** constructor.
///
/// That constructor is not a test convenience: it is the only route a leaf 0
/// which is not a genesis delegation can take into a store, and it is the route
/// every default-features build has.
fn create_with_raw_genesis(dir: &Path, origin: &str, leaf: &[u8]) {
    let store = FileLeafStore::create(dir, origin).unwrap();
    let anchor = Anchor::create(
        store,
        leaf,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap();
    assert_eq!(anchor.tree_size(), 1);
    drop(anchor);
}

/// Opens `dir` strictly, naming `root(dir)` as the expected root key.
fn open_strict(dir: &Path) -> AnchorResult<Anchor<FileLeafStore, FileSigner, AcceptAll>> {
    Anchor::open_verifying_genesis(
        FileLeafStore::open(dir).unwrap(),
        &root(dir).public_key(),
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
}

#[test]
fn the_strict_open_accepts_an_anchor_this_crate_created_and_it_is_still_a_working_anchor() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    drop(create_delegated(dir, ORIGIN).unwrap());

    let mut anchor = open_strict(dir).expect("a delegated genesis must survive a strict open");
    assert_eq!(anchor.origin(), ORIGIN);
    assert_eq!(anchor.tree_size(), 1);

    // The strict open returns an anchor, not a report. A constructor that
    // verified and then handed back something unusable would pass every refusal
    // case in this file.
    let appended = anchor
        .append(
            Submission {
                statement: b"appended through an anchor opened strictly",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    assert_eq!(appended.leaf_index, 1);
    let published = anchor.publish_checkpoint().unwrap();
    let verifier = NoteVerifierKey::new(ORIGIN, operational(dir).public_key()).unwrap();
    assert_eq!(
        verify_checkpoint(published.note.as_bytes(), &verifier)
            .unwrap()
            .tree_size(),
        2
    );
}

#[test]
fn an_uninterpreted_genesis_opens_under_open_and_is_refused_by_the_strict_open() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // Exactly what a default-features build creates: leaf 0 is whatever the
    // operator passed.
    create_with_raw_genesis(dir, ORIGIN, b"genesis");

    // ⛔ The gap this whole entry point exists for, asserted rather than
    // described. `open` accepts this store and the value it returns is
    // indistinguishable at every method from a DP16 anchor.
    let permissive = Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .expect("`open` reads no byte of leaf 0 and this store must still open");
    assert_eq!(permissive.tree_size(), 1);
    drop(permissive);

    // And the strict open is where it stops.
    assert!(matches!(
        open_strict(dir),
        Err(AnchorError::GenesisNotADelegation { ref origin, .. }) if origin == ORIGIN
    ));
}

#[test]
fn the_strict_open_refuses_a_genesis_delegation_from_a_root_key_the_caller_did_not_name() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    drop(create_delegated(dir, ORIGIN).unwrap());

    // Positive control: the same store, opened naming the key that signed it.
    drop(open_strict(dir).expect("the honest root key must be accepted"));

    let other_root = signer_from(dir, "other-root.key", OTHER_ROOT_SEED).public_key();
    assert_ne!(other_root, root(dir).public_key());
    assert!(matches!(
        Anchor::open_verifying_genesis(
            FileLeafStore::open(dir).unwrap(),
            &other_root,
            operational(dir),
            AcceptAll,
            AnchorConfig::unconfigured(),
        ),
        Err(AnchorError::GenesisNotADelegation { .. })
    ));
}

#[test]
fn the_strict_open_refuses_a_genesis_delegation_issued_for_a_different_origin() {
    let tmp = TempDir::new().unwrap();

    // One artifact, a perfectly valid delegation for OTHER_ORIGIN, signed by the
    // root key the opener names. The only thing wrong with it is which store it
    // is sitting in.
    let key_dir = tmp.path().join("keys");
    std::fs::create_dir(&key_dir).unwrap();
    let leaf = signed_delegation(&root(&key_dir), &genesis_claim(&key_dir, OTHER_ORIGIN));

    // ⭐ Positive control, and it is the whole point of this case: the SAME
    // bytes are accepted by a store whose origin is the one they name. So what
    // the refusal below measures is that the subject is compared against the
    // STORE's origin — a value the artifact has no say in — and not against the
    // artifact's own.
    let matching = tmp.path().join("matching");
    create_with_raw_genesis(&matching, OTHER_ORIGIN, &leaf);
    drop(
        Anchor::open_verifying_genesis(
            FileLeafStore::open(&matching).unwrap(),
            &root(&key_dir).public_key(),
            operational(&key_dir),
            AcceptAll,
            AnchorConfig::unconfigured(),
        )
        .expect("a delegation for this store's own origin must be accepted"),
    );

    let mismatched = tmp.path().join("mismatched");
    create_with_raw_genesis(&mismatched, ORIGIN, &leaf);
    assert!(matches!(
        Anchor::open_verifying_genesis(
            FileLeafStore::open(&mismatched).unwrap(),
            &root(&key_dir).public_key(),
            operational(&key_dir),
            AcceptAll,
            AnchorConfig::unconfigured(),
        ),
        Err(AnchorError::GenesisNotADelegation { ref origin, .. }) if origin == ORIGIN
    ));
}

#[test]
fn the_strict_open_refuses_a_seat_delegation_whose_identifier_is_this_stores_origin() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // The §3.3 cross-kind attack, at leaf 0: a valid `(seat, speaks-for)`
    // delegation, signed by the trusted root key, whose seat identifier is
    // literally this anchor's origin. A verifier that compared only the subject
    // VALUE would accept it. Nothing about it is malformed.
    let mut claim = genesis_claim(dir, ORIGIN);
    claim.subject_kind = DelegationSubjectKind::Seat;
    claim.role = DelegationRole::SpeaksFor;
    let leaf = signed_delegation(&root(dir), &claim);

    // Positive control on the forging path itself: these bytes ARE a valid
    // delegation, and `lys-core` says so when asked for the kind they carry.
    verify_delegation(
        &leaf,
        &root(dir).public_key(),
        DelegationSubjectKind::Seat,
        ORIGIN,
    )
    .expect("the forgery must be a valid seat delegation, or the refusal proves nothing");

    let store_dir = dir.join("anchor");
    create_with_raw_genesis(&store_dir, ORIGIN, &leaf);
    assert!(matches!(
        Anchor::open_verifying_genesis(
            FileLeafStore::open(&store_dir).unwrap(),
            &root(dir).public_key(),
            operational(dir),
            AcceptAll,
            AnchorConfig::unconfigured(),
        ),
        Err(AnchorError::GenesisNotADelegation { .. })
    ));
}

#[test]
fn the_strict_open_refuses_a_genesis_that_delegates_the_operational_role_to_the_root_key() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let mut claim = genesis_claim(dir, ORIGIN);
    claim.delegated_public_key = root(dir).public_key();
    let leaf = signed_delegation(&root(dir), &claim);

    // Positive control: `lys-core` accepts this artifact. The format permits
    // self-delegation, so the refusal below is DP16's rule and this crate's
    // alone — nothing between the bytes and here would flag it.
    verify_delegation(
        &leaf,
        &root(dir).public_key(),
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .expect("lys-core must accept a self-delegation, or this case tests the wrong layer");

    let store_dir = dir.join("anchor");
    create_with_raw_genesis(&store_dir, ORIGIN, &leaf);
    assert!(matches!(
        Anchor::open_verifying_genesis(
            FileLeafStore::open(&store_dir).unwrap(),
            &root(dir).public_key(),
            operational(dir),
            AcceptAll,
            AnchorConfig::unconfigured(),
        ),
        Err(AnchorError::GenesisDelegatesToTheRootKey { ref origin }) if origin == ORIGIN
    ));
}

#[test]
fn the_strict_open_refuses_a_genesis_whose_sequence_is_not_the_genesis_sequence() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // 1, not GENESIS_SEQUENCE + 1: read from the constant this would be the
    // constant agreeing with itself, and the convention is what is being pinned.
    let mut claim = genesis_claim(dir, ORIGIN);
    claim.sequence = 1;
    let leaf = signed_delegation(&root(dir), &claim);

    verify_delegation(
        &leaf,
        &root(dir).public_key(),
        DelegationSubjectKind::Domain,
        ORIGIN,
    )
    .expect("lys-core must accept any sequence: the format marks nothing as genesis");

    let store_dir = dir.join("anchor");
    create_with_raw_genesis(&store_dir, ORIGIN, &leaf);
    assert!(matches!(
        Anchor::open_verifying_genesis(
            FileLeafStore::open(&store_dir).unwrap(),
            &root(dir).public_key(),
            operational(dir),
            AcceptAll,
            AnchorConfig::unconfigured(),
        ),
        // The value is asserted as well as the variant: a message reporting what
        // was expected rather than what was found would pass on the variant
        // alone.
        Err(AnchorError::GenesisSequenceIsNotGenesis { sequence: 1, .. })
    ));
}

#[test]
fn the_check_runs_without_a_signer_and_hands_back_the_key_genesis_delegated() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    drop(create_delegated(dir, ORIGIN).unwrap());

    // The free function, reached with nothing but bytes, a key and an origin —
    // no signer, no admission policy, no write path. This is the shape a witness
    // or a read-only auditor has, and a check only reachable through a
    // constructor that demands a signing key would not be available to them.
    let delegation =
        verify_genesis_delegation(&leaf_zero_from_disk(dir), &root(dir).public_key(), ORIGIN)
            .expect("the check must be reachable without opening anything");

    assert_eq!(
        delegation.claim.delegated_public_key,
        operational(dir).public_key()
    );
    assert_ne!(
        delegation.claim.delegated_public_key,
        root(dir).public_key()
    );

    // Guarded by `lys-core`'s pair table rather than by this crate: `Domain`
    // permits only `Operational`, so the check does not re-test it and this
    // assertion is here to say where the rule lives, not to claim this crate
    // enforces it.
    assert_eq!(delegation.claim.role, DelegationRole::Operational);
}

#[test]
fn the_strict_open_does_not_require_the_opening_signer_to_be_the_delegated_key() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    drop(create_delegated(dir, ORIGIN).unwrap());

    // ⛔ This asserts an ABSENCE, deliberately, so that adding the obvious check
    // trips a test that says why it must not be added. Leaf 0 names the key
    // delegated at GENESIS; revocation is an append of a later, superseding
    // delegation, so after any rotation the operational key is not leaf 0's.
    // Comparing `signer.public_key()` against leaf 0 here would forbid rotation
    // while reading as a check that confirms the key, and deciding which
    // delegation is current needs a fold over the log that does not exist.
    let unrelated = signer_from(dir, "unrelated.key", OTHER_ROOT_SEED);
    assert_ne!(unrelated.public_key(), operational(dir).public_key());
    let anchor = Anchor::open_verifying_genesis(
        FileLeafStore::open(dir).unwrap(),
        &root(dir).public_key(),
        unrelated,
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .expect("the strict open must not pin the operational key to leaf 0's");
    assert_eq!(anchor.tree_size(), 1);
}
