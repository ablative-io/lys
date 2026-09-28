//! [`Frontier`] — the compact form of an RFC 6962 tree: the roots of its
//! perfect subtrees, largest first.
//!
//! # Why this exists
//!
//! A tree of `n` leaves decomposes into one perfect subtree per set bit of
//! `n`, and its root is those subtree roots folded from the right. That is all
//! an append needs: a new leaf merges with the rightmost subtrees of equal
//! height and the rest are untouched. So a log that keeps its frontier can
//! answer its size and root, and extend both, without holding or reading any
//! earlier leaf. A snapshot stores the frontier so a start resumes from it
//! instead of rehashing every leaf the log has ever held.
//!
//! # What it is checked against
//!
//! The frontier is not trusted for being well formed. [`Frontier::from_parts`]
//! refuses a node count that is not the number of set bits of the size, and
//! every consumer compares [`Frontier::root`] with a root it holds from
//! elsewhere (the store's pin, a snapshot's signed root) before building on
//! it. The fold here is written independently of `lys-core`'s tree, and the
//! tests hold the two to the same roots at every size they build.

use lys_core::merkle::{RootHash, raw_leaf_hash};
use sha2::{Digest, Sha256};

use crate::error::{StoreError, StoreResult};

/// RFC 6962 interior-node domain byte.
const NODE_PREFIX: u8 = 0x01;

/// The roots of the perfect subtrees of an RFC 6962 tree, largest first.
#[derive(Clone, PartialEq, Eq)]
pub struct Frontier {
    size: u64,
    nodes: Vec<[u8; 32]>,
}

impl std::fmt::Debug for Frontier {
    /// Summarizes the frontier by size and node count; the node hashes are
    /// public, but they say nothing a reader of the root does not already have.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frontier")
            .field("size", &self.size)
            .field("nodes", &self.nodes.len())
            .finish()
    }
}

impl Default for Frontier {
    fn default() -> Self {
        Self::new()
    }
}

impl Frontier {
    /// The frontier of the empty tree.
    pub fn new() -> Self {
        Self {
            size: 0,
            nodes: Vec::new(),
        }
    }

    /// A frontier from a stored size and its subtree roots, largest first.
    ///
    /// # Errors
    ///
    /// [`StoreError::FrontierMalformed`] when the number of nodes is not the
    /// number of set bits of `size`: such a list is not the frontier of any
    /// tree of that size.
    pub fn from_parts(size: u64, nodes: Vec<[u8; 32]>) -> StoreResult<Self> {
        let expected = size.count_ones();
        if usize::try_from(expected).ok() != Some(nodes.len()) {
            return Err(StoreError::FrontierMalformed {
                size,
                nodes: nodes.len(),
                expected,
            });
        }
        Ok(Self { size, nodes })
    }

    /// The frontier of the tree over `leaves`, in order.
    pub fn from_leaves<I>(leaves: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        let mut frontier = Self::new();
        for leaf in leaves {
            frontier.push(leaf.as_ref());
        }
        frontier
    }

    /// The number of leaves the frontier covers.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Whether the frontier covers no leaf.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// The subtree roots, largest first.
    pub fn nodes(&self) -> &[[u8; 32]] {
        &self.nodes
    }

    /// Appends raw leaf bytes, returning their RFC 6962 leaf hash.
    pub fn push(&mut self, leaf_bytes: &[u8]) -> [u8; 32] {
        let leaf_hash = raw_leaf_hash(leaf_bytes);
        self.push_hash(leaf_hash);
        leaf_hash
    }

    /// Appends a leaf by its RFC 6962 leaf hash.
    ///
    /// The new leaf merges with one existing subtree per trailing set bit of
    /// the old size, rightmost first, which is exactly the set of subtrees of
    /// its own height.
    pub fn push_hash(&mut self, leaf_hash: [u8; 32]) {
        let mut node = leaf_hash;
        let mut height = self.size;
        while height & 1 == 1 {
            let Some(left) = self.nodes.pop() else {
                break;
            };
            node = node_hash(&left, &node);
            height >>= 1;
        }
        self.nodes.push(node);
        self.size = self.size.saturating_add(1);
    }

    /// The RFC 6962 root of the tree the frontier covers.
    pub fn root(&self) -> [u8; 32] {
        let mut nodes = self.nodes.iter().rev();
        let Some(last) = nodes.next() else {
            return Sha256::digest([]).into();
        };
        nodes.fold(*last, |right, left| node_hash(left, &right))
    }

    /// The root with its size, in `lys-core`'s form.
    pub fn root_hash(&self) -> RootHash {
        RootHash::from_parts(self.root(), self.size)
    }
}

/// `SHA-256(0x01 ‖ left ‖ right)`, the RFC 6962 interior node.
fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([NODE_PREFIX]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

#[cfg(test)]
#[path = "frontier_tests.rs"]
mod tests;
