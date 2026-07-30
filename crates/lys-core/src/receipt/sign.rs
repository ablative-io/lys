//! Issue and verify `lys/anchor-receipt/v1` `COSE_Sign1` inclusion receipts.
//!
//! [`sign_receipt`] takes the leaf the anchor notarized and the RFC 6962
//! inclusion proof for it, **recomputes the Merkle root itself**, and signs
//! that root as a detached COSE payload:
//!
//! ```text
//! Sig_structure = ["Signature1", protected, h'', root]
//! protected     = {1: -8 (EdDSA), 3: <v1 receipt content type>,
//!                  4: <anchor key>, 395: 1 (RFC9162_SHA256)}
//! root          = SHA-256 Merkle Tree Hash reconstructed from leaf + path
//! ```
//!
//! The anchor never signs a caller-supplied root. It signs a value it derived,
//! which is the difference between vouching for a tree it computed and
//! rubber-stamping a number someone handed it — **a claim must not be settable
//! by the party being judged.**
//!
//! [`verify_receipt`] reverses it: recompute the leaf hash, walk the path to a
//! candidate root, and verify the anchor's signature against *that*. Any
//! mismatch collapses to [`TrustError::ReceiptVerification`].
//!
//! # Verifying requires naming the anchor you expect
//!
//! There is deliberately no way to "just verify" a receipt. A receipt carries
//! the anchor's key in its own protected header, so it verifies against
//! whatever key it carries — a receipt signed by a key nobody recognises is
//! cryptographically perfect and vouches for nothing. This is the same trap as
//! a self-signed certificate: the signature proves someone holding a key
//! *said* something, never that the statement is true.
//!
//! So [`verify_receipt`] requires the caller to pass the anchor key they
//! trust, and enforces the match. Callers who genuinely want to inspect an
//! unattributed receipt use [`AnchorReceipt::from_cose_bytes`], which is
//! honestly labelled as parsing rather than verification.
//!
//! # Domain separation from attestations
//!
//! A receipt's signing preimage begins `0x84 0x6A "Signature1"` — **identical**
//! to a `lys/attestation/v2` attestation's, because both are `COSE_Sign1`. The
//! byte-0 disjointness argument that separates attestations from signed notes
//! and X.509 TBS bytes therefore does *not* separate receipts from
//! attestations, and it would be a mistake to assume it does.
//!
//! What separates them is inside the signed bytes, in two independent places:
//!
//! - **The protected bucket differs.** An attestation's is a 3-entry map whose
//!   content type is `application/vnd.lys.attestation.v2+cbor`; a receipt's is
//!   a 4-entry map whose content type is
//!   `application/vnd.lys.receipt.v1+cbor` and which carries `vds`. Different
//!   bytes at the same offset means different signed messages, so no signature
//!   is ever valid under both readings.
//! - **The payload differs in kind and length.** An attestation signs a
//!   two-entry claims map (38–46 bytes); a receipt signs a bare 32-byte root.
//!
//! Both artifact verifiers additionally *pin* their own content type, so
//! neither will parse the other at all. Separation is by construction and by
//! enforcement, not by luck.
//!
//! # Known limitation: `tree_size` is authenticated only up to its walk class
//!
//! Because the proof rides in the unprotected header and is authenticated by
//! consequence, `tree_size` is pinned only as far as it changes the
//! reconstruction. It does not always change it. For `leaf_index = 0`, a tree
//! of size 3 and a tree of size 4 produce the *same* sequence of
//! left/right combinations, so one valid receipt can be re-presented with the
//! other size: same leaf, same index, same root, same signature, different
//! claimed size. `receipt::sign_tests` proves this rather than leaving it to
//! be discovered.
//!
//! This is not a forgery and not a false inclusion claim — the leaf, the
//! index and the root are all unaffected, and the information needed to
//! distinguish the two sizes is simply not present in an RFC 6962 inclusion
//! path. It is why RFC 9162 verifiers are told to check a proof against a
//! *known* root at a *known* size. A consumer who relies on `tree_size` for
//! anything load-bearing must cross-check it against the anchor's published
//! checkpoint at that size, where the mismatch shows up immediately.
//!
//! Fixing it in the format would mean moving the proof into the protected
//! header or signing `root ‖ tree_size`, and both break RFC 9942 conformance
//! — a high price for a low-impact, externally-detectable malleability. The
//! choice is to conform and document.

use crate::error::{TrustError, TrustResult};
use crate::keys::identity::Ed25519Identity;
use crate::merkle::raw_leaf_hash;
use crate::receipt::artifact::AnchorReceipt;
use crate::receipt::encoding::{self, DIGEST_LEN, MAX_PATH_ELEMENTS};
use crate::{cbor, merkle};

/// Issue a receipt for `leaf` at `leaf_index` in a tree of `tree_size`,
/// signing the reconstructed Merkle root with the anchor's key.
///
/// `leaf` is the raw leaf bytes as logged — for an anchor notarizing a child
/// log, the child's checkpoint bytes, which are self-describing (origin, size,
/// root). That is why no separate binding between receipt and child is needed:
/// verifying the receipt establishes *which* child root was notarized.
///
/// `inclusion_path` is the RFC 6962 path for `leaf_index` in a tree of
/// `tree_size`, leaf-ward first — exactly what
/// [`AppendOnlyTree`](crate::merkle::AppendOnlyTree) produces.
///
/// # Errors
///
/// Returns [`TrustError::MerkleTree`] with an actionable reason if the proof is
/// not self-consistent — a path whose length disagrees with
/// `(leaf_index, tree_size)`, an index outside the tree, an empty tree, or a
/// path longer than any `u64`-sized tree could require. Errors here are
/// deliberately descriptive rather than non-oracle: this is the anchor
/// operating on its own tree, no secret participates in the outcome, and the
/// caller already knows every input.
///
/// `tree_size == 1` is refused. RFC 9942's CDDL types the inclusion path as
/// `[+ bstr]` — *one or more* — and the sole leaf of a one-leaf tree has an
/// empty path, so a receipt for it could not be a conforming artifact. The
/// remedy is operational and cheap: seed the anchor's log with a genesis leaf
/// at initialisation, after which `tree_size >= 2` always holds and every path
/// has at least one node. Emitting a technically non-conforming *first*
/// receipt would be the worst possible one to get wrong, since the earliest
/// artifacts are the ones others reach for as interop test vectors.
pub fn sign_receipt(
    leaf: &[u8],
    leaf_index: u64,
    tree_size: u64,
    inclusion_path: &[[u8; DIGEST_LEN]],
    anchor_key: &Ed25519Identity,
) -> TrustResult<AnchorReceipt> {
    if tree_size == 1 {
        return Err(TrustError::MerkleTree {
            reason: "cannot issue a conforming receipt for a tree of size 1: RFC 9942 types the \
                     inclusion path as one-or-more nodes and a one-leaf tree's path is empty — \
                     seed the log with a genesis leaf so tree size is never below 2"
                .to_string(),
        });
    }
    if inclusion_path.len() > MAX_PATH_ELEMENTS {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "inclusion path of {} nodes exceeds the {MAX_PATH_ELEMENTS}-node maximum any \
                 tree of up to u64::MAX leaves can require",
                inclusion_path.len()
            ),
        });
    }

    let leaf_hash = raw_leaf_hash(leaf);
    let mut flattened = Vec::with_capacity(inclusion_path.len() * DIGEST_LEN);
    for node in inclusion_path {
        flattened.extend_from_slice(node);
    }
    // The anchor derives the root it is about to vouch for. This call is the
    // consistency gate on the whole proof: it rejects an index outside the
    // tree and a path length that disagrees with the claimed shape.
    let root = merkle::root_from_inclusion_path(&leaf_hash, leaf_index, tree_size, &flattened)?;

    let protected = encoding::protected_bytes(&anchor_key.public_key_bytes());
    let signature = anchor_key.sign(&cbor::sig_structure_bytes(&protected, &root));

    Ok(AnchorReceipt {
        anchor_public_key: anchor_key.public_key_bytes(),
        tree_size,
        leaf_index,
        inclusion_path: inclusion_path.to_vec(),
        signature,
    })
}

/// Verify that `receipt` is a valid inclusion receipt for `leaf`, issued by
/// the anchor whose key is `expected_anchor_public_key`.
///
/// Three things must hold, and all three failures are indistinguishable:
///
/// 1. The receipt names the expected anchor. Without this the check would
///    prove only that *someone* signed, which is not a trust statement — see
///    the module docs.
/// 2. The proof is internally consistent, so a root can be reconstructed from
///    `leaf` and the path at the claimed index and size.
/// 3. The anchor's signature verifies over that **reconstructed** root. This
///    is what authenticates the unprotected proof: a tampered path, index or
///    size yields a root the anchor never signed.
///
/// Verification uses strict Ed25519, rejecting malleable signatures and
/// small-order keys — non-repudiation requires a unique valid signature.
///
/// # Errors
///
/// Returns [`TrustError::ReceiptVerification`] for every failure. A receipt from
/// the wrong anchor, an inconsistent proof, a tampered path and a forged
/// signature are deliberately indistinguishable: a receipt verifier is exactly
/// the network-exposed surface where a distinguishable error becomes a parsing
/// oracle. Use [`AnchorReceipt::reconstructed_root`] when diagnosing a receipt
/// you own.
pub fn verify_receipt(
    receipt: &AnchorReceipt,
    leaf: &[u8],
    expected_anchor_public_key: &[u8; DIGEST_LEN],
) -> TrustResult<()> {
    if receipt.anchor_public_key != *expected_anchor_public_key {
        return Err(TrustError::ReceiptVerification);
    }
    let leaf_hash = raw_leaf_hash(leaf);
    let flattened = receipt.flattened_path();
    // Collapse the reconstruction's descriptive errors: they name which
    // structural check failed, which is the right diagnostic for an operator
    // and an oracle for an attacker.
    let root = merkle::root_from_inclusion_path(
        &leaf_hash,
        receipt.leaf_index,
        receipt.tree_size,
        &flattened,
    )
    .map_err(|_err| TrustError::ReceiptVerification)?;

    let protected = encoding::protected_bytes(&receipt.anchor_public_key);
    let sig_structure = cbor::sig_structure_bytes(&protected, &root);
    // Mapped, not propagated: `verify` reports `InvalidSignature`, and letting
    // that through would make a forged signature distinguishable from every
    // other way a receipt fails. It was indistinguishable only while receipts
    // reused `InvalidSignature` as their own collapse value — the moment they
    // stopped, this became a leak. `every_verification_failure_is_the_same_error`
    // caught it.
    Ed25519Identity::verify(
        &receipt.anchor_public_key,
        &sig_structure,
        &receipt.signature,
    )
    .map_err(|_err| TrustError::ReceiptVerification)
}

/// Parse a tagged `COSE_Sign1` receipt and verify it against `leaf` and the
/// expected anchor in one step, returning the parsed [`AnchorReceipt`] — the
/// bytes-in convenience for consumers holding raw artifact bytes.
///
/// Equivalent to [`AnchorReceipt::from_cose_bytes`] followed by
/// [`verify_receipt`].
///
/// # Errors
///
/// Returns [`TrustError::ReceiptVerification`] for every failure — a malformed or
/// non-canonical artifact, the wrong anchor, an inconsistent proof and an
/// invalid signature are deliberately indistinguishable (non-oracle).
pub fn verify_receipt_bytes(
    cose: &[u8],
    leaf: &[u8],
    expected_anchor_public_key: &[u8; DIGEST_LEN],
) -> TrustResult<AnchorReceipt> {
    let receipt = AnchorReceipt::from_cose_bytes(cose)?;
    verify_receipt(&receipt, leaf, expected_anchor_public_key)?;
    Ok(receipt)
}

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;
