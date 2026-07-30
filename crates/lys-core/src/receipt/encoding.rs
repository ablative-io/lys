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
    TrustError::ReceiptVerification
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
/// Pins `alg = -8`, `expected_content_type`, a 32-byte `kid`, and `vds = 1`, in
/// exactly that order and with no additional entries.
///
/// **`expected_content_type` is the caller's declaration of what it is parsing,
/// never a value taken from `protected_raw`.** Reading the wire's type and
/// comparing it against itself would accept anything; reading it and
/// *dispatching* on it would hand the choice to whoever wrote the artifact. The
/// content type is checked against a constant belonging to the code path that
/// asked, which is what makes a re-labelled artifact fail here rather than in
/// the signature check that would otherwise pass.
fn decode_protected(
    protected_raw: &[u8],
    expected_content_type: &str,
) -> TrustResult<[u8; DIGEST_LEN]> {
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
    if content_type != expected_content_type {
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

/// The fields extracted from a structurally valid consistency receipt.
pub(crate) struct DecodedConsistencyFields {
    /// Raw 32-byte Ed25519 anchor key from the protected `kid`.
    pub(crate) anchor_public_key: [u8; DIGEST_LEN],
    /// Claimed size of the older tree.
    pub(crate) tree_size_1: u64,
    /// Claimed size of the newer tree.
    pub(crate) tree_size_2: u64,
    /// Consistency path, in RFC 6962 §2.1.4.1 `SUBPROOF` order.
    pub(crate) consistency_path: Vec<[u8; DIGEST_LEN]>,
    /// 64-byte Ed25519 signature (`COSE_Sign1` item 3).
    pub(crate) signature: [u8; 64],
}

/// The consistency proof decoded from the unprotected `vdp` header.
struct DecodedConsistencyProof {
    tree_size_1: u64,
    tree_size_2: u64,
    consistency_path: Vec<[u8; DIGEST_LEN]>,
}

/// Decode the unprotected bucket `{396: {-2: [<bstr .cbor proof>]}}`.
///
/// **Exactly one** proof, for the same reason as [`decode_unprotected`]: one
/// signature over one derived root cannot mean anything sensible if two proofs
/// disagree.
///
/// Nothing here is trusted. Both claimed sizes are attacker-chosen until
/// [`crate::merkle::root_from_consistency_path`] has refused every ordering it
/// disallows and the derived root has satisfied the signature.
fn decode_consistency_unprotected(
    unprotected_item: &Value,
) -> TrustResult<DecodedConsistencyProof> {
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
    require_integer(proof_type, i128::from(PROOF_TYPE_CONSISTENCY))?;
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
    let [size_1_item, size_2_item, path_item] = proof.as_slice() else {
        return Err(reject());
    };
    let tree_size_1 = unsigned(size_1_item)?;
    let tree_size_2 = unsigned(size_2_item)?;
    let Value::Array(path) = path_item else {
        return Err(reject());
    };
    if path.len() > MAX_PATH_ELEMENTS {
        return Err(reject());
    }
    let mut consistency_path = Vec::with_capacity(path.len());
    for node in path {
        consistency_path.push(fixed_bytes::<DIGEST_LEN>(node)?);
    }
    Ok(DecodedConsistencyProof {
        tree_size_1,
        tree_size_2,
        consistency_path,
    })
}

/// Decode a consistency receipt into its fields, enforcing the
/// `lys/consistency-receipt/v1` shape: tag 18 over a 4-array; the protected map
/// pinned to `{1: -8, 3: CONSISTENCY_CONTENT_TYPE, 4: bstr(32), 395: 1}`; the
/// unprotected map pinned to a single `vdp` **`-2`** proof; a `nil` payload; a
/// 64-byte signature.
///
/// **An inclusion receipt cannot decode here and a consistency receipt cannot
/// decode as one**, in each case failing on the protected content type before
/// the proof is examined. That is the re-labelling refusal, and it is why the
/// two decoders pin different constants rather than sharing one that accepts
/// either.
///
/// # Errors
///
/// Every failure collapses to [`TrustError::ReceiptVerification`].
pub(crate) fn decode_consistency_fields(bytes: &[u8]) -> TrustResult<DecodedConsistencyFields> {
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
    if !matches!(payload_item, Value::Null) {
        return Err(reject());
    }
    let signature: [u8; 64] = fixed_bytes(signature_item)?;
    let Value::Bytes(protected_raw) = protected_item else {
        return Err(reject());
    };
    let anchor_public_key = decode_protected(protected_raw, CONSISTENCY_CONTENT_TYPE)?;
    let proof = decode_consistency_unprotected(unprotected_item)?;

    Ok(DecodedConsistencyFields {
        anchor_public_key,
        tree_size_1: proof.tree_size_1,
        tree_size_2: proof.tree_size_2,
        consistency_path: proof.consistency_path,
        signature,
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
/// Every failure collapses to [`TrustError::ReceiptVerification`].
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
    let anchor_public_key = decode_protected(protected_raw, CONTENT_TYPE)?;
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
