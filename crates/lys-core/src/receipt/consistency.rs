//! `lys/consistency-receipt/v1` — an anchor's signed statement that its log
//! grew from one view into another without rewriting history.
//!
//! Same COSE shape as [`super::artifact::AnchorReceipt`]: tagged `COSE_Sign1`,
//! `alg -8`, `kid` in the protected header, `vds 395 => 1`, and a **detached**
//! `nil` payload carrying the newer Merkle root. The proof rides in the
//! unprotected `vdp` header under type `-2` (RFC 9942 §5.3.1).
//!
//! # What is different, and why each difference is load-bearing
//!
//! **The verifier must supply the older root.** RFC 9942 does not say where a
//! verifier obtains it, and that silence is a trap: if the older root came from
//! the artifact, the anchor would be choosing *both* endpoints, and "consistent
//! with an earlier version of my log" would degrade to "consistent with
//! whichever earlier version I nominate today" — the equivocation the proof
//! exists to detect, re-entering through the front door.
//! [`verify_consistency_receipt`] therefore takes `old_root` as a required
//! argument and no entry point omits it.
//!
//! **The content type differs from the inclusion receipt's, and that is a
//! security property.** Both kinds sign a bare 32-byte root with a detached
//! payload, so with a shared content type their `Sig_structure` bytes would be
//! identical and an inclusion receipt could be re-labelled as a consistency one
//! with its signature still valid. The proof type `-1` versus `-2` lives in the
//! *unprotected* header and is free to rewrite, so it cannot be the
//! discriminator. The constant is `encoding::CONSISTENCY_CONTENT_TYPE`,
//! private because the media type is a wire fact rather than an API knob.
//!
//! **Equal sizes are refused at issuance *and* at verification.** An equal-size
//! proof has an empty path and states something true, but there is no
//! derivation left in it: the "derived" newer root would be the caller's own
//! `old_root` handed back, collapsing the signature check into *"has this anchor
//! ever signed this 32-byte value?"* over an attacker-chosen value. The refusal
//! lives in [`crate::merkle::root_from_consistency_path`], which both paths go
//! through.

use crate::cbor;
use crate::error::{TrustError, TrustResult};
use crate::keys::identity::Ed25519Identity;
use crate::merkle;

use super::encoding::{self, DIGEST_LEN, MAX_PATH_ELEMENTS};

/// An anchor's signed consistency statement between two views of its log.
///
/// Every field is attacker-chosen until [`verify_consistency_receipt`] has
/// derived a newer root from them and that root has satisfied the signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsistencyReceipt {
    /// Ed25519 verifying key of the issuing anchor, from the protected `kid`.
    ///
    /// Identifies *which* anchor signed and confers no trust: a receipt
    /// verifies against whatever key it carries, so verification requires the
    /// caller to name the anchor it expects.
    pub anchor_public_key: [u8; DIGEST_LEN],

    /// Size of the older tree the statement is made from.
    pub tree_size_1: u64,

    /// Size of the newer tree the statement is made about.
    pub tree_size_2: u64,

    /// RFC 6962 §2.1.4.1 `SUBPROOF` nodes.
    pub consistency_path: Vec<[u8; DIGEST_LEN]>,

    /// Ed25519 detached signature over the COSE `Sig_structure`
    /// `["Signature1", protected, h'', newer_root]`.
    pub signature: [u8; 64],
}

impl ConsistencyReceipt {
    /// Encode this receipt as its canonical tagged `COSE_Sign1` artifact.
    ///
    /// Infallible: the shape is fixed and the hand encoder is total over the
    /// field types.
    #[must_use]
    pub fn to_cose_bytes(&self) -> Vec<u8> {
        encoding::consistency_artifact_bytes(
            &self.anchor_public_key,
            self.tree_size_1,
            self.tree_size_2,
            &self.consistency_path,
            &self.signature,
        )
    }

    /// Parse a tagged `COSE_Sign1` `lys/consistency-receipt/v1` artifact.
    ///
    /// Enforces the input cap, tag 18, the protected header pin (including the
    /// **consistency** content type), the single `-2` proof shape, the `nil`
    /// payload and the 64-byte signature, then requires the input to be
    /// byte-identical to the canonical re-encoding of the parsed fields.
    ///
    /// **An inclusion receipt does not parse here**, failing on the protected
    /// content type before its proof is examined — that refusal is the
    /// re-labelling defence. As with [`super::AnchorReceipt::from_cose_bytes`],
    /// this stays monomorphic on purpose: naming the type *is* the caller's
    /// declaration of what it expected, and a type-sniffing entry point would
    /// hand that choice to whoever wrote the artifact.
    ///
    /// Performs no signature verification and no consistency check — follow
    /// with [`verify_consistency_receipt`].
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::ReceiptVerification`] for every rejected input;
    /// the causes are deliberately indistinguishable (non-oracle).
    pub fn from_cose_bytes(bytes: &[u8]) -> TrustResult<Self> {
        let fields = encoding::decode_consistency_fields(bytes)?;
        let canonical = encoding::consistency_artifact_bytes(
            &fields.anchor_public_key,
            fields.tree_size_1,
            fields.tree_size_2,
            &fields.consistency_path,
            &fields.signature,
        );
        if canonical != bytes {
            return Err(TrustError::ReceiptVerification);
        }
        Ok(Self {
            anchor_public_key: fields.anchor_public_key,
            tree_size_1: fields.tree_size_1,
            tree_size_2: fields.tree_size_2,
            consistency_path: fields.consistency_path,
            signature: fields.signature,
        })
    }

    /// The consistency path flattened for [`merkle::root_from_consistency_path`].
    fn flattened_path(&self) -> Vec<u8> {
        let mut flattened = Vec::with_capacity(self.consistency_path.len() * DIGEST_LEN);
        for node in &self.consistency_path {
            flattened.extend_from_slice(node);
        }
        flattened
    }
}

/// Issue a consistency receipt over an anchor's own log.
///
/// `old_root` is the root the anchor published at `tree_size_1`; the newer root
/// is *derived* from it and the path rather than passed in, so a caller cannot
/// have the anchor sign a root its own proof does not reach.
///
/// # Errors
///
/// Returns [`TrustError::MerkleTree`] if the sizes are not
/// `0 < tree_size_1 < tree_size_2`, if the path exceeds the 64-node maximum
/// any tree of up to `u64::MAX` leaves can require, or if the path does not
/// descend from `old_root`.
/// Equal sizes are refused here as well as at verification — see the module
/// docs.
pub fn sign_consistency_receipt(
    old_root: &[u8; DIGEST_LEN],
    tree_size_1: u64,
    tree_size_2: u64,
    consistency_path: &[[u8; DIGEST_LEN]],
    anchor_key: &Ed25519Identity,
) -> TrustResult<ConsistencyReceipt> {
    if consistency_path.len() > MAX_PATH_ELEMENTS {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "consistency path of {} nodes exceeds the {MAX_PATH_ELEMENTS}-node maximum any \
                 tree of up to u64::MAX leaves can require",
                consistency_path.len()
            ),
        });
    }
    let mut flattened = Vec::with_capacity(consistency_path.len() * DIGEST_LEN);
    for node in consistency_path {
        flattened.extend_from_slice(node);
    }
    // The anchor derives the root it is about to vouch for. This call is the
    // gate on the whole proof: it rejects equal or reversed sizes, a path of
    // the wrong length, and a path that does not descend from `old_root`.
    let new_root =
        merkle::root_from_consistency_path(old_root, tree_size_1, tree_size_2, &flattened)?;

    let protected = encoding::protected_bytes(
        encoding::CONSISTENCY_CONTENT_TYPE,
        &anchor_key.public_key_bytes(),
    );
    let signature = anchor_key.sign(&cbor::sig_structure_bytes(&protected, &new_root));

    Ok(ConsistencyReceipt {
        anchor_public_key: anchor_key.public_key_bytes(),
        tree_size_1,
        tree_size_2,
        consistency_path: consistency_path.to_vec(),
        signature,
    })
}

/// Verify `receipt` against the anchor the caller expects and the older root
/// the caller already held, returning the **newer root** it proves.
///
/// `old_root` is required and is the caller's own earlier view of this log. It
/// is not read from the receipt and there is deliberately no overload that
/// omits it: a verifier that accepted the anchor's word for both endpoints
/// would be checking a statement the anchor could make about any pair of trees
/// it liked.
///
/// The returned root is what the anchor actually signed, so a caller learns
/// where the log got to rather than merely confirming a pair it already knew.
///
/// # Errors
///
/// Returns [`TrustError::ReceiptVerification`] if the receipt was not issued by
/// `expected_anchor_public_key`, if the proof does not derive a root from
/// `old_root`, or if the signature does not cover the derived root. The causes
/// are deliberately indistinguishable.
pub fn verify_consistency_receipt(
    receipt: &ConsistencyReceipt,
    expected_anchor_public_key: &[u8; DIGEST_LEN],
    old_root: &[u8; DIGEST_LEN],
) -> TrustResult<[u8; DIGEST_LEN]> {
    if receipt.anchor_public_key != *expected_anchor_public_key {
        return Err(TrustError::ReceiptVerification);
    }
    // Collapse the derivation's descriptive errors: they name which structural
    // check failed, which is the right diagnostic for an operator and an oracle
    // for an attacker.
    let new_root = merkle::root_from_consistency_path(
        old_root,
        receipt.tree_size_1,
        receipt.tree_size_2,
        &receipt.flattened_path(),
    )
    .map_err(|_err| TrustError::ReceiptVerification)?;

    // Re-derived from a constant, not read from the artifact: at this point the
    // content type separating the two receipt kinds is not attacker-supplied.
    let protected = encoding::protected_bytes(
        encoding::CONSISTENCY_CONTENT_TYPE,
        &receipt.anchor_public_key,
    );
    let sig_structure = cbor::sig_structure_bytes(&protected, &new_root);
    Ed25519Identity::verify(
        &receipt.anchor_public_key,
        &sig_structure,
        &receipt.signature,
    )
    .map_err(|_err| TrustError::ReceiptVerification)?;
    Ok(new_root)
}

/// Parse and verify in one step, returning the parsed receipt and the newer
/// root it proves.
///
/// # Errors
///
/// As [`ConsistencyReceipt::from_cose_bytes`] and
/// [`verify_consistency_receipt`].
pub fn verify_consistency_receipt_bytes(
    bytes: &[u8],
    expected_anchor_public_key: &[u8; DIGEST_LEN],
    old_root: &[u8; DIGEST_LEN],
) -> TrustResult<(ConsistencyReceipt, [u8; DIGEST_LEN])> {
    let receipt = ConsistencyReceipt::from_cose_bytes(bytes)?;
    let new_root = verify_consistency_receipt(&receipt, expected_anchor_public_key, old_root)?;
    Ok((receipt, new_root))
}

#[cfg(test)]
#[path = "consistency_tests.rs"]
mod tests;
