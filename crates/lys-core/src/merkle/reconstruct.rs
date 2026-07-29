//! Root reconstruction from an inclusion path — RFC 6962 §2.1.1.
//!
//! [`verify_inclusion`](super::verify_inclusion) answers "does this proof hold
//! against a root I already have?". This module answers the different question
//! a receipt verifier must ask: **"what root does this proof imply?"**
//!
//! The distinction is the whole reason receipts can carry their inclusion proof
//! in an *unprotected* COSE header. An anchor signs the root as a detached
//! payload; the verifier recomputes a candidate root from the leaf and the path
//! and verifies the signature against *that*. A tampered path therefore yields
//! a root the anchor never signed, and the signature fails — the proof is
//! authenticated by consequence rather than by coverage. Nothing here is
//! trusted: this function's output is an input to a signature check, never a
//! verdict.
//!
//! It also means a receipt is self-verifying. A stranger holding the receipt
//! and the leaf needs nothing else — no checkpoint, no published root, no
//! service to ask. Had the root been supplied by the caller instead, the
//! receipt would only be checkable alongside a separate artifact.
//!
//! # Why this is written out rather than delegated
//!
//! `ct-merkle` verifies a proof against a supplied root and exposes no
//! root-reconstruction entry point, so the RFC 6962 walk is implemented here.
//! That creates one risk worth naming: a walk that disagreed with ct-merkle's
//! tree shape would produce roots no anchor ever signed, and *every receipt
//! ever issued* would be unverifiable. The risk is closed by test rather than
//! by care — `reconstruct_tests` builds real trees at every size from 1 to 33
//! and reconstructs from every leaf index in each, requiring the result to
//! equal the tree's own root. Any future change to ct-merkle's shape fails
//! loudly there instead of silently invalidating receipts.

use sha2::{Digest, Sha256};

use crate::error::{TrustError, TrustResult};

/// RFC 6962 domain-separation prefix for interior nodes.
const NODE_PREFIX: u8 = 0x01;
/// Length of a SHA-256 digest, and so of every node in an inclusion path.
const DIGEST_LEN: usize = 32;

/// Reconstructs the Merkle Tree Hash implied by an inclusion path.
///
/// Walks `path` upward from `leaf_hash` per RFC 6962 §2.1.1, combining nodes as
/// `SHA-256(0x01 ‖ left ‖ right)`, and returns the root the path implies for a
/// tree of `tree_size` leaves with the leaf at `leaf_index`.
///
/// `leaf_hash` is the RFC 6962 leaf hash — `SHA-256(0x00 ‖ leaf-bytes)` for raw
/// leaves, which [`raw_leaf_hash`](super::raw_leaf_hash) computes.
///
/// The returned root is **not** a verification result. It is a value to be
/// checked against something independently trustworthy — an anchor's signature
/// over it, or a published checkpoint. Treating a successful reconstruction as
/// proof of inclusion would be exactly the silent success this crate exists to
/// prevent: every well-formed path reconstructs *some* root.
///
/// # Errors
///
/// Returns [`TrustError::MerkleTree`] if `tree_size` is zero, `leaf_index` is
/// not below `tree_size`, `path` is not a whole number of 32-byte digests, or
/// the path length disagrees with the one `(leaf_index, tree_size)` requires —
/// too short and too long are both rejected, since a path of the wrong length
/// for the claimed tree describes a different tree.
pub fn root_from_inclusion_path(
    leaf_hash: &[u8; DIGEST_LEN],
    leaf_index: u64,
    tree_size: u64,
    path: &[u8],
) -> TrustResult<[u8; DIGEST_LEN]> {
    if tree_size == 0 {
        return Err(TrustError::MerkleTree {
            reason: "cannot reconstruct a root for a tree of size zero".to_string(),
        });
    }
    if leaf_index >= tree_size {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "leaf index {leaf_index} is not below the claimed tree size {tree_size}"
            ),
        });
    }
    if path.len() % DIGEST_LEN != 0 {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "inclusion path must be a whole number of {DIGEST_LEN}-byte digests, got {} bytes",
                path.len()
            ),
        });
    }

    // RFC 6962 §2.1.1, with `node` tracking the index of the current subtree
    // root within its level and `last` the index of the level's last node.
    let mut node = leaf_index;
    let mut last = tree_size - 1;
    let mut hash = *leaf_hash;

    for sibling in path.chunks_exact(DIGEST_LEN) {
        if last == 0 {
            return Err(TrustError::MerkleTree {
                reason: "inclusion path is longer than the claimed tree size allows".to_string(),
            });
        }
        // A right-hand node, or the last node of an incomplete level, combines
        // with its sibling on the left; otherwise the sibling is on the right.
        if node % 2 == 1 || node == last {
            hash = node_hash(sibling, &hash);
            // Climb past the levels this incomplete subtree spans.
            while node != 0 && node % 2 == 0 {
                node /= 2;
                last /= 2;
            }
        } else {
            hash = node_hash(&hash, sibling);
        }
        node /= 2;
        last /= 2;
    }

    if last != 0 {
        return Err(TrustError::MerkleTree {
            reason: "inclusion path is shorter than the claimed tree size requires".to_string(),
        });
    }
    Ok(hash)
}

/// Hashes an interior node as RFC 6962's `SHA-256(0x01 ‖ left ‖ right)`.
fn node_hash(left: &[u8], right: &[u8]) -> [u8; DIGEST_LEN] {
    let mut hasher = Sha256::new();
    hasher.update([NODE_PREFIX]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

#[cfg(test)]
#[path = "reconstruct_tests.rs"]
mod tests;
