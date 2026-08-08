//! Issue and verify `lys/delegation/v1` `COSE_Sign1` delegations.
//!
//! ```text
//! Sig_structure = ["Signature1", protected, h'', payload]
//! protected     = {1: -8 (EdDSA), 3: <v1 delegation content type>,
//!                  4: <root key>}
//! payload       = {1: subject_kind, 2: subject_value, 3: delegated key,
//!                  4: role, 5: not_before_unix_ms, 6: sequence}
//! ```
//!
//! # ⚠️ `kid` is a claim, not an authority
//!
//! This is the central trap of the format and it is worth stating in full,
//! because the artifact is *designed* to look self-sufficient. A delegation
//! carries the root key in its own protected header, so it verifies against
//! whatever key it carries. **An attacker signs a delegation for origin `O`
//! with their own root key, puts their own key in `kid`, and the result is
//! cryptographically perfect.** Every signature check passes. It vouches for
//! nothing, and a verifier that reads the key out of the artifact will accept
//! it — the same trap as a self-signed certificate, where the signature proves
//! someone holding a key *said* something and never that the statement is true.
//!
//! So [`verify_delegation`] takes the expected root key as a **required
//! argument** and enforces `kid == expected`. There is deliberately no
//! "just verify this" entry point. Callers who genuinely want to inspect an
//! unattributed delegation use
//! [`Delegation::from_cose_bytes`](crate::delegation::Delegation::from_cose_bytes),
//! which is honestly labelled as parsing rather than verification.
//!
//! # The subject is required for the same reason, one level down
//!
//! Without a subject check, a delegation issued for anchor A is a valid
//! delegation for anchor B whenever B's operator holds the same root key — or,
//! worse, whenever a verifier simply neglects to notice which subject it is
//! looking at. [`verify_delegation`] therefore takes the expected subject as a
//! required argument too, and enforces it against the **payload** subject, which
//! is inside the signed bytes.
//!
//! **The comparison is raw UTF-8 byte equality.** Not case-folded, not
//! Unicode-normalised, no trailing-dot, port or scheme handling. Every
//! alternative is a normalisation rule that two implementations can apply
//! differently, and a comparison that differs between verifiers is a comparison
//! an attacker gets to pick the side of. An ASCII origin cannot expose the
//! difference; an internationalised one would, which is why the rule is stated
//! rather than left to whichever library a future caller reaches for.
//!
//! # ⚠️ The subject KIND is a separate required argument, and it is not redundant
//!
//! [`verify_delegation`] takes the expected subject **kind** as well as the
//! expected subject **value**, and enforces both. That looks like belt and braces
//! and is not.
//!
//! A verifier that checked only the value would accept a **seat** delegation
//! whose seat identifier happened to equal an origin string. A seat identifier is
//! arbitrary text minted somewhere else, so that collision is **an attacker's
//! choice rather than an accident**: pick a seat identifier equal to the target
//! origin, obtain a `(seat, speaks-for)` delegation from a root key the target
//! verifier trusts, and a value-only verifier reads it as authority over the
//! origin. Requiring the kind makes the two namespaces disjoint by construction
//! instead of by the hope that they never overlap.
//!
//! This is the discipline the format applies *between* artifact kinds — a
//! receipt's content type is not a delegation's — applied *within* one. The pair
//! rule at decode ([`DelegationSubjectKind::permits`]) is the other half: it
//! guarantees a `(seat, ...)` payload cannot carry `operational`, so what a
//! kind-checking verifier refuses it refuses for a reason the artifact itself
//! records.
//!
//! [`DelegationSubjectKind::permits`]: crate::delegation::DelegationSubjectKind::permits
//!
//! # Domain separation from receipts and attestations
//!
//! The signing preimage begins `0x84 0x6A "Signature1"` — **identical** to a
//! receipt's, a consistency receipt's and a `lys/attestation/v2` attestation's.
//! Byte-0 disjointness does *not* separate them and re-deriving that comfort
//! would be a mistake. What separates them is the content type inside the
//! signed bytes together with each verifier pinning its own: a delegation is
//! refused by receipt and attestation verification, and neither of those is
//! accepted here, in both cases before any signature is examined.
//!
//! # Why signing is two-phase
//!
//! [`delegation_preimage`] and [`assemble_delegation`] exist alongside the
//! [`sign_delegation`] convenience, which is implemented in terms of them. The
//! split is not a third pattern for its own sake: **the root key is the offline
//! key**, whose entire job is signing delegations into the log precisely so it
//! can live somewhere the operational key cannot. Issuing one is therefore the
//! signing operation an operator would most want to perform on an air-gapped
//! machine, and the two-phase shape is what makes that possible without this
//! crate becoming signer-generic — hand the preimage to whatever holds the key,
//! bring back 64 bytes.
//!
//! [`assemble_delegation`] verifies the supplied signature before returning.
//! Assembling an artifact whose signature does not match its own bytes produces
//! a file that fails verification at some later, far less debuggable moment —
//! typically in a stranger's hands, years on, with no way to tell a transcription
//! error from a forgery.

use crate::cbor;
use crate::delegation::artifact::{Delegation, DelegationClaim, DelegationSubjectKind};
use crate::delegation::encoding::{self, KEY_LEN};
use crate::error::{TrustError, TrustResult};
use crate::keys::identity::Ed25519Identity;

/// The RFC 9052 §4.4 `Sig_structure` bytes a delegation's signature covers:
/// `["Signature1", protected, h'', payload]`.
///
/// # `external_aad` is pinned to the empty byte string
///
/// The third element is `h''` — zero-length, encoded `0x40`, RFC 9052 §4.4's
/// default. It is pinned here rather than left to the default because it sits
/// **inside** the signed bytes: an unpinned `external_aad` means two conforming
/// implementations produce different signatures over the same claim and neither
/// is wrong. It is the kind of gap that survives review precisely because
/// everyone in the room happens to know the answer, so it is written down.
///
/// This is phase one of the two-phase signing path — hand these bytes to
/// whatever holds the offline root key. It is a pure function of its arguments
/// and touches no key material.
pub fn delegation_preimage(root_public_key: &[u8; KEY_LEN], claim: &DelegationClaim) -> Vec<u8> {
    cbor::sig_structure_bytes(
        &encoding::protected_bytes(encoding::CONTENT_TYPE, root_public_key),
        &encoding::payload_bytes(claim),
    )
}

/// Assemble the tagged `COSE_Sign1` artifact from a claim and a signature
/// produced over [`delegation_preimage`] — phase two of the two-phase path.
///
/// The signature is verified against `root_public_key` and the re-derived
/// preimage **before** any bytes are returned. This is not a courtesy check: an
/// artifact whose signature does not match its own bytes is indistinguishable
/// from a forgery to everyone who later receives it, so the only moment it can
/// be caught cheaply is here, where the operator still has the inputs in front
/// of them.
///
/// # It also refuses every claim the decoder would refuse
///
/// The signature check above was written to stop this function emitting an
/// artifact that fails verification later — and then a *second* route to that
/// same outcome was found, going around it. The artifact size cap was enforced
/// on the decode side only, so a subject value of 3884 bytes signed and verified
/// while 3885 signed **successfully** and failed every verification afterwards.
/// An empty subject value and an unusable delegated key had the same shape.
///
/// So the encode side now mirrors the decode side in full, through
/// `encoding::check_encodable`: a non-empty subject value, a
/// `(subject_kind, role)` pair this version defines, a delegated key strict
/// Ed25519 could accept, and an encoded length within the cap. "Verify before
/// returning" is the *principle*; the signature was only ever one instance of
/// it.
///
/// # Errors
///
/// - [`TrustError::DelegationEncoding`], with an actionable reason, if the claim
///   is one this crate's own decoder would reject — including a
///   `(subject_kind, role)` pair this version does not define. Descriptive rather
///   than non-oracle: this is issuance, the caller supplied every input and holds
///   the key, and no stranger is being kept in the dark.
/// - [`TrustError::DelegationVerification`] if the signature does not verify —
///   strictly, so malleable signatures and small-order keys are refused at
///   issuance rather than being discovered at verification.
pub fn assemble_delegation(
    root_public_key: &[u8; KEY_LEN],
    claim: &DelegationClaim,
    signature: &[u8; 64],
) -> TrustResult<Vec<u8>> {
    encoding::check_encodable(root_public_key, claim)?;
    let preimage = delegation_preimage(root_public_key, claim);
    Ed25519Identity::verify(root_public_key, &preimage, signature)
        .map_err(|_err| TrustError::DelegationVerification)?;
    Ok(encoding::artifact_bytes(root_public_key, claim, signature))
}

/// Sign `claim` with `root_key` and return the tagged `COSE_Sign1` artifact —
/// the single-machine convenience, implemented in terms of
/// [`delegation_preimage`] and [`assemble_delegation`] so there is exactly one
/// definition of what gets signed.
///
/// The root key in the protected `kid` is taken from `root_key` itself and is
/// not a caller-supplied field, so a delegation whose `kid` is not the key that
/// signed it cannot be produced by this path at all.
///
/// # Errors
///
/// - [`TrustError::DelegationEncoding`] if the claim is one the decoder would
///   reject — an empty subject value, a `(subject_kind, role)` pair this version
///   does not define, an unusable delegated key, or a subject value long enough
///   to push the artifact past the size cap. Reached through
///   [`assemble_delegation`], and the reason names the constraint.
/// - [`TrustError::DelegationVerification`] if the signature this function just
///   produced does not verify against the key that produced it. That is
///   unreachable for a well-formed Ed25519 identity; the check is kept rather
///   than asserted away because the alternative is an `expect` in library code,
///   and because it is the same gate an externally supplied signature passes
///   through.
pub fn sign_delegation(
    root_key: &Ed25519Identity,
    claim: &DelegationClaim,
) -> TrustResult<Vec<u8>> {
    let root_public_key = root_key.public_key_bytes();
    let signature = root_key.sign(&delegation_preimage(&root_public_key, claim));
    assemble_delegation(&root_public_key, claim, &signature)
}

/// Verify a tagged `COSE_Sign1` delegation and return it.
///
/// Five things must hold, and every failure returns the same error:
///
/// 1. The bytes parse as a canonical `lys/delegation/v1` artifact —
///    including the content type pin, which is what refuses a receipt or an
///    attestation here, the `(subject_kind, role)` pair rule, and the
///    empty-unprotected-bucket rule.
/// 2. The artifact names the expected root key in its protected `kid`. Without
///    this the check would prove only that *someone* signed, which is not a
///    trust statement — see the module docs.
/// 3. The payload `subject_kind` equals `expected_subject_kind`. **Not redundant
///    with the value below**: a seat identifier is arbitrary text chosen
///    elsewhere, so a value-only verifier can be handed a seat delegation whose
///    identifier equals the origin it cares about, and that collision is an
///    attacker's choice rather than an accident. See the module docs.
/// 4. The payload `subject_value` equals `expected_subject_value` by **raw UTF-8
///    byte equality** — no case folding, no Unicode normalisation, no
///    trailing-dot or port or scheme handling. This is what refuses a delegation
///    replayed from another anchor; see the module docs for why the comparison is
///    the strict one.
/// 5. The signature verifies over the re-derived `Sig_structure`, using strict
///    Ed25519 — malleable signatures and small-order keys are rejected, because
///    non-repudiation requires a unique valid signature.
///
/// # ⛔ None of the four checks returns early, and the reason is measured
///
/// An earlier version of this function checked `kid`, then the subject, then the
/// signature, returning at the first failure — and its doc claimed that "nothing
/// depends on the order: every failure returns the same value, so it is not
/// observable." **That was false.** An adversarial review measured a **32.8×**
/// separation between a `kid`/subject mismatch and a bad signature, with a
/// round-robin sampling design whose control arm reproduced to 1.001×. Ed25519
/// verification dominates everything that precedes it, so skipping it is loud.
///
/// The error values were identical the whole time. A delegation verifier is
/// examined by strangers, so the leak answers *"is this verifier configured to
/// trust root key K, for subject S?"* to anyone who can hand it bytes and a
/// stopwatch. That does not brute-force a key — it **confirms a guess**, and for
/// a public anchor the candidate set is small.
///
/// **The general rule: a collapsed error type is not a non-oracle if the amount
/// of work done differs per cause.** An error enum is observable in one channel;
/// a function is observable in several. So the signature verification runs
/// unconditionally and all four results are combined into one decision at the
/// end.
///
/// # What this does and does not claim
///
/// It removes the *dominant* signal — an entire Ed25519 verification. It is
/// **not** a claim of constant-time execution:
///
/// - The `kid` and subject-value comparisons use a fold that visits every byte
///   rather than stopping at the first difference, so no prefix oracle remains
///   in the source. The subject-*kind* comparison is an equality between two
///   two-inhabitant enums and does no data-dependent work worth equalising. Rust's optimiser is free to reintroduce branching, and this
///   crate takes no dependency capable of forbidding it.
/// - **Parse failure is still distinguishable from verification failure**, and
///   that is accepted rather than overlooked. A malformed artifact returns
///   before any signature work. What that reveals is a property of the *input*,
///   which the party who supplied it already knows — not a property of the
///   verifier's configuration, which is the thing being protected here.
///
/// # Errors
///
/// Returns [`TrustError::DelegationVerification`] for every failure. A malformed
/// or non-canonical artifact, a delegation from a root key the caller does not
/// trust, one issued for a different subject, one issued for a different subject
/// *kind* under the same value, one carrying an unknown role or an undefined
/// `(subject_kind, role)` pair, and a forged signature all return that one value:
/// a delegation verifier is exactly the network-exposed surface where a
/// distinguishable error becomes an oracle for the verifier's own configuration.
pub fn verify_delegation(
    cose: &[u8],
    expected_root_public_key: &[u8; KEY_LEN],
    expected_subject_kind: DelegationSubjectKind,
    expected_subject_value: &str,
) -> TrustResult<Delegation> {
    let delegation = Delegation::from_cose_bytes(cose)?;

    // The signature check runs FIRST and UNCONDITIONALLY. Moving it after the
    // comparisons, or making it conditional on them, restores the 32.8×
    // oracle — this ordering is a security property, not a style choice, and
    // `signature_verification_runs_for_every_mismatch_kind` counts it.
    let signature_ok = check_signature(&delegation);

    let kid_ok = bytes_eq(&delegation.root_public_key, expected_root_public_key);
    let subject_kind_ok = delegation.claim.subject_kind == expected_subject_kind;
    let subject_value_ok = bytes_eq(
        delegation.claim.subject_value.as_bytes(),
        expected_subject_value.as_bytes(),
    );

    // `&` rather than `&&`: all four operands are already computed, and the
    // non-short-circuiting form says so to anyone reading it later.
    if signature_ok & kid_ok & subject_kind_ok & subject_value_ok {
        Ok(delegation)
    } else {
        Err(TrustError::DelegationVerification)
    }
}

// The instrument behind `signature_verification_runs_for_every_mismatch_kind`.
//
// WHY AN INSTRUMENT WAS NECESSARY AT ALL: the timing fix in
// `verify_delegation` changes *no behaviour*. An early return on a `kid`
// mismatch produces exactly the same error for exactly the same inputs.
// Restoring it was injected as a drift and the entire suite stayed green,
// because every test asserted outcomes and the defect is in the work done. A
// wall-clock assertion would be the direct measurement and the wrong tool —
// flaky under load, and a machine fast enough to blur the difference would
// report success.
//
// Thread-local rather than global because Rust's test harness gives each test
// its own thread, so a per-thread count is exactly "verifications this test
// caused", with no serialisation and no cross-test interference. A global
// counter would race, and a race here fails *open*.
#[cfg(test)]
thread_local! {
    /// Count of Ed25519 verifications [`check_signature`] has performed on this
    /// thread.
    pub(crate) static SIGNATURE_VERIFICATIONS: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

/// Re-derive the preimage and check the signature, recording that it happened.
///
/// Extracted so the count and the verification cannot be separated: an edit that
/// skips the work also skips the increment, which is what makes the counter a
/// measurement rather than a decoration sitting next to one.
fn check_signature(delegation: &Delegation) -> bool {
    let preimage = delegation_preimage(&delegation.root_public_key, &delegation.claim);
    let outcome = Ed25519Identity::verify(
        &delegation.root_public_key,
        &preimage,
        &delegation.signature,
    )
    .is_ok();
    #[cfg(test)]
    SIGNATURE_VERIFICATIONS.with(|count| count.set(count.get() + 1));
    outcome
}

/// Byte-slice equality that visits every byte of the shorter slice rather than
/// stopping at the first difference.
///
/// Used for the `kid` and subject-value comparisons in [`verify_delegation`].
/// The values are not secret — the *expected* ones are, in the sense that whether
/// a verifier holds them is the thing an attacker is probing — so what matters is
/// that a near-miss costs the same as a far-miss.
///
/// **This is early-exit removal, not a constant-time guarantee.** The length
/// comparison is a branch, and a subject value's length is already observable
/// from the artifact's size; the optimiser may reintroduce branching in the loop. A real
/// constant-time claim needs a primitive that can forbid that, which this crate
/// does not depend on. Stated plainly because the previous version of this
/// module overclaimed a related property and was wrong.
fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (l, r) in left.iter().zip(right.iter()) {
        difference |= l ^ r;
    }
    difference == 0
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;
