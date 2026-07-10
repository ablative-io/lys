//! [`Attestation`] — the parsed form of the `lys/attestation/v2` tagged
//! `COSE_Sign1` artifact.
//!
//! # Invariants
//!
//! - **The only durable form is the COSE bytes.** The struct deliberately
//!   implements no `serde` traits: nothing can persist an attestation except
//!   [`Attestation::to_cose_bytes`], so no second wire shape can ever exist
//!   alongside the frozen COSE artifact.
//! - **Round-trip identity:** `from_cose_bytes(x.to_cose_bytes()) == x` for
//!   every `Attestation` value, and `to_cose_bytes` of a parsed artifact
//!   reproduces the input bytes exactly.
//! - **Canonical-encoding strictness:** [`Attestation::from_cose_bytes`]
//!   rejects any input that is not byte-identical to the canonical
//!   re-encoding of its parsed fields — including inputs whose signature is
//!   cryptographically valid (unprotected-header smuggling, indefinite
//!   lengths, oversized integer heads, reordered maps, tag stripping,
//!   duplicate keys, trailing garbage). Vanilla COSE verifiers accept some
//!   of those mutants; lys never does.
//! - Parsing performs no signature verification; pair `from_cose_bytes` with
//!   [`super::sign::verify_attestation`] (or use
//!   [`super::sign::verify_attestation_bytes`]) before trusting the fields.
//! - Every parse failure collapses to
//!   [`TrustError::InvalidSignature`](crate::error::TrustError::InvalidSignature)
//!   (non-oracle).

use crate::attestation::encoding;
use crate::error::{TrustError, TrustResult};

/// Ed25519-signed attestation over a payload's SHA-256 hash, carried on the
/// wire as a tagged `COSE_Sign1` (RFC 9052) — the `lys/attestation/v2`
/// artifact.
///
/// Constructed by [`super::sign::sign_attestation`] or parsed from artifact
/// bytes by [`Self::from_cose_bytes`]; consumed by
/// [`super::sign::verify_attestation`]. The struct is intentionally a plain
/// record with public fields — the trust crate provides the primitive;
/// consumers wrap it with their domain meaning (execution receipt, audit
/// entry, etc.).
///
/// Every field is signature-covered: the signer key rides in the protected
/// `kid` header and the hash and timestamp are claims inside the payload,
/// all inside the Ed25519-signed `Sig_structure` (RFC 9052 §4.4). The
/// artifact never carries the attested payload bytes themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    /// SHA-256 digest of the attested payload bytes (claims key `1`).
    pub payload_hash: [u8; 32],

    /// Ed25519 detached signature over the COSE `Sig_structure`
    /// `["Signature1", protected, h'', claims]` (`COSE_Sign1` item 3).
    pub signature: [u8; 64],

    /// Ed25519 verifying key of the signer, from the protected `kid`
    /// header — inside the signed bytes, so it cannot be swapped without
    /// invalidating `signature`.
    pub signer_public_key: [u8; 32],

    /// Unix-millisecond timestamp captured when the attestation was signed
    /// (claims key `2`).
    ///
    /// Authenticated: the timestamp is a signed claim, so it cannot be
    /// altered after signing without invalidating `signature`. Stored as
    /// `i64` (matching `chrono::DateTime::timestamp_millis`) so pre-epoch
    /// timestamps remain representable; the trust crate makes no
    /// monotonicity or freshness guarantees — those are the consumer's
    /// responsibility.
    pub timestamp: i64,
}

impl Attestation {
    /// Encode this attestation as its canonical tagged `COSE_Sign1` artifact
    /// (191–199 bytes, media type `application/cose`).
    ///
    /// Infallible: the artifact shape is fixed and the hand encoder is
    /// total over the field types. The emitted bytes verify with any
    /// off-the-shelf COSE library.
    pub fn to_cose_bytes(&self) -> Vec<u8> {
        encoding::artifact_bytes(
            &self.signer_public_key,
            &self.payload_hash,
            self.timestamp,
            &self.signature,
        )
    }

    /// Parse a tagged `COSE_Sign1` `lys/attestation/v2` artifact.
    ///
    /// Enforces the full verifier algorithm's structural steps: the input
    /// cap, the required tag 18, the exact protected header pin
    /// (`alg = -8`, the v2 content type, a 32-byte `kid`), the empty
    /// unprotected map, the exact two-claim payload shape, the 64-byte
    /// signature — and then **canonical-encoding strictness**: the input
    /// must be byte-identical to the canonical re-encoding of the parsed
    /// fields, so no non-canonical artifact is ever accepted even when its
    /// signature would verify.
    ///
    /// This performs no signature verification — follow with
    /// [`super::sign::verify_attestation`], or use
    /// [`super::sign::verify_attestation_bytes`] for the combined check.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::InvalidSignature`] for every rejected input —
    /// oversize, malformed CBOR, wrong shape, wrong header pins, and
    /// non-canonical encoding are deliberately indistinguishable
    /// (non-oracle).
    pub fn from_cose_bytes(bytes: &[u8]) -> TrustResult<Self> {
        let fields = encoding::decode_fields(bytes)?;
        let canonical = encoding::artifact_bytes(
            &fields.signer_public_key,
            &fields.payload_hash,
            fields.timestamp,
            &fields.signature,
        );
        if canonical != bytes {
            return Err(TrustError::InvalidSignature);
        }
        Ok(Self {
            payload_hash: fields.payload_hash,
            signature: fields.signature,
            signer_public_key: fields.signer_public_key,
            timestamp: fields.timestamp,
        })
    }
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
