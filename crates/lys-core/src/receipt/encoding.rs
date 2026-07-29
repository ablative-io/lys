//! Byte-exact CBOR/COSE encoding for the `lys/anchor-receipt/v1` artifact.
//!
//! # Invariants
//!
//! - **Encoding is hand-assembled and infallible.** Every emitted byte comes
//!   from this module's fixed-shape writers over [`crate::cbor`]'s canonical
//!   heads — RFC 8949 §4.2 core deterministic by construction, immune to any
//!   serializer dependency's encoding choices across upgrades.
//! - **Decoding of untrusted input is never hand-rolled.** [`decode_fields`]
//!   parses with `ciborium` and then enforces the exact artifact shape; the
//!   caller ([`super::artifact::AnchorReceipt::from_cose_bytes`]) additionally
//!   re-encodes the extracted fields and requires byte-identity with the
//!   input (canonical-encoding strictness).
//! - The protected header bucket is always exactly 80 bytes:
//!   `{1: -8 (EdDSA), 3: "application/vnd.lys.receipt.v1+cbor",
//!   4: <raw 32-byte Ed25519 anchor key>, 395: 1 (RFC9162_SHA256)}` in
//!   RFC 8949 §4.2 key order. Ascending numeric label order and ascending
//!   bytewise-encoded-key order coincide for `{1, 3, 4, 395}`, because all
//!   four are non-negative and shorter heads sort first.
//! - **The payload is `nil`.** The signature covers the anchor's 32-byte
//!   Merkle root as a *detached* payload, which never appears in the artifact
//!   — the verifier recomputes it. See [`super`] for why that is the point.
//! - Every decode failure collapses to
//!   [`TrustError::InvalidSignature`](crate::error::TrustError::InvalidSignature)
//!   (non-oracle; see the [`super`] module docs).
//!
//! # The vdp shape follows RFC 9942 exactly, including its wrappers
//!
//! The verifiable data proof is **not** a bare `[tree_size, leaf_index, path]`
//! array. RFC 9942's CDDL nests it twice:
//!
//! ```text
//! 396 => { -1 => [ + inclusion-proof ] }
//! inclusion-proof = bstr .cbor [ tree-size, leaf-index, inclusion-path ]
//! ```
//!
//! Both wrappers are reproduced here — the array-of-proofs at `-1`, and the
//! `bstr` that wraps each proof's CBOR. They are easy to drop by accident and
//! dropping either would make our receipts unparseable by every conforming
//! RFC 9942 implementation, which is the one outcome this format exists to
//! avoid. lys issues and accepts **exactly one** proof in that array; see
//! [`decode_fields`].

use ciborium::value::Value;

use crate::cbor::{
    MAJOR_ARRAY, MAJOR_MAP, MAJOR_TAG, MAJOR_UNSIGNED, NULL, write_bytes, write_head, write_i64,
    write_text,
};
use crate::error::{TrustError, TrustResult};

/// The `lys/anchor-receipt/v1` domain discriminator: the protected content
/// type (COSE header label 3). Signature-covered. This string is a frozen wire
/// contract — evolving the artifact means a new `v2` media type, never a
/// mutation of this one.
///
/// It is also what separates a receipt from a `lys/attestation/v2`
/// attestation, which is the same COSE shape signed by the same kind of key.
pub(crate) const CONTENT_TYPE: &str = "application/vnd.lys.receipt.v1+cbor";

/// Length of a SHA-256 digest, and so of every node in an inclusion path.
pub(crate) const DIGEST_LEN: usize = 32;

/// Hard cap on inclusion-path elements. A path over a tree of at most
/// `u64::MAX` leaves can never exceed 64 nodes, so anything longer is
/// structurally impossible rather than merely large — rejected before any
/// hashing work.
pub(crate) const MAX_PATH_ELEMENTS: usize = 64;

/// Hard input cap for [`decode_fields`]. A canonical receipt is at most
/// roughly 2.4 KiB (80-byte protected bucket, a 64-node path at 34 bytes per
/// node, a 64-byte signature); this bound is comfortably above that and
/// rejects oversize input before parsing.
pub(crate) const MAX_ARTIFACT_LEN: usize = 4096;

/// CBOR tag number for `COSE_Sign1` (RFC 9052 §2). The artifact is always
/// tagged, and the verifier requires the tag.
const COSE_SIGN1_TAG: u64 = 18;

/// COSE header label `alg`.
const HEADER_LABEL_ALG: u64 = 1;
/// COSE header label `content type`.
const HEADER_LABEL_CONTENT_TYPE: u64 = 3;
/// COSE header label `kid`.
const HEADER_LABEL_KID: u64 = 4;
/// COSE header label `vds` — verifiable data structure (RFC 9942).
const HEADER_LABEL_VDS: u64 = 395;
/// COSE header label `vdp` — verifiable data proofs (RFC 9942).
const HEADER_LABEL_VDP: u64 = 396;

/// The `alg` value: `EdDSA`.
///
/// `-8` rather than RFC 9864's preferred `-19`, deliberately and for the same
/// reason as the shipped attestation: `go-cose` ships only `-8`, and a receipt
/// no off-the-shelf library verifies is worthless. A move to `-19` is a `v2`
/// matter, triggered when the Go and Python COSE ecosystems both accept it.
const ALG_EDDSA: i64 = -8;

/// The `vds` value: `RFC9162_SHA256 = 1` — the same RFC 6962 SHA-256 tree lys
/// already implements and conformance-tests, so this is a re-encoding of
/// identical semantics rather than a new proof system.
const VDS_RFC9162_SHA256: u64 = 1;

/// The `vdp` proof-type key for an inclusion proof (RFC 9942).
const PROOF_TYPE_INCLUSION: i64 = -1;

/// Build the 80-byte protected header map
/// `{1: -8, 3: CONTENT_TYPE, 4: anchor_public_key, 395: 1}` in canonical key
/// order.
pub(crate) fn protected_bytes(anchor_public_key: &[u8; DIGEST_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(80);
    write_head(&mut out, MAJOR_MAP, 4);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_ALG);
    write_i64(&mut out, ALG_EDDSA);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_CONTENT_TYPE);
    write_text(&mut out, CONTENT_TYPE);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_KID);
    write_bytes(&mut out, anchor_public_key);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_VDS);
    write_head(&mut out, MAJOR_UNSIGNED, VDS_RFC9162_SHA256);
    out
}

/// Build the inner inclusion-proof CBOR — the value the RFC 9942
/// `bstr .cbor` wrapper carries: `[tree_size, leaf_index, [path...]]`.
fn inclusion_proof_bytes(
    tree_size: u64,
    leaf_index: u64,
    inclusion_path: &[[u8; DIGEST_LEN]],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(24 + inclusion_path.len() * (DIGEST_LEN + 2));
    write_head(&mut out, MAJOR_ARRAY, 3);
    write_head(&mut out, MAJOR_UNSIGNED, tree_size);
    write_head(&mut out, MAJOR_UNSIGNED, leaf_index);
    write_head(&mut out, MAJOR_ARRAY, inclusion_path.len() as u64);
    for node in inclusion_path {
        write_bytes(&mut out, node);
    }
    out
}

/// Build the unprotected header map `{396: {-1: [<bstr .cbor proof>]}}`.
///
/// This bucket is **not** signature-covered, and is safe there because the
/// verifier recomputes the root from it and checks the signature against that
/// recomputed value — see the [`super`] module docs.
fn unprotected_bytes(
    tree_size: u64,
    leaf_index: u64,
    inclusion_path: &[[u8; DIGEST_LEN]],
) -> Vec<u8> {
    let proof = inclusion_proof_bytes(tree_size, leaf_index, inclusion_path);
    let mut out = Vec::with_capacity(proof.len() + 16);
    write_head(&mut out, MAJOR_MAP, 1);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_VDP);
    write_head(&mut out, MAJOR_MAP, 1);
    write_i64(&mut out, PROOF_TYPE_INCLUSION);
    write_head(&mut out, MAJOR_ARRAY, 1);
    write_bytes(&mut out, &proof);
    out
}

/// Build the complete tagged `COSE_Sign1` receipt:
/// `18([protected, {396: {...}}, nil, signature])` with all-definite lengths
/// and a detached payload.
pub(crate) fn artifact_bytes(
    anchor_public_key: &[u8; DIGEST_LEN],
    tree_size: u64,
    leaf_index: u64,
    inclusion_path: &[[u8; DIGEST_LEN]],
    signature: &[u8; 64],
) -> Vec<u8> {
    let protected = protected_bytes(anchor_public_key);
    let unprotected = unprotected_bytes(tree_size, leaf_index, inclusion_path);
    let mut out = Vec::with_capacity(protected.len() + unprotected.len() + 96);
    write_head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    write_head(&mut out, MAJOR_ARRAY, 4);
    write_bytes(&mut out, &protected);
    out.extend_from_slice(&unprotected);
    out.push(NULL);
    write_bytes(&mut out, signature);
    out
}

/// The fields extracted from a structurally valid receipt.
pub(crate) struct DecodedFields {
    /// Raw 32-byte Ed25519 anchor key from the protected `kid`.
    pub(crate) anchor_public_key: [u8; DIGEST_LEN],
    /// Claimed size of the anchor's tree.
    pub(crate) tree_size: u64,
    /// Claimed index of the proven leaf.
    pub(crate) leaf_index: u64,
    /// Inclusion path, leaf-ward to root-ward.
    pub(crate) inclusion_path: Vec<[u8; DIGEST_LEN]>,
    /// 64-byte Ed25519 signature (`COSE_Sign1` item 3).
    pub(crate) signature: [u8; 64],
}

/// Non-oracle failure for every rejected receipt (see module docs).
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

/// Extract an integer value equal to `expected`, used for map-key and
/// header-value pins.
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

/// Extract a non-negative integer that fits in a `u64`.
fn unsigned(value: &Value) -> TrustResult<u64> {
    let Value::Integer(int) = value else {
        return Err(reject());
    };
    u64::try_from(i128::from(*int)).map_err(|_err| reject())
}

/// Decode the protected bucket, returning the anchor key from `kid`.
///
/// Pins `alg = -8`, the v1 receipt content type, a 32-byte `kid`, and
/// `vds = 1`, in exactly that order and with no additional entries.
fn decode_protected(protected_raw: &[u8]) -> TrustResult<[u8; DIGEST_LEN]> {
    let Value::Map(protected) = parse_value(protected_raw)? else {
        return Err(reject());
    };
    let [(alg_key, alg), (ct_key, ct), (kid_key, kid), (vds_key, vds)] = protected.as_slice()
    else {
        return Err(reject());
    };
    require_integer(alg_key, i128::from(HEADER_LABEL_ALG))?;
    require_integer(alg, i128::from(ALG_EDDSA))?;
    require_integer(ct_key, i128::from(HEADER_LABEL_CONTENT_TYPE))?;
    let Value::Text(content_type) = ct else {
        return Err(reject());
    };
    if content_type != CONTENT_TYPE {
        return Err(reject());
    }
    require_integer(kid_key, i128::from(HEADER_LABEL_KID))?;
    let anchor_public_key: [u8; DIGEST_LEN] = fixed_bytes(kid)?;
    require_integer(vds_key, i128::from(HEADER_LABEL_VDS))?;
    require_integer(vds, i128::from(VDS_RFC9162_SHA256))?;
    Ok(anchor_public_key)
}

/// The inclusion proof decoded from the unprotected `vdp` header.
struct DecodedProof {
    tree_size: u64,
    leaf_index: u64,
    inclusion_path: Vec<[u8; DIGEST_LEN]>,
}

/// Decode the unprotected bucket `{396: {-1: [<bstr .cbor proof>]}}`.
///
/// **Exactly one** inclusion proof is accepted. RFC 9942 permits an array of
/// them, but a receipt carries a single Ed25519 signature over a single root,
/// so a multi-proof receipt would need a rule for what it means when the
/// proofs disagree. Checking only the first while carrying others would be a
/// confusion attack waiting to happen: a downstream reader could act on a
/// proof the verifier never looked at. Accepting one is the conservative
/// reading, and it is what lys issues. Widening it later means deciding the
/// all-proofs-must-agree rule explicitly, which is a `v2` matter.
fn decode_unprotected(unprotected_item: &Value) -> TrustResult<DecodedProof> {
    let Value::Map(unprotected) = unprotected_item else {
        return Err(reject());
    };
    let [(vdp_key, vdp)] = unprotected.as_slice() else {
        return Err(reject());
    };
    require_integer(vdp_key, i128::from(HEADER_LABEL_VDP))?;
    let Value::Map(proofs_by_type) = vdp else {
        return Err(reject());
    };
    let [(proof_type, proofs)] = proofs_by_type.as_slice() else {
        return Err(reject());
    };
    require_integer(proof_type, i128::from(PROOF_TYPE_INCLUSION))?;
    let Value::Array(proofs) = proofs else {
        return Err(reject());
    };
    let [proof_item] = proofs.as_slice() else {
        return Err(reject());
    };
    let Value::Bytes(proof_raw) = proof_item else {
        return Err(reject());
    };

    let Value::Array(proof) = parse_value(proof_raw)? else {
        return Err(reject());
    };
    let [size_item, index_item, path_item] = proof.as_slice() else {
        return Err(reject());
    };
    let tree_size = unsigned(size_item)?;
    let leaf_index = unsigned(index_item)?;
    let Value::Array(path) = path_item else {
        return Err(reject());
    };
    if path.len() > MAX_PATH_ELEMENTS {
        return Err(reject());
    }
    let mut inclusion_path = Vec::with_capacity(path.len());
    for node in path {
        inclusion_path.push(fixed_bytes::<DIGEST_LEN>(node)?);
    }
    Ok(DecodedProof {
        tree_size,
        leaf_index,
        inclusion_path,
    })
}

/// Decode a receipt into its fields, enforcing the exact
/// `lys/anchor-receipt/v1` shape: tag 18 over a 4-array; the protected map
/// pinned to `{1: -8, 3: CONTENT_TYPE, 4: bstr(32), 395: 1}`; the unprotected
/// map pinned to a single `vdp` inclusion proof; a `nil` payload; a 64-byte
/// signature.
///
/// Canonical-encoding strictness is the caller's byte-compare — this function
/// accepts what ciborium parses.
///
/// An **empty** inclusion path is accepted here, because it is the correct
/// path for the only leaf of a tree of size 1 and refusing it would mean
/// refusing a true statement another RFC 9942 implementation may legitimately
/// make. lys itself never *issues* one (see
/// [`super::sign::sign_receipt`]), and nothing is admitted by accepting it:
/// [`crate::merkle::root_from_inclusion_path`] independently requires the path
/// length that `(leaf_index, tree_size)` demands, so an empty path
/// reconstructs a root only when `tree_size == 1`.
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
        payload_item,
        signature_item,
    ] = items.as_slice()
    else {
        return Err(reject());
    };

    // The payload must be absent: a receipt's signature covers the Merkle
    // root as a *detached* payload, so an artifact that carries any payload at
    // all is not a receipt.
    if !matches!(payload_item, Value::Null) {
        return Err(reject());
    }
    let signature: [u8; 64] = fixed_bytes(signature_item)?;

    let Value::Bytes(protected_raw) = protected_item else {
        return Err(reject());
    };
    let anchor_public_key = decode_protected(protected_raw)?;
    let proof = decode_unprotected(unprotected_item)?;

    Ok(DecodedFields {
        anchor_public_key,
        tree_size: proof.tree_size,
        leaf_index: proof.leaf_index,
        inclusion_path: proof.inclusion_path,
        signature,
    })
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
