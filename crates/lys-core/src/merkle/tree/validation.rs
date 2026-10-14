//! Proof arguments are checked before either tree constructs a path.

use crate::error::{TrustError, TrustResult};

/// Refuses an inclusion proof for an index outside a tree of `len` leaves.
pub(super) fn check_inclusion_index(len: u64, leaf_index: u64) -> TrustResult<()> {
    if leaf_index >= len {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "inclusion proof requested for leaf index {leaf_index} but tree has {len} leaves"
            ),
        });
    }
    Ok(())
}

/// Refuses a consistency proof unless `0 < old_size <= new_size == len`.
pub(super) fn check_consistency_sizes(len: u64, old_size: u64, new_size: u64) -> TrustResult<()> {
    if new_size != len {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "consistency proof requires new_size to equal current tree length: \
                 new_size={new_size}, current_len={len}"
            ),
        });
    }
    if old_size == 0 {
        return Err(TrustError::MerkleTree {
            reason: "consistency proof requires old_size > 0; \
                     RFC 6962 has no consistency proof from an empty tree"
                .to_string(),
        });
    }
    if old_size > new_size {
        return Err(TrustError::MerkleTree {
            reason: format!(
                "consistency proof old_size must be <= new_size: \
                 old_size={old_size}, new_size={new_size}"
            ),
        });
    }
    Ok(())
}
