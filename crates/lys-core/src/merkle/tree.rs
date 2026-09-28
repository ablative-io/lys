//! [`AppendOnlyTree`] — the append-only Merkle log over a SHA-256
//! `ct_merkle::MemoryBackedTree`.
//!
//! Only [`AppendOnlyTree::append`] mutates the tree. There is no API that
//! deletes, replaces, reorders, or otherwise touches past leaves; the type
//! exists precisely to make the append-only invariant a property of the
//! public surface, not just of the underlying RFC 6962 construction.
//!
//! ct-merkle's `prove_inclusion` and `prove_consistency` panic on
//! out-of-range arguments. The wrapper methods pre-check the arguments and
//! return [`TrustError::MerkleTree`] instead, keeping the no-panic invariant
//! of this crate intact.
//!
//! The tree is generic over the leaf type `L: Serialize`. Leaves are
//! converted to a deterministic byte representation via `serialize_leaf`
//! before being pushed into the underlying ct-merkle tree, so the original
//! `L` is not stored on the tree and the trust crate stays domain-agnostic.
//!
//! The [`RawLeaf`] marker selects the parallel raw-byte encoding:
//! `AppendOnlyTree<RawLeaf>` hashes leaf bytes verbatim
//! (`SHA-256(0x00 ‖ bytes)` per RFC 6962) via [`AppendOnlyTree::append_raw`]
//! and never gains the postcard methods, so the two leaf encodings cannot
//! be mixed in one tree.
//!
//! [`HashTree`] is the raw-leaf tree without the leaves: it holds only the
//! RFC 6962 leaf and interior hashes, is fed leaf hashes rather than bytes,
//! and proves inclusion and consistency byte-identically to
//! `AppendOnlyTree<RawLeaf>` over the same leaves. It is the tree for a log
//! whose leaves live in storage: its memory is at most two hashes per leaf,
//! and no leaf is hashed by it.

use std::fmt;
use std::marker::PhantomData;

use ct_merkle::mem_backed_tree::MemoryBackedTree;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{TrustError, TrustResult};
use crate::merkle::leaf::{SerializedLeaf, serialize_leaf};
use crate::merkle::proof::{ConsistencyProof, InclusionProof, RootHash};

/// Append-only Merkle transparency log over any `L: Serialize` leaf.
///
/// Each [`Self::append`] serializes the leaf with `postcard` and pushes the
/// resulting bytes into a `ct_merkle::MemoryBackedTree<Sha256, _>`. The
/// original `L` is not stored; the tree retains the serialized bytes plus
/// the internal Merkle nodes.
///
/// The empty tree's root is RFC 6962's deterministic zero-leaf root —
/// `SHA-256("")` paired with `num_leaves = 0`. After `n` appends, [`Self::root`]
/// returns the SHA-256 Merkle Tree Hash of the `n` leaves with `num_leaves =
/// n`. Two trees built from the same `Serialize` sequence in the same order
/// produce the same root.
///
/// # The leaf encoding is a FROZEN WIRE CONTRACT
///
/// Leaves are hashed via their `postcard` encoding, which is derived
/// entirely from `L`'s shape: fields, field declaration order, and enum
/// variant declaration order. Once leaves of `L` exist in a persisted or
/// published tree, that shape must **never** change — reordering or adding
/// fields, or reordering enum variants, silently alters the bytes of
/// historical leaves, so every previously published root and proof stops
/// verifying with no error and no version signal. Schema evolution
/// requires a new versioned leaf type or an explicitly versioned envelope.
/// When long-lived verifiability matters, prefer a leaf type that pins the
/// payload as pre-encoded bytes (e.g. a struct holding `Vec<u8>`) so the
/// hashed bytes are under explicit consumer control. See the
/// [`leaf`](super::leaf) module docs for the full contract.
pub struct AppendOnlyTree<L> {
    inner: MemoryBackedTree<Sha256, SerializedLeaf>,
    // `fn(L)` is the standard "leaf-type tag" marker — it tracks `L` in the
    // type signature without imposing variance constraints on the wrapper
    // and without requiring `L: Debug`/`Clone` on the wrapper's derives.
    _marker: PhantomData<fn(L)>,
}

impl<L> AppendOnlyTree<L> {
    /// Builds a new empty tree.
    ///
    /// The empty tree's [`Self::root`] is the deterministic zero-leaf root
    /// (`SHA-256("")` with `num_leaves = 0`) defined by RFC 6962 and
    /// implemented by ct-merkle.
    pub fn new() -> Self {
        Self {
            inner: MemoryBackedTree::new(),
            _marker: PhantomData,
        }
    }

    /// Returns the number of leaves in the tree.
    pub fn len(&self) -> u64 {
        self.inner.len()
    }

    /// Returns `true` if no leaves have been appended.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the current root hash, capturing both the SHA-256 Merkle Tree
    /// Hash and the current leaf count.
    pub fn root(&self) -> RootHash {
        RootHash::from_inner(self.inner.root())
    }

    /// Builds an inclusion proof for the leaf at `leaf_index`.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] if `leaf_index` is greater than or
    /// equal to the current tree length, or if the index does not fit in a
    /// `usize` on this target. Both conditions are pre-checked so the
    /// underlying ct-merkle call (which would otherwise panic) is never made
    /// with an invalid argument.
    pub fn prove_inclusion(&self, leaf_index: u64) -> TrustResult<InclusionProof> {
        check_inclusion_index(self.inner.len(), leaf_index)?;
        let idx = usize::try_from(leaf_index).map_err(|_err| TrustError::MerkleTree {
            reason: format!("inclusion proof leaf index {leaf_index} does not fit in usize"),
        })?;
        Ok(InclusionProof::from_inner(self.inner.prove_inclusion(idx)))
    }

    /// Builds a consistency proof showing that the tree of size `old_size` is
    /// a prefix of the current tree (which must be exactly `new_size` leaves).
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] if `new_size` does not equal the
    /// current tree length, `old_size` is zero, `old_size` exceeds `new_size`,
    /// or the resulting `num_additions` does not fit in a `usize`. These
    /// checks ensure the underlying ct-merkle call (which would otherwise
    /// panic when `num_additions >= self.len()`) is never made with an
    /// invalid argument.
    pub fn prove_consistency(&self, old_size: u64, new_size: u64) -> TrustResult<ConsistencyProof> {
        check_consistency_sizes(self.inner.len(), old_size, new_size)?;
        // old_size > 0 and old_size <= new_size = self.len(), so this never
        // wraps around.
        let num_additions = new_size - old_size;
        let num_additions_usize =
            usize::try_from(num_additions).map_err(|_err| TrustError::MerkleTree {
                reason: format!(
                    "consistency proof num_additions {num_additions} does not fit in usize"
                ),
            })?;
        Ok(ConsistencyProof::from_inner(
            self.inner.prove_consistency(num_additions_usize),
        ))
    }
}

/// Type-level marker for trees whose leaves are raw bytes hashed verbatim
/// (leaf hash = `SHA-256(0x00 ‖ bytes)` per RFC 6962).
///
/// Uninhabited: never a value, and it deliberately does NOT implement
/// `Serialize`, so the postcard methods ([`AppendOnlyTree::append`],
/// [`AppendOnlyTree::reconstruct_from_leaves`]) do not exist on
/// `AppendOnlyTree<RawLeaf>` — the two leaf encodings cannot be mixed in
/// one tree even by accident.
///
/// **Invariant:** for every leaf appended via
/// [`AppendOnlyTree::append_raw`],
/// `leaf_hash = SHA-256(0x00 ‖ leaf-bytes)`; a third party reproduces it
/// with `(printf '\x00'; cat leaf-file) | shasum -a 256`.
pub enum RawLeaf {}

impl AppendOnlyTree<RawLeaf> {
    /// Appends raw bytes verbatim and returns the new tree size.
    ///
    /// The bytes are hashed exactly as supplied — no postcard, no length
    /// prefix — so the RFC 6962 leaf hash is `SHA-256(0x00 ‖ leaf_bytes)`
    /// (see [`raw_leaf_hash`](super::leaf::raw_leaf_hash)). Infallible:
    /// there is no serialization step to fail.
    pub fn append_raw(&mut self, leaf_bytes: &[u8]) -> u64 {
        self.inner
            .push(SerializedLeaf::from_raw_bytes(leaf_bytes.to_vec()));
        self.inner.len()
    }

    /// Rebuilds a raw-leaf tree from leaves in their original append order.
    ///
    /// The reconstruction path for restart and prefix rebuilds, mirroring
    /// [`AppendOnlyTree::reconstruct_from_leaves`] for the raw encoding.
    /// Because it reuses [`Self::append_raw`], the rebuilt tree reproduces
    /// the original root bit-for-bit.
    pub fn reconstruct_from_raw_leaves<I>(leaves: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        let mut tree = Self::new();
        for leaf in leaves {
            tree.append_raw(leaf.as_ref());
        }
        tree
    }
}

impl<L: Serialize> AppendOnlyTree<L> {
    /// Appends `leaf` to the tree and returns the new tree size.
    ///
    /// The leaf is serialized via `postcard` before being hashed into the
    /// tree. The first append returns `1`, the next `2`, and so on.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] if the leaf cannot be serialized.
    /// No other failure modes exist at this scale of tree (ct-merkle's
    /// internal limits are well beyond practical use).
    pub fn append(&mut self, leaf: L) -> TrustResult<u64> {
        let serialized = serialize_leaf(&leaf)?;
        self.inner.push(serialized);
        Ok(self.inner.len())
    }

    /// Rebuilds a tree from a sequence of leaves in their original append
    /// order.
    ///
    /// This is the mechanism for reconstructing in-memory tree state from
    /// persistent storage on restart. Because reconstruction reuses
    /// [`Self::append`], it goes through the same serialization path and
    /// therefore reproduces the original root hash bit-for-bit.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] if any leaf fails to serialize.
    pub fn reconstruct_from_leaves(leaves: Vec<L>) -> TrustResult<Self> {
        let mut tree = Self::new();
        for leaf in leaves {
            tree.append(leaf)?;
        }
        Ok(tree)
    }
}

impl<L> Default for AppendOnlyTree<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L> fmt::Debug for AppendOnlyTree<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppendOnlyTree")
            .field("num_leaves", &self.inner.len())
            .finish()
    }
}

/// Refuses an inclusion proof for an index outside a tree of `len` leaves.
fn check_inclusion_index(len: u64, leaf_index: u64) -> TrustResult<()> {
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
fn check_consistency_sizes(len: u64, old_size: u64, new_size: u64) -> TrustResult<()> {
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

/// RFC 6962 interior-node domain byte.
const NODE_PREFIX: u8 = 0x01;

/// RFC 6962 interior node: `SHA-256(0x01 ‖ left ‖ right)`.
fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([NODE_PREFIX]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// RFC 6962's split of `n >= 2` leaves: the largest power of two below `n`.
fn split_point(size: u64) -> u64 {
    1 << (u64::BITS - 1 - (size - 1).leading_zeros())
}

/// An append-only RFC 6962 tree that holds hashes and nothing else.
///
/// It is fed leaf hashes, `SHA-256(0x00 ‖ leaf-bytes)` computed by whoever
/// read the leaf, and never sees a leaf's bytes: a log builds it by streaming
/// its stored leaves through, one at a time, and extends it on append with the
/// leaf hash it already computed, so no leaf is hashed twice.
///
/// # Invariants
///
/// - Level `h` holds the root of every complete perfect subtree of height `h`,
///   left to right: entry `i` is the hash of leaves `i·2^h .. (i+1)·2^h`.
///   Level 0 is the leaf hashes. So a tree of `n` leaves holds
///   `2n − popcount(n)` hashes, fewer than two per leaf.
/// - Every RFC 6962 subtree `MTH(D[a:b])` is either one of those perfect
///   subtrees or the hash of a perfect left subtree and a smaller right one,
///   so roots and proofs are computed from the stored hashes alone, and are
///   byte-identical to a raw-leaf [`AppendOnlyTree`]'s over the same leaves.
/// - Nothing is removed or replaced: [`Self::push_leaf_hash`] is the only
///   mutation.
#[derive(Clone, Default)]
pub struct HashTree {
    size: u64,
    levels: Vec<Vec<[u8; 32]>>,
}

impl HashTree {
    /// Builds a new empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds an empty tree with room for `leaves` leaves and their interior
    /// hashes, so a tree streamed to a known size allocates once per level and
    /// holds no spare capacity. A size beyond this target's address space
    /// reserves nothing and the tree grows as it is fed.
    pub fn with_capacity(leaves: u64) -> Self {
        let mut levels = Vec::new();
        let mut width = leaves;
        while width > 0 {
            levels.push(Vec::with_capacity(
                usize::try_from(width).unwrap_or_default(),
            ));
            width >>= 1;
        }
        Self { size: 0, levels }
    }

    /// Returns the number of leaves in the tree.
    pub fn len(&self) -> u64 {
        self.size
    }

    /// Returns `true` if no leaf has been appended.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// The bytes of hash storage the tree holds, spare capacity included.
    /// Never more than two 32-byte hashes per leaf for a tree built by
    /// [`Self::with_capacity`] to its size.
    pub fn retained_bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|level| level.capacity() * std::mem::size_of::<[u8; 32]>())
            .sum()
    }

    /// Appends a leaf by its RFC 6962 leaf hash and returns the new tree size.
    ///
    /// The new leaf completes one perfect subtree per trailing set bit of the
    /// old size; each is hashed once, here, and kept.
    pub fn push_leaf_hash(&mut self, leaf_hash: [u8; 32]) -> u64 {
        let mut node = leaf_hash;
        let mut height = 0;
        loop {
            if self.levels.len() == height {
                self.levels.push(Vec::new());
            }
            let Some(level) = self.levels.get_mut(height) else {
                break;
            };
            level.push(node);
            if level.len() % 2 == 1 {
                break;
            }
            let [.., left, right] = level.as_slice() else {
                break;
            };
            node = node_hash(left, right);
            height += 1;
        }
        self.size = self.size.saturating_add(1);
        self.size
    }

    /// Returns the current root hash with the current leaf count.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] if a hash the tree must hold is
    /// missing, which only a broken invariant can cause.
    pub fn root(&self) -> TrustResult<RootHash> {
        let root = if self.size == 0 {
            Sha256::digest([]).into()
        } else {
            self.subtree(0, self.size)?
        };
        Ok(RootHash::from_parts(root, self.size))
    }

    /// Builds the RFC 6962 inclusion proof `PATH(leaf_index, D[n])`.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] if `leaf_index` is not below the
    /// tree length, with the same reason [`AppendOnlyTree::prove_inclusion`]
    /// gives.
    pub fn prove_inclusion(&self, leaf_index: u64) -> TrustResult<InclusionProof> {
        check_inclusion_index(self.size, leaf_index)?;
        let mut path = Vec::new();
        let (mut start, mut size, mut index) = (0, self.size, leaf_index);
        while size > 1 {
            let split = split_point(size);
            if index < split {
                path.push(self.subtree(start + split, size - split)?);
                size = split;
            } else {
                path.push(self.subtree(start, split)?);
                start += split;
                index -= split;
                size -= split;
            }
        }
        InclusionProof::try_from_bytes(path.iter().rev().flatten().copied().collect())
    }

    /// Builds the RFC 6962 consistency proof `PROOF(old_size, D[new_size])`.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::MerkleTree`] unless `0 < old_size <= new_size`
    /// and `new_size` is the tree length, with the same reasons
    /// [`AppendOnlyTree::prove_consistency`] gives.
    pub fn prove_consistency(&self, old_size: u64, new_size: u64) -> TrustResult<ConsistencyProof> {
        check_consistency_sizes(self.size, old_size, new_size)?;
        let mut path = Vec::new();
        let (mut start, mut size, mut old, mut whole) = (0, new_size, old_size, true);
        while old != size {
            let split = split_point(size);
            if old <= split {
                path.push(self.subtree(start + split, size - split)?);
                size = split;
            } else {
                path.push(self.subtree(start, split)?);
                start += split;
                old -= split;
                size -= split;
                whole = false;
            }
        }
        if !whole {
            path.push(self.subtree(start, size)?);
        }
        ConsistencyProof::try_from_bytes(path.iter().rev().flatten().copied().collect())
    }

    /// `MTH(D[start : start + size])` for a subtree RFC 6962's recursion
    /// reaches, whose `start` is a multiple of the largest power of two not
    /// above `size`.
    fn subtree(&self, start: u64, size: u64) -> TrustResult<[u8; 32]> {
        if size.is_power_of_two() {
            let height = size.trailing_zeros();
            let node = if start % size == 0 {
                self.stored(height, start >> height)
            } else {
                None
            };
            return node.ok_or_else(|| TrustError::MerkleTree {
                reason: format!(
                    "hash tree of {} leaves holds no node for leaves {start}..{}",
                    self.size,
                    start.saturating_add(size)
                ),
            });
        }
        if size == 0 {
            return Err(TrustError::MerkleTree {
                reason: format!("hash tree asked for an empty subtree at leaf {start}"),
            });
        }
        let split = split_point(size);
        let left = self.subtree(start, split)?;
        let right = self.subtree(start + split, size - split)?;
        Ok(node_hash(&left, &right))
    }

    /// The stored root of the `index`-th perfect subtree of height `height`.
    fn stored(&self, height: u32, index: u64) -> Option<[u8; 32]> {
        let level = self.levels.get(usize::try_from(height).ok()?)?;
        level.get(usize::try_from(index).ok()?).copied()
    }
}

impl fmt::Debug for HashTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HashTree")
            .field("num_leaves", &self.size)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "hash_tree_tests.rs"]
mod hash_tree_tests;
