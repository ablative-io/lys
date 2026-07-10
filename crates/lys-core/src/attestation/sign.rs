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
use crate::error::{TrustError, TrustResult};
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
    let signature = signing_key.sign(&encoding::sig_structure_bytes(&protected, &claims));
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
    let protected = encoding::protected_bytes(&attestation.signer_public_key);
    let claims = encoding::claims_bytes(&attestation.payload_hash, attestation.timestamp);
    let sig_structure = encoding::sig_structure_bytes(&protected, &claims);
    Ed25519Identity::verify(
        &attestation.signer_public_key,
        &sig_structure,
        &attestation.signature,
    )
}

/// Parse a tagged `COSE_Sign1` artifact and verify it against `payload` in
/// one step, returning the parsed [`Attestation`] on success — the
/// bytes-in convenience mirroring `verify_note`.
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
