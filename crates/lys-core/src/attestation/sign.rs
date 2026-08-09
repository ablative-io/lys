//! Sign and verify `lys/attestation/v2` `COSE_Sign1` attestations over
//! arbitrary payload bytes.
//!
//! [`sign_attestation`] hashes the payload with SHA-256, captures the
//! current unix-millisecond timestamp, and signs the COSE `Sig_structure`
//! (RFC 9052 §4.4, empty `external_aad`) with the supplied
//! [`Ed25519Identity`]:
//!
//! ```text
//! Sig_structure = ["Signature1", protected, h'', claims]
//! protected     = {1: -8 (EdDSA), 3: <v2 content type>, 4: <signer key>}
//! claims        = {1: <SHA-256 payload hash>, 2: <unix-ms timestamp>}
//! ```
//!
//! [`verify_attestation`] recomputes the digest of the candidate payload,
//! compares it against `attestation.payload_hash`, rebuilds the
//! `Sig_structure` from the attestation's own fields, and verifies the
//! Ed25519 signature against `attestation.signer_public_key` (strict
//! verification). Any mismatch — wrong payload, tampered signature,
//! tampered timestamp, or wrong signer key — collapses to
//! [`TrustError::InvalidSignature`].
//!
//! Verification is v2-only: the COSE `Sig_structure` above is the sole
//! accepted signing scheme. A signature over anything else — the deleted v1
//! preimage (`lys/attestation/v1 ‖ timestamp_le ‖ hash`), a bare payload
//! hash, raw payload bytes — is rejected. There is no fallback path.
//!
//! Two properties fall out of the construction:
//!
//! - **Every field is authenticated.** The signer key (protected `kid`),
//!   the payload hash, and the timestamp all ride inside the signed bytes;
//!   none can be altered after signing without invalidating the signature.
//! - **Domain separation.** The signed message always begins
//!   `0x84 0x6A "Signature1"`, which is byte-0 disjoint from every other
//!   signing context in this crate: the old v1 preimage began `0x6C`
//!   (deleted), signed-note bodies are valid UTF-8 (`0x84` is a UTF-8
//!   continuation byte, impossible at position 0), and X.509 TBS bytes
//!   begin `0x30` (DER SEQUENCE). Among future lys COSE artifacts the
//!   protected content type — itself signature-covered and pinned by the
//!   verifier — separates the v2 attestation. The raw
//!   [`Ed25519Identity::sign`] primitive itself stays unprefixed by
//!   necessity: the CA path signs exact X.509 TBS bytes through it, and
//!   those bytes must not be altered. Any raw-sign caller whose message
//!   could start with a valid `Sig_structure` encoding would need its own
//!   separation; within this crate no such caller exists.
//!
//! The signed claims embed the 32-byte hash, not the raw payload. This
//! keeps the signing input fixed-size and uniform regardless of payload
//! length. Consumers that need to attest to large payloads pass the bytes
//! once and let `sign_attestation` produce the canonical hash.

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::attestation::artifact::Attestation;
use crate::attestation::encoding;
use crate::cbor;
use crate::error::{TrustError, TrustResult};
use crate::keys::compare::bytes_eq_no_early_exit as bytes_eq;
use crate::keys::identity::Ed25519Identity;

/// Hash `payload` with SHA-256, capture the current timestamp, sign the
/// COSE `Sig_structure` with `signing_key`, and package the result as an
/// [`Attestation`].
///
/// The signature covers `["Signature1", protected, h'', claims]` — the
/// protected bucket carries the `EdDSA` algorithm, the v2 content type, and
/// the signer's public key; the claims carry the payload digest and the
/// unix-millisecond timestamp. The original payload bytes are not stored
/// on the attestation.
///
/// `sign_attestation` is infallible: `Utc::now().timestamp_millis()` is
/// total over the representable date range, the hand encoder is total over
/// the field types, and Ed25519 deterministic signing has no failure mode
/// in dalek 2.
pub fn sign_attestation(payload: &[u8], signing_key: &Ed25519Identity) -> Attestation {
    let payload_hash = sha256_digest(payload);
    let timestamp = Utc::now().timestamp_millis();
    let signer_public_key = signing_key.public_key_bytes();
    let protected = encoding::protected_bytes(&signer_public_key);
    let claims = encoding::claims_bytes(&payload_hash, timestamp);
    let signature = signing_key.sign(&cbor::sig_structure_bytes(&protected, &claims));
    Attestation {
        payload_hash,
        signature,
        signer_public_key,
        timestamp,
    }
}

/// Verify that `attestation` is a valid signature over `payload` and the
/// attestation's own timestamp by `attestation.signer_public_key`.
///
/// # ⚠️ This answers "is this self-consistent", NOT "is this from someone I trust"
///
/// **The key it verifies against is the key the artifact carries.** An attacker
/// signs an attestation over any payload with their own key, puts their own
/// public key in the artifact, and this function returns `Ok`. The signature is
/// cryptographically perfect and it vouches for nothing — the same trap as a
/// self-signed certificate, where the signature proves the holder signed it and
/// says nothing about who the holder is.
///
/// A caller that has not separately decided which key it expects has performed
/// no authentication. Use [`verify_attestation_by_signer`] to state the
/// expected key and have the comparison done here, where it is done without an
/// early exit and is not distinguishable from a signature failure.
///
/// The check is two-step: the SHA-256 digest of `payload` must equal
/// `attestation.payload_hash`, and the Ed25519 signature must strictly
/// verify against the embedded public key over the `Sig_structure` rebuilt
/// from the attestation's own fields. Because the signed bytes are rebuilt
/// from the struct, a tampered timestamp, hash, or signer key fails
/// signature verification. All failures collapse to
/// [`TrustError::InvalidSignature`] so callers cannot distinguish them by
/// error variant — a tampered payload, a tampered timestamp, and a forged
/// signature all look the same to the verifier, which is the desired
/// property.
///
/// Verification is v2-only: only signatures over the COSE `Sig_structure`
/// are accepted. A signature over the deleted v1 preimage or the bare
/// payload hash is rejected like any other invalid signature.
///
/// # Errors
///
/// Returns [`TrustError::InvalidSignature`] if the recomputed payload hash
/// does not match `attestation.payload_hash`, if the public key is not a
/// valid Ed25519 point, or if the signature does not strictly verify over
/// the rebuilt `Sig_structure` (covering tampered signature bytes and
/// tampered timestamps alike).
pub fn verify_attestation(attestation: &Attestation, payload: &[u8]) -> TrustResult<()> {
    let recomputed = sha256_digest(payload);
    if recomputed != attestation.payload_hash {
        return Err(TrustError::InvalidSignature);
    }
    check_signature(attestation)
}

// The instrument behind `the_signature_check_still_runs_when_the_signer_is_wrong`.
//
// WHY ONE IS NEEDED: `verify_attestation_by_signer` claims that the signature
// work and the key comparison both always happen. An edit that returns early on
// a signer mismatch produces exactly the same error for exactly the same inputs,
// so every outcome-based test stays green while the claim becomes false. That is
// the defect shape `delegation::verify_delegation` was measured at 32.8× on, and
// the fix there needed a counter for the same reason: a wall-clock assertion is
// flaky under load, and a machine fast enough to blur the difference reports
// success.
//
// Thread-local because Rust's test harness gives each test its own thread, so
// the count is exactly "verifications this test caused" with no serialisation
// and no cross-test races. A global counter would race, and a race here fails
// OPEN.
//
// ⚠️ It is `#[cfg(test)]`, so the property is instrument-guaranteed in test
// builds only — the same limitation already recorded for the delegation counter,
// restated rather than inherited silently.
#[cfg(test)]
thread_local! {
    /// Count of Ed25519 verifications [`check_signature`] has performed on this
    /// thread.
    pub(crate) static SIGNATURE_VERIFICATIONS: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

/// Rebuild the `Sig_structure` and check the signature, recording that it
/// happened.
///
/// Extracted so the count and the verification cannot be separated: an edit that
/// skips the work also skips the increment, which is what makes the counter a
/// measurement rather than a decoration sitting beside one.
fn check_signature(attestation: &Attestation) -> TrustResult<()> {
    #[cfg(test)]
    SIGNATURE_VERIFICATIONS.with(|count| count.set(count.get().saturating_add(1)));

    let protected = encoding::protected_bytes(&attestation.signer_public_key);
    let claims = encoding::claims_bytes(&attestation.payload_hash, attestation.timestamp);
    let sig_structure = cbor::sig_structure_bytes(&protected, &claims);
    Ed25519Identity::verify(
        &attestation.signer_public_key,
        &sig_structure,
        &attestation.signature,
    )
}

/// Verify `attestation` against `payload` **and** that it was signed by
/// `expected_signer_public_key` — the authenticating verifier.
///
/// [`verify_attestation`] establishes only that an attestation is internally
/// consistent, because it verifies against the key the artifact itself carries.
/// This function adds the missing half: the caller says which key it is willing
/// to believe, and an artifact signed by any other key is refused however
/// well-formed it is.
///
/// # Why the expected key is compared here and not by the caller
///
/// A caller writing `if att.signer_public_key == expected` after a successful
/// verify gets the right answer and two wrong properties. It compares with
/// `==`, which may return on the first differing byte; and it runs the
/// comparison only when verification already succeeded, so *whether* the
/// comparison happens is itself observable. Doing it here means the signature
/// work and the key comparison both always happen, and the two failures are
/// indistinguishable in the returned value.
///
/// # Errors
///
/// Returns [`TrustError::InvalidSignature`] — the single value — for a payload
/// mismatch, an invalid signature, **and** a signer that is not
/// `expected_signer_public_key`. A caller must not be able to learn which of
/// the three it hit, since "your signature was fine but you are not who I
/// wanted" tells a prober exactly which key the verifier holds.
pub fn verify_attestation_by_signer(
    attestation: &Attestation,
    payload: &[u8],
    expected_signer_public_key: &[u8; 32],
) -> TrustResult<()> {
    // Both operands are computed before either is consulted, and combined with
    // `&` rather than `&&`. This mirrors `delegation::verify_delegation`, where
    // an early return on the key comparison was measured at 32.8× and is a
    // security property rather than a style choice.
    let inner_ok = verify_attestation(attestation, payload).is_ok();
    let signer_ok = bytes_eq(&attestation.signer_public_key, expected_signer_public_key);

    if inner_ok & signer_ok {
        Ok(())
    } else {
        Err(TrustError::InvalidSignature)
    }
}

/// Parse a tagged `COSE_Sign1` artifact, verify it against `payload`, and
/// require that it was signed by `expected_signer_public_key` — the
/// authenticating counterpart to [`verify_attestation_bytes`].
///
/// # Errors
///
/// Returns [`TrustError::InvalidSignature`] for every failure: malformed or
/// non-canonical artifact, payload mismatch, invalid signature, and unexpected
/// signer are all deliberately indistinguishable.
pub fn verify_attestation_bytes_by_signer(
    cose: &[u8],
    payload: &[u8],
    expected_signer_public_key: &[u8; 32],
) -> TrustResult<Attestation> {
    let attestation = Attestation::from_cose_bytes(cose)?;
    verify_attestation_by_signer(&attestation, payload, expected_signer_public_key)?;
    Ok(attestation)
}

/// Parse a tagged `COSE_Sign1` artifact and verify it against `payload` in
/// one step, returning the parsed [`Attestation`] on success — the
/// bytes-in convenience mirroring `verify_note`.
///
/// ⚠️ **Self-consistency only** — it verifies against the key the artifact
/// carries. See [`verify_attestation`]'s warning, and
/// [`verify_attestation_bytes_by_signer`] for the authenticating form.
///
/// Equivalent to [`Attestation::from_cose_bytes`] followed by
/// [`verify_attestation`]; used by consumers (such as the CLI) that hold
/// the raw `.cose` file bytes.
///
/// # Errors
///
/// Returns [`TrustError::InvalidSignature`] for every failure — malformed
/// or non-canonical artifact, payload mismatch, and invalid signature are
/// deliberately indistinguishable (non-oracle).
pub fn verify_attestation_bytes(cose: &[u8], payload: &[u8]) -> TrustResult<Attestation> {
    let attestation = Attestation::from_cose_bytes(cose)?;
    verify_attestation(&attestation, payload)?;
    Ok(attestation)
}

/// SHA-256 digest of `bytes` as a fixed-size 32-byte array.
fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;
