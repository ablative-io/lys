//! Decoding receipt artifacts: the structural parse of the envelope, the
//! protected bucket and the inclusion or consistency proof, each refusal
//! collapsed to the one non-oracle error the parent module's docs describe.

use ciborium::value::Value;

use super::{
    ALG_EDDSA, CONSISTENCY_CONTENT_TYPE, CONTENT_TYPE, COSE_SIGN1_TAG, DIGEST_LEN,
    HEADER_LABEL_ALG, HEADER_LABEL_CONTENT_TYPE, HEADER_LABEL_KID, HEADER_LABEL_VDP,
    HEADER_LABEL_VDS, MAX_ARTIFACT_LEN, MAX_PATH_ELEMENTS, PROOF_TYPE_CONSISTENCY,
    PROOF_TYPE_INCLUSION, VDS_RFC9162_SHA256,
};
use crate::error::{TrustError, TrustResult};

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
    ciborium::de::from_reader(bytes).ok().ok_or_else(reject)
}

/// Extract a fixed-size byte array from a CBOR bstr value.
fn fixed_bytes<const N: usize>(value: &Value) -> TrustResult<[u8; N]> {
    let Value::Bytes(bytes) = value else {
        return Err(reject());
    };
    bytes.as_slice().try_into().ok().ok_or_else(reject)
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
    u64::try_from(i128::from(*int)).ok().ok_or_else(reject)
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
/// [`crate::receipt::sign::sign_receipt`]), and nothing is admitted by accepting it:
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
