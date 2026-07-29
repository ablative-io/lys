//! Canonical CBOR encoding primitives shared by every lys COSE artifact.
//!
//! These are the head writers for RFC 8949 §4.2 core deterministic encoding —
//! shortest-form argument, definite lengths, no floats. They live here rather
//! than in one artifact module because lys now has **two** frozen COSE wire
//! formats (`lys/attestation/v2` and `lys/anchor-receipt/v1`), and two copies
//! of a canonical head writer is how two formats quietly stop agreeing on what
//! canonical means. A change here is visible to both, and both pin their bytes
//! exhaustively in tests.
//!
//! # Invariants
//!
//! - **Encoding is total.** Every function here is infallible over its
//!   argument types; there is no encoding failure mode to handle. Canonicality
//!   is a property of the construction, not of a validation pass.
//! - **Decoding of untrusted input never comes through this module.** Parsing
//!   is `ciborium`'s job, and each artifact module then enforces its own exact
//!   shape and its own non-oracle error collapse. Sharing encode primitives is
//!   safe; sharing decode helpers would mean sharing an error type, and each
//!   artifact class deliberately collapses to a different one.

/// CBOR major types (RFC 8949 §3.1).
pub(crate) const MAJOR_UNSIGNED: u8 = 0;
pub(crate) const MAJOR_NEGATIVE: u8 = 1;
pub(crate) const MAJOR_BYTES: u8 = 2;
pub(crate) const MAJOR_TEXT: u8 = 3;
pub(crate) const MAJOR_ARRAY: u8 = 4;
pub(crate) const MAJOR_MAP: u8 = 5;
pub(crate) const MAJOR_TAG: u8 = 6;

/// The encoded simple value `null` (major type 7, value 22) — the CBOR `nil`
/// a detached `COSE_Sign1` payload is required to carry.
pub(crate) const NULL: u8 = 0xf6;

/// Append the canonical (shortest-form) CBOR head for `major`/`value`
/// (RFC 8949 §4.2.1 rule 2).
pub(crate) fn write_head(out: &mut Vec<u8>, major: u8, value: u64) {
    let major_bits = major << 5;
    if let Ok(small) = u8::try_from(value) {
        if small < 24 {
            out.push(major_bits | small);
        } else {
            out.push(major_bits | 0x18);
            out.push(small);
        }
    } else if let Ok(v) = u16::try_from(value) {
        out.push(major_bits | 0x19);
        out.extend_from_slice(&v.to_be_bytes());
    } else if let Ok(v) = u32::try_from(value) {
        out.push(major_bits | 0x1a);
        out.extend_from_slice(&v.to_be_bytes());
    } else {
        out.push(major_bits | 0x1b);
        out.extend_from_slice(&value.to_be_bytes());
    }
}

/// Append the canonical CBOR encoding of the signed 64-bit integer `value`
/// (major type 0 for `value >= 0`, major type 1 encoding `-1 - n` otherwise).
pub(crate) fn write_i64(out: &mut Vec<u8>, value: i64) {
    // Total on all of i64: `unsigned_abs` never overflows, and for negative
    // `value` the CBOR major-1 argument is `-1 - value == |value| - 1`,
    // where `|value| >= 1` so the subtraction never underflows.
    let (major, magnitude) = if value >= 0 {
        (MAJOR_UNSIGNED, value.unsigned_abs())
    } else {
        (MAJOR_NEGATIVE, value.unsigned_abs() - 1)
    };
    write_head(out, major, magnitude);
}

/// Append a canonical CBOR text string: shortest-form head, then the UTF-8
/// bytes.
pub(crate) fn write_text(out: &mut Vec<u8>, text: &str) {
    write_head(out, MAJOR_TEXT, text.len() as u64);
    out.extend_from_slice(text.as_bytes());
}

/// Append a canonical CBOR byte string: shortest-form head, then the bytes.
pub(crate) fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    write_head(out, MAJOR_BYTES, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

/// Build the `COSE_Sign1` `Sig_structure` signing preimage (RFC 9052 §4.4)
/// with an empty `external_aad`: `["Signature1", protected, h'', payload]`.
///
/// `payload` is the bytes the signature covers. For a detached payload — as in
/// `lys/anchor-receipt/v1`, where the message itself carries `nil` — these are
/// the detached bytes, which the verifier must supply or recompute rather than
/// read out of the artifact.
pub(crate) fn sig_structure_bytes(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    const CONTEXT: &str = "Signature1";
    let mut out = Vec::with_capacity(CONTEXT.len() + protected.len() + payload.len() + 8);
    write_head(&mut out, MAJOR_ARRAY, 4);
    write_text(&mut out, CONTEXT);
    write_bytes(&mut out, protected);
    write_bytes(&mut out, &[]);
    write_bytes(&mut out, payload);
    out
}

#[cfg(test)]
#[path = "cbor_tests.rs"]
mod tests;
