//! Byte-exact CBOR/COSE encoding for the `lys/attestation/v2` artifact.
//!
//! # Invariants
//!
//! - **Encoding is hand-assembled and infallible.** Every emitted byte is
//!   produced by this module's fixed-shape writers over [`crate::cbor`]'s
//!   canonical heads — RFC 8949 §4.2 core deterministic by construction,
//!   immune to any serializer dependency's encoding choices across upgrades.
//! - **Decoding of untrusted input is never hand-rolled.** [`decode_fields`]
//!   parses with `ciborium` and then enforces the exact artifact shape; the
//!   caller ([`super::artifact::Attestation::from_cose_bytes`]) additionally
//!   re-encodes the extracted fields and requires byte-identity with the
//!   input (canonical-encoding strictness).
//! - The protected header bucket is always exactly 80 bytes:
//!   `{1: -8 (EdDSA), 3: "application/vnd.lys.attestation.v2+cbor",
//!   4: <raw 32-byte Ed25519 signer key>}` in RFC 8949 §4.2 key order.
//! - The claims payload is always 38–46 bytes:
//!   `{1: <32-byte SHA-256 payload hash>, 2: <unix-ms timestamp>}` — the
//!   only variable part is the timestamp's canonical shortest-form int head.
//! - The complete tagged artifact is therefore always 191–199 bytes; the
//!   parser input cap is [`MAX_ARTIFACT_LEN`].
//! - Every decode failure collapses to
//!   [`TrustError::InvalidSignature`](crate::error::TrustError::InvalidSignature)
//!   (non-oracle; see the [`super`] module docs).

use ciborium::value::Value;

use crate::cbor::{
    MAJOR_ARRAY, MAJOR_MAP, MAJOR_TAG, MAJOR_UNSIGNED, write_bytes, write_head, write_i64,
    write_text,
};
use crate::error::{TrustError, TrustResult};

/// The `lys/attestation/v2` domain discriminator: the protected content type
/// (COSE header label 3). Signature-covered. This string is a frozen wire
/// contract — evolving the artifact means a new `v3` media type, never a
/// mutation of this one.
pub(crate) const CONTENT_TYPE: &str = "application/vnd.lys.attestation.v2+cbor";

/// Hard input cap for [`decode_fields`]. Canonical artifacts are 191–199
/// bytes; anything above this bound is rejected before parsing.
pub(crate) const MAX_ARTIFACT_LEN: usize = 1024;

/// CBOR tag number for `COSE_Sign1` (RFC 9052 §2). The artifact is always
/// tagged, and the verifier requires the tag.
const COSE_SIGN1_TAG: u64 = 18;

/// COSE header label `alg` and its `EdDSA` value (RFC 9053: `EdDSA = -8`,
/// the deployed-practice code point — see the design's §1.4 check).
const HEADER_LABEL_ALG: u64 = 1;
/// COSE header label `content type`.
const HEADER_LABEL_CONTENT_TYPE: u64 = 3;
/// COSE header label `kid`.
const HEADER_LABEL_KID: u64 = 4;
/// The `alg` value: `EdDSA`. Stored as the i128 the decoder compares against.
const ALG_EDDSA: i128 = -8;

/// Claims map key for the 32-byte SHA-256 payload hash.
const CLAIM_KEY_PAYLOAD_HASH: u64 = 1;
/// Claims map key for the unix-millisecond timestamp.
const CLAIM_KEY_TIMESTAMP: u64 = 2;

/// Build the 80-byte protected header map:
/// `{1: -8, 3: CONTENT_TYPE, 4: signer_public_key}` in canonical key order.
pub(crate) fn protected_bytes(signer_public_key: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(80);
    write_head(&mut out, MAJOR_MAP, 3);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_ALG);
    write_i64(&mut out, -8);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_CONTENT_TYPE);
    write_text(&mut out, CONTENT_TYPE);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_KID);
    write_bytes(&mut out, signer_public_key);
    out
}

/// Build the claims payload map:
/// `{1: payload_hash, 2: timestamp}` in canonical key order (38–46 bytes).
pub(crate) fn claims_bytes(payload_hash: &[u8; 32], timestamp: i64) -> Vec<u8> {
    let mut out = Vec::with_capacity(46);
    write_head(&mut out, MAJOR_MAP, 2);
    write_head(&mut out, MAJOR_UNSIGNED, CLAIM_KEY_PAYLOAD_HASH);
    write_bytes(&mut out, payload_hash);
    write_head(&mut out, MAJOR_UNSIGNED, CLAIM_KEY_TIMESTAMP);
    write_i64(&mut out, timestamp);
    out
}

/// Build the complete tagged `COSE_Sign1` artifact (191–199 bytes):
/// `18([protected, {}, claims, signature])` with all-definite lengths.
pub(crate) fn artifact_bytes(
    signer_public_key: &[u8; 32],
    payload_hash: &[u8; 32],
    timestamp: i64,
    signature: &[u8; 64],
) -> Vec<u8> {
    let protected = protected_bytes(signer_public_key);
    let claims = claims_bytes(payload_hash, timestamp);
    let mut out = Vec::with_capacity(199);
    write_head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    write_head(&mut out, MAJOR_ARRAY, 4);
    write_bytes(&mut out, &protected);
    write_head(&mut out, MAJOR_MAP, 0);
    write_bytes(&mut out, &claims);
    write_bytes(&mut out, signature);
    out
}

/// The four fields extracted from a structurally valid artifact.
pub(crate) struct DecodedFields {
    /// Raw 32-byte Ed25519 signer key from the protected `kid`.
    pub(crate) signer_public_key: [u8; 32],
    /// 32-byte SHA-256 payload hash from claims key `1`.
    pub(crate) payload_hash: [u8; 32],
    /// Unix-millisecond timestamp from claims key `2`.
    pub(crate) timestamp: i64,
    /// 64-byte Ed25519 signature (`COSE_Sign1` item 3).
    pub(crate) signature: [u8; 64],
}

/// Non-oracle failure for every rejected artifact (see module docs).
fn reject() -> TrustError {
    TrustError::InvalidSignature
}

/// Parse one CBOR value from `bytes` with ciborium. Trailing garbage is not
/// detected here; the caller's re-encode-and-compare gate covers it.
fn parse_value(bytes: &[u8]) -> TrustResult<Value> {
    ciborium::de::from_reader(bytes).map_err(|_err| reject())
}

/// Extract a fixed-size byte array from a CBOR bstr value.
fn fixed_bytes<const N: usize>(value: &Value) -> TrustResult<[u8; N]> {
    let Value::Bytes(bytes) = value else {
        return Err(reject());
    };
    bytes.as_slice().try_into().map_err(|_err| reject())
}

/// Extract an integer value equal to `expected`, used for map-key pins.
fn require_integer(value: &Value, expected: i128) -> TrustResult<()> {
    let Value::Integer(int) = value else {
        return Err(reject());
    };
    if i128::from(*int) == expected {
        Ok(())
    } else {
        Err(reject())
    }
}

/// Decode an artifact into its four fields, enforcing the exact
/// `lys/attestation/v2` shape (steps 1–4 of the verifier algorithm):
/// tag 18 over a 4-array; empty unprotected map; protected map exactly
/// `{1: -8, 3: CONTENT_TYPE, 4: bstr(32)}`; claims map exactly
/// `{1: bstr(32), 2: int(i64)}`; signature exactly 64 bytes.
///
/// Canonical-encoding strictness (step 5) is the caller's byte-compare —
/// this function accepts what ciborium parses.
///
/// # Errors
///
/// Every failure collapses to [`TrustError::InvalidSignature`].
pub(crate) fn decode_fields(bytes: &[u8]) -> TrustResult<DecodedFields> {
    if bytes.len() > MAX_ARTIFACT_LEN {
        return Err(reject());
    }
    let Value::Tag(COSE_SIGN1_TAG, boxed) = parse_value(bytes)? else {
        return Err(reject());
    };
    let Value::Array(items) = *boxed else {
        return Err(reject());
    };
    let [
        protected_item,
        unprotected_item,
        claims_item,
        signature_item,
    ] = items.as_slice()
    else {
        return Err(reject());
    };

    let Value::Map(unprotected) = unprotected_item else {
        return Err(reject());
    };
    if !unprotected.is_empty() {
        return Err(reject());
    }
    let signature: [u8; 64] = fixed_bytes(signature_item)?;

    let Value::Bytes(protected_raw) = protected_item else {
        return Err(reject());
    };
    let Value::Map(protected) = parse_value(protected_raw)? else {
        return Err(reject());
    };
    let [(alg_key, alg), (ct_key, ct), (kid_key, kid)] = protected.as_slice() else {
        return Err(reject());
    };
    require_integer(alg_key, i128::from(HEADER_LABEL_ALG))?;
    require_integer(alg, ALG_EDDSA)?;
    require_integer(ct_key, i128::from(HEADER_LABEL_CONTENT_TYPE))?;
    let Value::Text(content_type) = ct else {
        return Err(reject());
    };
    if content_type != CONTENT_TYPE {
        return Err(reject());
    }
    require_integer(kid_key, i128::from(HEADER_LABEL_KID))?;
    let signer_public_key: [u8; 32] = fixed_bytes(kid)?;

    let Value::Bytes(claims_raw) = claims_item else {
        return Err(reject());
    };
    let Value::Map(claims) = parse_value(claims_raw)? else {
        return Err(reject());
    };
    let [(hash_key, hash), (ts_key, ts)] = claims.as_slice() else {
        return Err(reject());
    };
    require_integer(hash_key, i128::from(CLAIM_KEY_PAYLOAD_HASH))?;
    let payload_hash: [u8; 32] = fixed_bytes(hash)?;
    require_integer(ts_key, i128::from(CLAIM_KEY_TIMESTAMP))?;
    let Value::Integer(ts_int) = ts else {
        return Err(reject());
    };
    let timestamp: i64 = (*ts_int).try_into().map_err(|_err| reject())?;

    Ok(DecodedFields {
        signer_public_key,
        payload_hash,
        timestamp,
        signature,
    })
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
