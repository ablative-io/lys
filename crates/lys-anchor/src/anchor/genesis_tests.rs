#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on genesis-as-delegation.
//!
//! # What is deliberately NOT tested here
//!
//! **The delegation format.** `lys-core` has a golden vector held as a literal, a
//! `go-cose` gate, an independent encoder and 64 tests over the encoding. A
//! `lys-anchor` test asserting the protected header, the head widths or the
//! `Sig_structure` again is one party agreeing with itself through an extra
//! import. Nothing below inspects a byte of the artifact: leaf 0 is handed to
//! `lys-core`'s own third-party entry point,
//! [`verify_delegation`](lys_core::delegation::verify_delegation), and what is
//! asserted is what *this file* supplied to the store and to the signers.
//!
//! # Where the second party comes from
//!
//! - **`DELEGATION-V1.md` §3.2 and §3.3, as prose.** `verify_delegation` takes
//!   the expected root key, the expected subject **kind** and the expected subject
//!   **value** as required arguments *because* an artifact verifies against
//!   whatever key it carries, for whatever subject it names, under whatever kind it
//!   claims. This file uses that requirement as the instrument: the root key it
//!   names is the one the **root signer** reported, the value it names is the
//!   literal this file handed `FileLeafStore::create`, and the kind it names is
//!   `Domain` because §5 makes that normative for genesis. A `create` that put its
//!   own operational key in `kid`, a constant origin in the payload, or a `Seat`
//!   kind in the one leaf that can never be corrected, fails against arguments it
//!   never got to choose.
//! - **The signers' own public keys.** `delegated_public_key` is checked against
//!   `operational.public_key()` — the value the signer computed from its seed —
//!   and against `root.public_key()` for *inequality*, over two distinct seeds,
//!   so a path that delegated to the root key by mistake cannot pass by the two
//!   keys happening to coincide.
//! - **The bytes on disk.** Leaf 0 is read back through a *fresh*
//!   [`FileLeafStore`] handle that never saw the append, so an anchor that
//!   returned what it was handed rather than what it wrote would fail.
//! - **`genesis`'s stated ordering invariant.** The module docs say nothing is
//!   appended unless the delegation was produced and verified. Two cases below
//!   inject a signer that fails in each of the two ways that can happen and
//!   assert the store's extent, which is a fact the store reports and this crate
//!   cannot substitute.
//!
//! # Every refusal opens with a positive control
//!
//! A suite made only of refusals cannot tell a working verifier from one that
//! rejects everything. Each negative case below first asserts that the
//! unmodified setup is *accepted*, then introduces the one difference under test.
//!
//! # The drift injections that were run, and what they measured
//!
//! Each was applied to `genesis.rs` alone, with the suite unchanged. Four
//! isolated to exactly one test:
//!
//! | injection | failed |
//! |---|---|
//! | `sequence` becomes `GENESIS_SEQUENCE + 1` | `leaf_zero_is_a_delegation…` only |
//! | `delegated_public_key` becomes the **root** key | `leaf_zero_is_a_delegation…` only |
//! | `not_before_unix_ms` is off by one | `leaf_zero_is_a_delegation…` only |
//! | the origin becomes a committed constant | `the_delegations_origin_is_the_stores…` **only** |
//! | `subject_kind` becomes `Seat` | `leaf_zero_is_a_delegation…` only (the pair moves with it, so the artifact stays valid — which is exactly why this needs asserting) |
//! | the log is appended to on the signing-failure path | `a_root_signer_that_declines…` only |
//!
//! ⚠️ **The origin row is the one worth reading twice, and it is why the
//! two-origin case exists.** The constant injected was
//! `"example.com/lys/genesis-delegation-test"` — this file's own [`ORIGIN`] — so
//! `leaf_zero_is_a_delegation…` stayed **green**: its store's origin happened to
//! equal the hardcoded one. A committed origin constant is invisible to any test
//! whose fixture origin matches it, which is precisely the constant most likely
//! to be written. Only a case that creates two stores under two origins and
//! counts that both ran can catch it, and DP15's whole content is that no such
//! constant may exist.
//!
//! Two injections were **not** isolated, and both are honest failures of the
//! injection rather than of the suite: naming the operational key in the
//! protected `kid`, and appending a placeholder before signing, each break the
//! construction outright, so every case's positive control fails. A defect that
//! large is not the one an isolating test is for.
//!
//! **One case exists because of what the others could not fail on.** Every case
//! above asserts that leaf 0 *parses and verifies*; none of them would notice an
//! anchor that could no longer append or publish. Leaf 0 went from whatever the
//! caller passed to a ~180-byte COSE artifact, so
//! `an_anchor_whose_leaf_zero_is_a_delegation_is_still_a_working_anchor` appends,
//! publishes, and checks that the key the **checkpoint** verified under is the key
//! the **delegation** confers — the one assertion in this file that joins the two
//! halves of DP16's two-key model, reached by two independent routes.
//!
//! One rule below is **guarded by `lys-core` rather than by this file**, and it is
//! said plainly rather than claimed:
//! `a_root_signer_whose_advertised_key_is_not_the_one_it_signs_with…` fails
//! because `assemble_delegation` verifies before returning, which `lys-core`
//! already tests. What this file adds is that `genesis.rs` reaches the artifact
//! through the *verifying* entry point and not around it — so the test pins a
//! routing decision, and no injection confined to `genesis.rs` can make it fail
//! while any other route to an artifact does not exist.

use std::path::Path;

use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_core::delegation::{DelegationRole, DelegationSubjectKind, verify_delegation};
use lys_core::error::TrustError;
use lys_log_store::{FileLeafStore, LeafStore};
use tempfile::TempDir;

use crate::admission::{AcceptAll, AdmissionPolicy, MaxSize, NotAdmitted, SubmitterContext};
use crate::keys::{FileSigner, Signer};
use crate::wire::Submission;

use super::*;

const ORIGIN: &str = "example.com/lys/genesis-delegation-test";

/// An arbitrary but fixed effectivity claim, chosen so a path that substituted a
/// clock reading, a zero, or `sequence` would produce a different number. It is
/// not `0` (indistinguishable from a field nobody wrote), not `1` (which is
/// `role`'s wire value), and needs an 8-byte CBOR head.
const NOT_BEFORE: u64 = 1_700_000_000_000;

/// The operational key's seed — `lys-core`'s conformance fixture seed, so key
/// material is deterministic and never generated.
const OPERATIONAL_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// The root key's seed. **Distinct from the operational one**, which is what
/// makes `delegated_public_key != root_public_key` a real assertion rather than a
/// tautology.
const ROOT_SEED: &[u8; 32] = b"lys-anchor-genesis-root-seed-01!";

/// A second root seed, for "root key A does not verify against root key B".
const OTHER_ROOT_SEED: &[u8; 32] = b"lys-anchor-genesis-root-seed-02!";

/// Writes `seed` to `dir/name` and loads a [`FileSigner`] over it.
fn signer_from(dir: &Path, name: &str, seed: &[u8; 32]) -> FileSigner {
    let path = dir.join(name);
    std::fs::write(&path, seed).unwrap();
    FileSigner::load(&path).unwrap()
}

/// The anchor's operational signer.
fn operational(dir: &Path) -> FileSigner {
    signer_from(dir, "anchor.key", OPERATIONAL_SEED)
}

/// The anchor's root signer.
fn root(dir: &Path) -> FileSigner {
    signer_from(dir, "root.key", ROOT_SEED)
}

/// Creates a store under `origin` and an anchor over it whose leaf 0 is a
/// delegation from `root(dir)` to `operational(dir)`.
fn create_delegated(
    dir: &Path,
    origin: &str,
) -> AnchorResult<Anchor<FileLeafStore, FileSigner, AcceptAll>> {
    let store = FileLeafStore::create(dir, origin).unwrap();
    Anchor::create_with_delegated_genesis(
        store,
        &root(dir),
        NOT_BEFORE,
        operational(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
}

/// Leaf 0 as it is on disk, read through a handle that never saw the append.
fn leaf_zero_from_disk(dir: &Path) -> Vec<u8> {
    FileLeafStore::open(dir)
        .unwrap()
        .leaf(0)
        .unwrap()
        .expect("leaf 0 must be on disk")
}

/// A [`Signer`] that advertises a key and declines to sign with it.
///
/// It returns [`AnchorError::SignerDeclined`] carrying a sentinel string this
/// file chose. **The sentinel is the point, not the variant:** matching it proves
/// the signer's own error reached the caller *unchanged* rather than being
/// replaced by one of this crate's, which is the property that would break
/// silently if the genesis path ever started mapping signer failures into its
/// own vocabulary.
///
/// An earlier version of this fixture borrowed `AnchorError::SignerKey` and said
/// so in a comment, because no variant meant "a remote signer declined". Writing
/// that comment is what surfaced the gap: the [`Signer`] contract documents a
/// failure the error type could not express, and `#[non_exhaustive]` meant no
/// downstream implementor could add one either. The variant now exists, so the
/// fixture no longer reports a key-file problem for something that is not one.
struct DecliningSigner {
    public_key: [u8; 32],
}

/// The sentinel [`DecliningSigner`] puts in its error, asserted on so that
/// propagation is checked rather than merely "an error came back".
const DECLINED_REASON: &str = "a signer that declined, and said so in these words";

impl Signer for DecliningSigner {
    fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    fn sign(&self, _message: &[u8]) -> AnchorResult<[u8; 64]> {
        Err(AnchorError::SignerDeclined {
            reason: DECLINED_REASON.to_string(),
        })
    }
}

/// A [`Signer`] that signs with one key and advertises another — a breach of
/// [`Signer`]'s stated contract, injected to check that this crate does not take
/// that contract on trust at the one position it can never repair.
struct MisadvertisingSigner {
    signing: FileSigner,
    advertised: [u8; 32],
}

impl Signer for MisadvertisingSigner {
    fn public_key(&self) -> [u8; 32] {
        self.advertised
    }

    fn sign(&self, message: &[u8]) -> AnchorResult<[u8; 64]> {
        self.signing.sign(message)
    }
}

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
            Err(AnchorError::GenesisRootKeyIsOperationalKey { origin }) => {
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
        Err(AnchorError::GenesisAlreadyWritten { origin, tree_size }) => {
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
        Err(AnchorError::SignerDeclined { reason }) => assert_eq!(reason, DECLINED_REASON),
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
        Err(AnchorError::GenesisDelegation { origin, source }) => {
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
