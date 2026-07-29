//! Authenticated sealed envelope: composition of [`super::seal`] with a
//! sender [`Attestation`] over the context-tagged sealed envelope bytes.
//!
//! [`sign_and_seal`] seals the payload for the recipient first, then signs
//! an attestation over `SEALED_ENVELOPE_CONTEXT_V1 || ephemeral_public_key
//! || ciphertext || nonce` with the sender's Ed25519 identity. The returned
//! tuple is the sealed envelope and the attestation that proves the envelope
//! came from that specific sender.
//!
//! The context prefix exists because attestations are a generic primitive:
//! without it, an attestation a sender legitimately produced over some other
//! payload that happened to equal a sealed envelope's canonical bytes could
//! be replayed as an envelope attestation (and vice versa). Prefixing the
//! attested message with a construction-specific tag binds the attestation
//! to *this* composition, so a signature only ever means "this sender sealed
//! this envelope" — never anything a generic attestation could be confused
//! with.
//!
//! [`open_and_verify`] inverts the composition: it checks that the
//! attestation's embedded signer key (carried in the COSE artifact's
//! signature-covered protected `kid` header — see
//! [`crate::attestation`]) matches the expected sender public key, verifies
//! the signature against the context-tagged sealed-envelope bytes, and only
//! then unseals. Failure of either attestation step short-circuits with
//! [`TrustError::AttestationFailed`] — the recipient never decrypts a
//! payload it cannot bind to a sender, which closes the substitution
//! oracle that bare [`super::seal`] necessarily leaves open.
//!
//! The standalone [`super::seal`] and [`super::open`] primitives remain
//! available for broadcast or anonymous use cases. Authenticated sealing is
//! a strict superset, not a replacement.

use x25519_dalek::StaticSecret;

use crate::attestation::artifact::Attestation;
use crate::attestation::sign::{sign_attestation, verify_attestation};
use crate::error::{TrustError, TrustResult};
use crate::keys::identity::Ed25519Identity;
use crate::seal::sealed_envelope::{SealedEnvelope, open, seal};

/// Context tag prefixed to the sealed-envelope bytes before attestation.
///
/// Binds the sender's attestation to the authenticated-seal composition
/// specifically. A generic [`sign_attestation`] over arbitrary bytes that
/// happen to match an envelope's canonical encoding cannot be replayed as
/// an envelope attestation, because it lacks this prefix; likewise an
/// envelope attestation cannot be presented as an attestation over some
/// other payload. Versioned so a future change to the attested message can
/// rotate the tag unambiguously.
const SEALED_ENVELOPE_CONTEXT_V1: &[u8] = b"lys/sealed-envelope/v1";

/// Build the attested message for `envelope`:
/// `SEALED_ENVELOPE_CONTEXT_V1 || ephemeral_public_key || ciphertext ||
/// nonce`.
///
/// Shared by [`sign_and_seal`] and [`open_and_verify`] so signer and
/// verifier can never drift apart.
fn contextualized_envelope_bytes(envelope: &SealedEnvelope) -> Vec<u8> {
    let envelope_bytes = envelope.attestation_bytes();
    let mut message = Vec::with_capacity(SEALED_ENVELOPE_CONTEXT_V1.len() + envelope_bytes.len());
    message.extend_from_slice(SEALED_ENVELOPE_CONTEXT_V1);
    message.extend_from_slice(&envelope_bytes);
    message
}

/// Seal `payload` for the recipient's X25519 public key and sign the
/// resulting envelope with `sender_identity`, returning the pair.
///
/// The attestation covers `SEALED_ENVELOPE_CONTEXT_V1 ||
/// ephemeral_public_key || ciphertext || nonce` — a construction-specific
/// context tag followed by every byte that travels with the envelope.
/// (`sign_attestation` hashes its input, so the attestation's COSE claims
/// carry `payload_hash = SHA-256(context-tagged envelope bytes)`; the
/// context binding survives the v2 COSE artifact unchanged, now with the
/// sender key additionally inside the signed bytes via the protected
/// `kid` header.)
/// Signing the canonical sealed-envelope bytes (rather than the plaintext)
/// means the sender commits to the exact ciphertext the recipient receives;
/// an adversary cannot replay or substitute parts of the envelope without
/// invalidating the signature. The context tag prevents a generic
/// attestation the sender produced elsewhere from being confused with an
/// envelope attestation (see the module docs).
///
/// # Errors
///
/// Returns whatever [`seal`] returns ([`TrustError::Seal`] on a low-order
/// recipient public key, AES-GCM failure, or HKDF failure).
/// [`sign_attestation`] is itself infallible (see the attestation module
/// docs), so the only error path is through `seal`.
pub fn sign_and_seal(
    payload: &[u8],
    sender_identity: &Ed25519Identity,
    recipient_x25519_public_key: &[u8; 32],
) -> TrustResult<(SealedEnvelope, Attestation)> {
    let envelope = seal(payload, recipient_x25519_public_key)?;
    let attestation = sign_attestation(&contextualized_envelope_bytes(&envelope), sender_identity);
    Ok((envelope, attestation))
}

/// Verify that `attestation` was produced by `sender_public_key` over
/// `envelope` and, only on success, unseal the envelope with
/// `recipient_x25519_secret`.
///
/// Verification is two gates in strict order:
///
/// 1. The attestation's embedded signer public key must equal
///    `sender_public_key`. This rejects forgeries where an adversary signs a
///    valid sealed envelope with their own key and hopes the recipient
///    accepts it.
/// 2. The attestation signature must verify against the context-tagged
///    sealed-envelope bytes (`SEALED_ENVELOPE_CONTEXT_V1 || canonical
///    envelope bytes`), so only attestations produced for this composition
///    are accepted — a generic attestation over the bare envelope bytes is
///    rejected.
///
/// Either failure returns [`TrustError::AttestationFailed`] *before* the
/// AES-GCM cipher is touched, so the recipient is not an unsealing oracle
/// for envelopes whose sender cannot be verified.
///
/// # Errors
///
/// - [`TrustError::AttestationFailed`] if the embedded signer key does not
///   match `sender_public_key`, or the signature does not verify against
///   the context-tagged sealed-envelope bytes.
/// - [`TrustError::UnsealFailed`] if the attestation verifies but the
///   ciphertext or nonce fail AES-GCM authentication (tampering after
///   signing is structurally impossible since the signature covers
///   nonce + ciphertext + ephemeral key, but the unseal error is still
///   reported for completeness).
pub fn open_and_verify(
    envelope: &SealedEnvelope,
    attestation: &Attestation,
    sender_public_key: &[u8; 32],
    recipient_x25519_secret: &StaticSecret,
) -> TrustResult<Vec<u8>> {
    if attestation.signer_public_key != *sender_public_key {
        return Err(TrustError::AttestationFailed);
    }
    verify_attestation(attestation, &contextualized_envelope_bytes(envelope))
        .map_err(|_err| TrustError::AttestationFailed)?;
    open(envelope, recipient_x25519_secret)
}

#[cfg(test)]
#[path = "authenticated_tests.rs"]
mod tests;
