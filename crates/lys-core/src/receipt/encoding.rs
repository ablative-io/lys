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
//! - The protected header bucket is
//!   `{1: -8 (EdDSA), 3: <content type>,
//!   4: <raw 32-byte Ed25519 anchor key>, 395: 1 (RFC9162_SHA256)}` in
//!   RFC 8949 §4.2 key order — exactly 80 bytes for an inclusion receipt and
//!   92 for a consistency one, the difference being the media-type string
//!   alone. Ascending numeric label order and ascending bytewise-encoded-key
//!   order coincide for `{1, 3, 4, 395}`, because all four are non-negative
//!   and shorter heads sort first.
//! - **The content type is the caller's declaration, never the wire's.** The
//!   two receipt kinds are otherwise the same COSE shape signed by the same key
//!   over a 32-byte detached root, so the content type inside the signed bytes
//!   is the only thing that stops one being re-labelled as the other.
//! - **The payload is `nil`.** The signature covers the anchor's 32-byte
//!   Merkle root as a *detached* payload, which never appears in the artifact
//!   — the verifier recomputes it. See [`super`] for why that is the point.
//! - Every decode failure collapses to
//!   [`TrustError::ReceiptVerification`](crate::error::TrustError::ReceiptVerification)
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

use crate::cbor::{
    MAJOR_ARRAY, MAJOR_MAP, MAJOR_TAG, MAJOR_UNSIGNED, NULL, write_bytes, write_head, write_i64,
    write_text,
};

mod decode;

pub(crate) use decode::{decode_consistency_fields, decode_fields};

/// The `lys/anchor-receipt/v1` domain discriminator: the protected content
/// type (COSE header label 3). Signature-covered. This string is a frozen wire
/// contract — evolving the artifact means a new `v2` media type, never a
/// mutation of this one.
///
/// It is also what separates a receipt from a `lys/attestation/v2`
/// attestation, which is the same COSE shape signed by the same kind of key.
pub(crate) const CONTENT_TYPE: &str = "application/vnd.lys.receipt.v1+cbor";

/// The `lys/consistency-receipt/v1` domain discriminator (COSE header label 3).
///
/// **It must differ from [`CONTENT_TYPE`], and that is a security property
/// rather than a naming convention.** The two receipt kinds are the same COSE
/// shape signed by the same key over a 32-byte detached root; with a shared
/// content type their protected headers would be byte-identical, so an inclusion
/// receipt over root `R` at size `S` could be re-labelled as a consistency
/// receipt claiming `R` is the newer root at `tree_size_2 = S`, and the
/// anchor's real signature would verify over it. The proof lives in the
/// *unprotected* header, so nothing there is covered; the content type inside
/// the signed bytes is what refuses the re-label before any proof is examined.
///
/// Frozen wire contract, as for [`CONTENT_TYPE`]: evolving the artifact means a
/// new `v2` media type, never a mutation of this one.
pub(crate) const CONSISTENCY_CONTENT_TYPE: &str = "application/vnd.lys.consistency-receipt.v1+cbor";

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

/// The `vdp` proof-type key for a consistency proof (RFC 9942 §5.3.1).
///
/// **This label is in the *unprotected* header and therefore not
/// signature-covered.** It says which proof the artifact carries; it does not
/// and cannot say which kind of receipt it is. That is the content type's job —
/// see [`CONSISTENCY_CONTENT_TYPE`].
const PROOF_TYPE_CONSISTENCY: i64 = -2;

/// Build the protected header map
/// `{1: -8, 3: content_type, 4: anchor_public_key, 395: 1}` in canonical key
/// order — 80 bytes for [`CONTENT_TYPE`], 92 for [`CONSISTENCY_CONTENT_TYPE`].
///
/// **`content_type` is the caller's declaration of which artifact kind it is
/// building, and is never read from an artifact.** At verification the header is
/// re-derived through this function from the constant belonging to the code path
/// doing the verifying, so the discriminator that separates an inclusion receipt
/// from a consistency one is not attacker-supplied at all. That property is
/// exactly as strong as each call site passing *its own* constant: pass the
/// wrong one and the two signing preimages coincide, which is the whole of the
/// re-labelling attack. `the_two_receipt_kinds_sign_different_bytes` is the
/// gate on that, and it compares the buckets byte-for-byte rather than merely
/// asserting they differ — two paths drifting together would still be unequal
/// to a third.
pub(crate) fn protected_bytes(content_type: &str, anchor_public_key: &[u8; DIGEST_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(92);
    write_head(&mut out, MAJOR_MAP, 4);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_ALG);
    write_i64(&mut out, ALG_EDDSA);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_CONTENT_TYPE);
    write_text(&mut out, content_type);
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

/// Build the inner consistency-proof CBOR — the value the RFC 9942
/// `bstr .cbor` wrapper carries: `[tree_size_1, tree_size_2, [path...]]`.
///
/// Structurally the inclusion proof's twin, and deliberately written out rather
/// than shared with it: the two arrays hold three fields of the same CBOR shape
/// but *different meaning*, and a shared writer distinguished only by its
/// caller's intent is how the wrong pair of numbers gets encoded under the
/// right label.
fn consistency_proof_bytes(
    tree_size_1: u64,
    tree_size_2: u64,
    consistency_path: &[[u8; DIGEST_LEN]],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(24 + consistency_path.len() * (DIGEST_LEN + 2));
    write_head(&mut out, MAJOR_ARRAY, 3);
    write_head(&mut out, MAJOR_UNSIGNED, tree_size_1);
    write_head(&mut out, MAJOR_UNSIGNED, tree_size_2);
    write_head(&mut out, MAJOR_ARRAY, consistency_path.len() as u64);
    for node in consistency_path {
        write_bytes(&mut out, node);
    }
    out
}

/// Build the unprotected header map `{396: {-2: [<bstr .cbor proof>]}}`.
///
/// Not signature-covered, and safe there for the same reason as the inclusion
/// bucket **plus one specific to `-2`**: the verifier derives the newer root
/// from this proof *and* from an older root it supplied itself, then checks the
/// signature against the derived value. The type label `-2` is not what makes
/// the verifier run the consistency procedure — the content type in the
/// protected bucket is.
fn consistency_unprotected_bytes(
    tree_size_1: u64,
    tree_size_2: u64,
    consistency_path: &[[u8; DIGEST_LEN]],
) -> Vec<u8> {
    let proof = consistency_proof_bytes(tree_size_1, tree_size_2, consistency_path);
    let mut out = Vec::with_capacity(proof.len() + 16);
    write_head(&mut out, MAJOR_MAP, 1);
    write_head(&mut out, MAJOR_UNSIGNED, HEADER_LABEL_VDP);
    write_head(&mut out, MAJOR_MAP, 1);
    write_i64(&mut out, PROOF_TYPE_CONSISTENCY);
    write_head(&mut out, MAJOR_ARRAY, 1);
    write_bytes(&mut out, &proof);
    out
}

/// Build the complete tagged `COSE_Sign1` consistency receipt.
///
/// Identical envelope to [`artifact_bytes`] but with
/// [`CONSISTENCY_CONTENT_TYPE`] in the protected bucket — which is the entire
/// separation between the two kinds, since both carry a detached `nil` payload
/// over a 32-byte root.
pub(crate) fn consistency_artifact_bytes(
    anchor_public_key: &[u8; DIGEST_LEN],
    tree_size_1: u64,
    tree_size_2: u64,
    consistency_path: &[[u8; DIGEST_LEN]],
    signature: &[u8; 64],
) -> Vec<u8> {
    let protected = protected_bytes(CONSISTENCY_CONTENT_TYPE, anchor_public_key);
    let unprotected = consistency_unprotected_bytes(tree_size_1, tree_size_2, consistency_path);
    let mut out = Vec::with_capacity(protected.len() + unprotected.len() + 96);
    write_head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    write_head(&mut out, MAJOR_ARRAY, 4);
    write_bytes(&mut out, &protected);
    out.extend_from_slice(&unprotected);
    out.push(NULL);
    write_bytes(&mut out, signature);
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
    let protected = protected_bytes(CONTENT_TYPE, anchor_public_key);
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

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
