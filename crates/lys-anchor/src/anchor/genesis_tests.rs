#![cfg(test)]
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
//! # The open-time cases, and the injections run against them
//!
//! The second half of this file is about [`Anchor::open_verifying_genesis`] and
//! [`verify_genesis_delegation`] — what *opening* is willing to accept, which
//! was nothing at all until it existed. Its second party is the same one:
//! `lys-core`'s verifier, plus the fact that **every forged leaf 0 below is
//! signed**. Each forgery is built through the same two-phase pair `genesis.rs`
//! uses, so it is canonical, correctly typed, in the pair table and verifies
//! against the key in its own `kid`; each refusal is therefore a refusal of a
//! cryptographically perfect artifact, and each case says so by asserting that
//! `lys-core` **accepts** the artifact before this crate declines it.
//!
//! | injection into `genesis.rs` | failed |
//! |---|---|
//! | the self-delegation check deleted | `…delegates_the_operational_role_to_the_root_key` **only** |
//! | the `sequence` check deleted | `…sequence_is_not_the_genesis_sequence` **only** |
//! | the store's origin replaced by a committed constant (this file's own [`ORIGIN`]) | `…issued_for_a_different_origin` **only** |
//! | `verify_delegation` downgraded to a bare `from_cose_bytes` parse | `…root_key_the_caller_did_not_name`, `…issued_for_a_different_origin`, `…seat_delegation_whose_identifier_is_this_stores_origin` |
//! | the expected subject kind flipped `Domain` → `Seat` | eight cases, including every positive control |
//!
//! ⛔ **The parse-only row is the one to read twice, because of what stayed
//! green.** `an_uninterpreted_genesis_opens_under_open_and_is_refused_by_the_strict_open`
//! is the case the whole entry point exists for, and it **passed** under an
//! injection that removed the root-key check, the subject check and the kind
//! check together — because `b"genesis"` is not COSE, so the *parse* refuses it
//! and the verification never has to. A suite whose only strict-open case was
//! the headline one would report a working verifier while three of its four
//! checks were gone. The three cases that caught it all supply artifacts that
//! parse perfectly and differ in exactly one signed field.
//!
//! ⚠️ **The kind row is an honest failure of the injection, not a finding about
//! the suite** — the same shape already recorded above for the two creation-time
//! injections that broke the construction outright. Flipping the expected kind
//! makes every genuine domain genesis fail to verify, so every positive control
//! goes down with it and nothing is isolated. The kind's own work is shown
//! instead by the seat case, which hands the checker a *valid* seat delegation
//! whose identifier is literally this store's origin — the artifact a value-only
//! verifier would accept.
//!
//! Two cases carry no injection because what they assert is an **absence**:
//! `the_strict_open_does_not_require_the_opening_signer_to_be_the_delegated_key`
//! exists so that adding the obvious operational-key check trips a test that
//! explains why it must not be added (it would forbid rotation), and
//! `the_check_runs_without_a_signer…` pins that the rule is reachable by a party
//! holding only bytes.
//!
//! # What these cases could pass while the behaviour is wrong
//!
//! - **Nothing here forces any caller to use the strict open.**
//!   [`Anchor::open`](super::Anchor::open) still reads no byte of leaf 0, and the
//!   uninterpreted case asserts that it still opens such a store — the residual
//!   is recorded, not closed. A consumer that never calls
//!   `open_verifying_genesis` is exactly as unprotected as before.
//! - **`sequence == GENESIS_SEQUENCE` is checked against a convention.** These
//!   cases would still pass if the *format* grew a real genesis marker and this
//!   crate ignored it.
//! - **The `(Domain, Operational)` pair is `lys-core`'s rule.** The role
//!   assertion here would go green against an implementation that read the role
//!   off the artifact, because `lys-core` refuses the other pairs first.
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
use lys_core::delegation::{
    DelegationClaim, DelegationRole, DelegationSubjectKind, assemble_delegation,
    delegation_preimage, verify_delegation,
};
use lys_core::error::TrustError;
use lys_log_store::{FileLeafStore, LeafStore};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{AcceptAll, AdmissionPolicy, MaxSize, NotAdmitted, SubmitterContext};
use crate::anchor::Anchor;
use crate::error::SigningError;
use crate::keys::{FileSigner, Signer};
use crate::wire::Submission;

use super::*;

#[path = "genesis_tests/delegation.rs"]
mod delegation;
#[path = "genesis_tests/refusals.rs"]
mod refusals;
#[path = "genesis_tests/strict_open.rs"]
mod strict_open;

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
/// It returns [`SigningError::SignerDeclined`] carrying a sentinel string this
/// file chose. **The sentinel is the point, not the variant:** matching it proves
/// the signer's own error reached the caller *unchanged* rather than being
/// replaced by one of this crate's, which is the property that would break
/// silently if the genesis path ever started mapping signer failures into its
/// own vocabulary.
///
/// An earlier version of this fixture borrowed `SigningError::SignerKey` and said
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
        Err(AnchorError::Signing(SigningError::SignerDeclined {
            reason: DECLINED_REASON.to_string(),
        }))
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
