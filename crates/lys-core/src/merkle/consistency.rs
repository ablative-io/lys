//! Newer-root derivation from a consistency path — RFC 6962 §2.1.4.2.
//!
//! [`verify_consistency`](super::verify_consistency) answers "are these two
//! roots I already hold consistent?". This module answers the question a
//! consistency-receipt verifier must ask instead: **"given the root I held
//! before, what root does this proof say the log has grown into?"**
//!
//! The difference is not cosmetic. A verifier that must supply both roots learns
//! nothing it did not already know — it can only confirm a pair it holds in
//! full. Deriving the newer root is what lets a receipt *tell* a verifier where
//! the log got to, which is the case that matters and the reason RFC 9942
//! carries the newer root as a **detached** payload. Supplying both roots would
//! make that detachment pointless.
//!
//! # What authenticates what
//!
//! Nothing here is trusted. The derived root is an input to a signature check,
//! never a verdict — identical in shape to
//! [`root_from_inclusion_path`](super::root_from_inclusion_path). Two things
//! carry the weight:
//!
//! - **The old root is the caller's.** It is a required argument and no entry
//!   point omits it. RFC 9942 is silent on where a verifier obtains it, and if
//!   the verifier took it from the artifact then the anchor would be choosing
//!   *both* endpoints: "consistent with an earlier version of my log" would
//!   become "consistent with whichever earlier version I nominate today", which
//!   is the equivocation a consistency proof exists to detect, walking back in
//!   through the front door.
//! - **The old root is checked against the path, not assumed.** RFC 6962's
//!   algorithm reconstructs the older root as `fr` while it reconstructs the
//!   newer one as `sr`, from the same nodes. This function requires `fr` to
//!   equal the supplied old root. A path that does not descend from the
//!   caller's own earlier view is refused here rather than producing a newer
//!   root that a signature would then appear to endorse.
//!
//! # Equal sizes are refused, and that reverses an earlier ruling
//!
//! An equal-size consistency proof has an empty path and states something true:
//! *these two views are the same log.* The wire draft first ruled that issuance
//! would refuse it while verification kept accepting it, by analogy with the
//! one-leaf inclusion path (`[+ bstr]` cannot express an empty path either).
//!
//! **The analogy is wrong, and writing this function is what showed it.** For a
//! one-leaf tree the empty path still leaves a derivation standing: the root is
//! computed from the *leaf hash*, which the verifier supplies and which the
//! receipt's statement is about, so tampering still changes the result. For
//! equal sizes there is no derivation left at all — the "derived" newer root is
//! the caller's `old_root` argument, unchanged and untouched.
//!
//! That degenerates the signature check into *"has this anchor ever signed this
//! 32-byte value?"*, with the value entirely attacker-chosen. Every other input
//! shape is authenticated by consequence: alter anything and the derived root
//! stops matching what was signed. Here there is no consequence to alter, so
//! the only thing standing between an unrelated 32-byte signature and an
//! accepted consistency receipt is the content type in the protected header.
//! Domain separation should be the second line of that defence, not the only
//! one.
//!
//! Nothing is lost by refusing: a verifier that already holds `old_root` learns
//! precisely nothing from being told the log is still that size. **"Accept
//! anything true" is the right rule while the true statement carries
//! information; a vacuous statement whose acceptance degenerates a signature
//! check is not the case it was written for.**

use sha2::{Digest, Sha256};

use crate::error::{TrustError, TrustResult};

/// RFC 6962 domain-separation prefix for interior nodes.
const NODE_PREFIX: u8 = 0x01;
/// Length of a SHA-256 digest, and so of every node in a consistency path.
const DIGEST_LEN: usize = 32;

/// Derives the newer Merkle Tree Hash a consistency path implies.
///
/// Runs RFC 6962 §2.1.4.2's verification algorithm over `path`, reconstructing
/// both the older root (`fr`) and the newer one (`sr`). The older
/// reconstruction is required to equal `old_root`; the newer one is returned.
///
/// `old_root` is the root the caller *itself* previously held for the log at
/// `size_1` — never a value read out of the artifact being checked. See the
/// module docs.
///
/// The returned root is **not** a verification result. It is a candidate value
/// to be checked against something independently trustworthy, which for a
/// consistency receipt means the anchor's signature over it.
///
/// # Errors
///
/// Returns [`TrustError::MerkleTree`] if `size_1` is zero, if `size_1` is not
/// strictly below `size_2` (equal sizes are refused — see the module docs), if
/// `path` is not a whole number of 32-byte digests, if `path`'s length
/// disagrees with the one `(size_1, size_2)` requires in either direction, or
/// if the older root the path reconstructs is not `old_root`.
pub fn root_from_consistency_path(
    old_root: &[u8; DIGEST_LEN],
    size_1: u64,
    size_2: u64,
    path: &[u8],
) -> TrustResult<[u8; DIGEST_LEN]> {
    if size_1 == 0 {
        return Err(TrustError::MerkleTree {
            reason: "a consistency proof cannot start from a tree of size zero".to_string(),
        });
    }
    if size_1 >= size_2 {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "consistency requires the older size {size_1} to be strictly below the newer size {size_2}"
            ),
        });
    }
    if path.len() % DIGEST_LEN != 0 {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "consistency path must be a whole number of {DIGEST_LEN}-byte digests, got {} bytes",
                path.len()
            ),
        });
    }

    // RFC 6962 §2.1.4.2 step 1: when `size_1` is an exact power of two the older
    // root is itself a node of the newer tree, so the proof omits it and the
    // verifier prepends its own copy. That is *why* the `fr == old_root` check
    // below is not the only thing binding the result to the caller's view: in
    // this branch `old_root` is fed directly into the newer reconstruction.
    let nodes: Vec<&[u8]> = path.chunks_exact(DIGEST_LEN).collect();
    let (seed, rest): (&[u8], &[&[u8]]) = if size_1.is_power_of_two() {
        (old_root.as_slice(), nodes.as_slice())
    } else {
        let Some((first, rest)) = nodes.split_first() else {
            return Err(TrustError::MerkleTree {
                reason: "consistency path is empty but the claimed sizes require nodes".to_string(),
            });
        };
        (first, rest)
    };

    // Steps 2 and 3.
    let mut fnode = size_1 - 1;
    let mut snode = size_2 - 1;
    while fnode & 1 == 1 {
        fnode >>= 1;
        snode >>= 1;
    }

    // Step 4.
    let mut older: [u8; DIGEST_LEN] = seed.try_into().map_err(|_err| TrustError::MerkleTree {
        reason: "consistency path node is not a 32-byte digest".to_string(),
    })?;
    let mut newer = older;

    // Step 5.
    for node in rest {
        if snode == 0 {
            return Err(TrustError::MerkleTree {
                reason: "consistency path is longer than the claimed sizes allow".to_string(),
            });
        }
        if fnode & 1 == 1 || fnode == snode {
            older = node_hash(node, &older);
            newer = node_hash(node, &newer);
            while fnode != 0 && fnode & 1 == 0 {
                fnode >>= 1;
                snode >>= 1;
            }
        } else {
            newer = node_hash(&newer, node);
        }
        fnode >>= 1;
        snode >>= 1;
    }

    // Step 6. `snode != 0` means the path ran out before the newer tree was
    // spanned; a short path describes a different pair of trees, exactly as a
    // short inclusion path describes a different tree.
    if snode != 0 {
        return Err(TrustError::MerkleTree {
            reason: "consistency path is shorter than the claimed sizes require".to_string(),
        });
    }
    if older != *old_root {
        return Err(TrustError::MerkleTree {
            reason: "consistency path does not descend from the supplied older root".to_string(),
        });
    }
    Ok(newer)
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
#[path = "consistency_tests.rs"]
mod tests;
