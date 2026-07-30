//! [`AnchorReceipt`] — the parsed form of the `lys/anchor-receipt/v1` tagged
//! `COSE_Sign1` artifact.
//!
//! # Invariants
//!
//! - **The only durable form is the COSE bytes.** The struct deliberately
//!   implements no `serde` traits: nothing can persist a receipt except
//!   [`AnchorReceipt::to_cose_bytes`], so no second wire shape can ever exist
//!   alongside the frozen COSE artifact.
//! - **Round-trip identity:** `from_cose_bytes(x.to_cose_bytes()) == x` for
//!   every `AnchorReceipt` value, and `to_cose_bytes` of a parsed receipt
//!   reproduces the input bytes exactly.
//! - **The root is not a field.** The signed value is the anchor's Merkle
//!   root, carried as a *detached* payload and therefore absent from the
//!   artifact. Storing it on the struct would break round-trip identity
//!   (`from_cose_bytes` could not populate it) and, worse, would invite
//!   callers to read a root instead of recomputing one. It is derived by
//!   [`AnchorReceipt::reconstructed_root`].
//! - **Canonical-encoding strictness:** [`AnchorReceipt::from_cose_bytes`]
//!   rejects any input that is not byte-identical to the canonical re-encoding
//!   of its parsed fields — including inputs whose signature is
//!   cryptographically valid (indefinite lengths, oversized integer heads,
//!   reordered maps, extra unprotected entries, tag stripping, duplicate keys,
//!   trailing garbage).
//! - Parsing performs no signature verification and no proof check; pair
//!   `from_cose_bytes` with [`super::sign::verify_receipt`] (or use
//!   [`super::sign::verify_receipt_bytes`]) before trusting any field.
//! - Every parse failure collapses to
//!   [`TrustError::ReceiptVerification`]
//!   (non-oracle).

use crate::error::{TrustError, TrustResult};
use crate::merkle::{raw_leaf_hash, root_from_inclusion_path};
use crate::receipt::encoding::{self, DIGEST_LEN};

/// An anchor's Ed25519-signed inclusion receipt, carried on the wire as a
/// tagged `COSE_Sign1` (RFC 9052) with an RFC 9942 verifiable data proof — the
/// `lys/anchor-receipt/v1` artifact.
///
/// The receipt asserts: *"the leaf that hashes to this path's base was
/// included at index `leaf_index` in my tree of size `tree_size`, whose root I
/// vouch for."*
///
/// Constructed by [`super::sign::sign_receipt`] or parsed from artifact bytes
/// by [`Self::from_cose_bytes`]; consumed by [`super::sign::verify_receipt`].
///
/// # Which fields are signature-covered, and why the rest are still safe
///
/// Only `anchor_public_key` rides inside the signed bytes, in the protected
/// `kid` header — inheriting the fix that took attestations from v1 to v2,
/// rather than rediscovering it. The proof fields (`tree_size`, `leaf_index`,
/// `inclusion_path`) sit in the *unprotected* header, which reads as
/// "unauthenticated" and is not: they are inputs to the root reconstruction
/// whose output the signature is checked against, so altering any of them
/// yields a root the anchor never signed. They are authenticated **by
/// consequence rather than by coverage**. See the [`super`] module docs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorReceipt {
    /// Ed25519 verifying key of the issuing anchor, from the protected `kid`
    /// header — inside the signed bytes, so it cannot be swapped without
    /// invalidating `signature`.
    ///
    /// This identifies *which* anchor signed. It does not make that anchor
    /// trustworthy: a receipt verifies against whatever key it carries, so a
    /// key nobody recognises produces a perfectly valid receipt that vouches
    /// for nothing. [`super::sign::verify_receipt`] therefore requires the
    /// caller to name the anchor they expect.
    pub anchor_public_key: [u8; DIGEST_LEN],

    /// Size of the anchor's tree at the moment the root was signed.
    pub tree_size: u64,

    /// Index of the proven leaf within that tree.
    pub leaf_index: u64,

    /// RFC 6962 inclusion path from the leaf's sibling up to the root,
    /// leaf-ward first.
    pub inclusion_path: Vec<[u8; DIGEST_LEN]>,

    /// Ed25519 detached signature over the COSE `Sig_structure`
    /// `["Signature1", protected, h'', root]` (`COSE_Sign1` item 3), where
    /// `root` is the detached 32-byte Merkle root.
    pub signature: [u8; 64],
}

impl AnchorReceipt {
    /// Encode this receipt as its canonical tagged `COSE_Sign1` artifact
    /// (media type `application/cose`).
    ///
    /// Infallible: the artifact shape is fixed and the hand encoder is total
    /// over the field types. The emitted bytes verify with any off-the-shelf
    /// COSE library, given the detached root as the payload.
    pub fn to_cose_bytes(&self) -> Vec<u8> {
        encoding::artifact_bytes(
            &self.anchor_public_key,
            self.tree_size,
            self.leaf_index,
            &self.inclusion_path,
            &self.signature,
        )
    }

    /// Parse a tagged `COSE_Sign1` `lys/anchor-receipt/v1` artifact.
    ///
    /// Enforces the full structural algorithm: the input cap, the required tag
    /// 18, the exact protected header pin (`alg = -8`, the v1 receipt content
    /// type, a 32-byte `kid`, `vds = 1`), the single-inclusion-proof `vdp`
    /// shape, the `nil` payload, the 64-byte signature — and then
    /// **canonical-encoding strictness**: the input must be byte-identical to
    /// the canonical re-encoding of the parsed fields, so no non-canonical
    /// artifact is ever accepted even when its signature would verify.
    ///
    /// This performs no signature verification and no inclusion check —
    /// follow with [`super::sign::verify_receipt`], or use
    /// [`super::sign::verify_receipt_bytes`] for the combined check. A parsed
    /// receipt's fields are attacker-chosen values until then.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::ReceiptVerification`] for every rejected input —
    /// oversize, malformed CBOR, wrong shape, wrong header pins, and
    /// non-canonical encoding are deliberately indistinguishable (non-oracle).
    pub fn from_cose_bytes(bytes: &[u8]) -> TrustResult<Self> {
        let fields = encoding::decode_fields(bytes)?;
        let canonical = encoding::artifact_bytes(
            &fields.anchor_public_key,
            fields.tree_size,
            fields.leaf_index,
            &fields.inclusion_path,
            &fields.signature,
        );
        if canonical != bytes {
            return Err(TrustError::ReceiptVerification);
        }
        Ok(Self {
            anchor_public_key: fields.anchor_public_key,
            tree_size: fields.tree_size,
            leaf_index: fields.leaf_index,
            inclusion_path: fields.inclusion_path,
            signature: fields.signature,
        })
    }

    /// Recompute the Merkle root this receipt's proof implies for `leaf`.
    ///
    /// `leaf` is the raw leaf bytes — for an anchor, the notarized child's
    /// checkpoint bytes. They are hashed with
    /// [`raw_leaf_hash`]
    /// (`SHA-256(0x00 ‖ leaf)`, RFC 6962) and walked up `inclusion_path`.
    ///
    /// The result is **not** a verification result. Every well-formed path
    /// reconstructs *some* root; only comparing it against something
    /// independently trustworthy — here, the anchor's signature over it —
    /// establishes anything. Use [`super::sign::verify_receipt`] unless you
    /// are deliberately implementing that comparison yourself.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`]
    /// with an actionable reason if the receipt's `tree_size`, `leaf_index`
    /// and path length are not mutually consistent. This is the diagnostic
    /// path, and it is deliberately *not* non-oracle: it is for an operator
    /// holding their own receipt, not for a verifier judging an untrusted one.
    /// [`super::sign::verify_receipt`] collapses the same failures into its
    /// single indistinguishable error.
    pub fn reconstructed_root(&self, leaf: &[u8]) -> TrustResult<[u8; DIGEST_LEN]> {
        let leaf_hash = raw_leaf_hash(leaf);
        let flattened = self.flattened_path();
        root_from_inclusion_path(&leaf_hash, self.leaf_index, self.tree_size, &flattened)
    }

    /// The inclusion path as one contiguous buffer, the form
    /// [`root_from_inclusion_path`] consumes.
    pub(crate) fn flattened_path(&self) -> Vec<u8> {
        let mut flattened = Vec::with_capacity(self.inclusion_path.len() * DIGEST_LEN);
        for node in &self.inclusion_path {
            flattened.extend_from_slice(node);
        }
        flattened
    }
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
