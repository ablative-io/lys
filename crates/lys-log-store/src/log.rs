//! [`Log`] — an RFC 6962 append-only tree over any [`LeafStore`].
//!
//! # Why the tree lives here and not in the caller
//!
//! The integrity routine below — rebuild from stored leaves, compare against
//! the pin, repair exactly one interrupted append — is the part that must be
//! *identical* whichever backend is underneath. Leaving it to callers would
//! mean a file-backed log and a database-backed log disagreeing about what
//! counts as corrupt, which is the same defect as two copies of a canonical
//! encoder disagreeing about what counts as canonical.
//!
//! It cannot live in `lys-core` either: `lys-core` performs no I/O, and this
//! routine is defined entirely by what durable storage can do to you.
//!
//! # Append order is load-bearing
//!
//! [`Log::append`] stores the leaf durably **before** advancing the pin, and
//! never the reverse. That ordering is what makes "storage is exactly one leaf
//! ahead of the pin" the only divergence a crash can produce, and therefore the
//! only one [`Log::open`] repairs.
//!
//! Pinning first would allow the opposite state — a pin ahead of the leaves —
//! which is *not* repairable and must not be: a pin covering a leaf that was
//! never stored describes a tree nobody can rebuild. Recovery deliberately has
//! no case for it, so it surfaces as [`StoreError::PinMismatch`].

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::checkpoint::CheckpointBody;
use lys_core::merkle::{AppendOnlyTree, RawLeaf, raw_leaf_hash};

use crate::error::{StoreError, StoreResult};
use crate::store::{LeafStore, PinnedRoot};

/// Checks that `origin` satisfies the rules a signed checkpoint will impose.
///
/// A log's origin doubles as the key name of its checkpoint notes, so an
/// origin accepted at creation but rejected at checkpoint time would produce a
/// log that cannot publish — discovered at the moment it matters most. This is
/// run when a store is created so the failure lands at creation instead.
///
/// # Errors
///
/// [`StoreError::Trust`] carrying `lys-core`'s own reason for refusing it.
pub fn validate_origin(origin: &str) -> StoreResult<()> {
    let empty_root = AppendOnlyTree::<RawLeaf>::new().root();
    CheckpointBody::from_root(origin, &empty_root)?;
    Ok(())
}

/// An append-only Merkle log over a [`LeafStore`], verified at open.
///
/// Holds every leaf in memory: prefix rebuilds (for consistency proofs) need
/// the leaf bytes, and a log that re-read them from storage per proof would
/// trade a bounded, visible memory cost for repeated I/O. That is a fine trade
/// at the scale lys logs run at, and it is stated rather than hidden because it
/// is the reason this type is not suitable for an unbounded log.
pub struct Log<S: LeafStore> {
    store: S,
    leaves: Vec<Vec<u8>>,
    tree: AppendOnlyTree<RawLeaf>,
    recovered_to: Option<u64>,
    poisoned: bool,
}

impl<S: LeafStore> std::fmt::Debug for Log<S> {
    /// Summarizes the log without dumping leaf contents — they are public log
    /// content, but arbitrarily large.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Log")
            .field("origin", &self.store.origin())
            .field("num_leaves", &self.leaves.len())
            .field("recovered_to", &self.recovered_to)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl<S: LeafStore> Log<S> {
    /// Opens a log over `store`: loads every leaf, rebuilds the tree, and
    /// reconciles it with the pinned root.
    ///
    /// One divergence is tolerated and repaired — exactly one extra contiguous
    /// leaf whose pinned-size prefix rebuilds to the pinned root, which is an
    /// append interrupted between storing the leaf and advancing the pin. The
    /// repair advances the pin, is reported by
    /// [`recovered_to`](Self::recovered_to), and is never silent.
    ///
    /// Anything else is a mismatch. In particular, a tampered leaf *inside* the
    /// pinned prefix does not qualify however many leaves are present, because
    /// the prefix root will not match.
    ///
    /// # Errors
    ///
    /// [`StoreError::PinMismatch`] if the rebuilt tree disagrees with the pin
    /// in any other way, [`StoreError::LeafMissingWithinExtent`] if the store
    /// breaks its own contiguity promise, and whatever the store returns while
    /// reading leaves or writing the repaired pin.
    pub fn open(store: S) -> StoreResult<Self> {
        let extent = store.extent();
        let mut leaves = Vec::with_capacity(usize::try_from(extent).unwrap_or(0));
        for index in 0..extent {
            let leaf = store
                .leaf(index)?
                .ok_or(StoreError::LeafMissingWithinExtent { index, extent })?;
            leaves.push(leaf);
        }
        let tree = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(&leaves);
        let mut log = Self {
            store,
            leaves,
            tree,
            recovered_to: None,
            poisoned: false,
        };
        log.reconcile_with_pin()?;
        Ok(log)
    }

    /// Compares the rebuilt tree against the pin, applying the single
    /// tolerated crash-recovery divergence.
    fn reconcile_with_pin(&mut self) -> StoreResult<()> {
        let (rebuilt_root, rebuilt_size) = self.tree.root().to_parts();
        let pinned = self.store.pinned();
        if rebuilt_size == pinned.tree_size && rebuilt_root == pinned.root {
            return Ok(());
        }
        if rebuilt_size == pinned.tree_size + 1 {
            let prefix = self.prefix_tree(pinned.tree_size)?;
            let (prefix_root, _prefix_size) = prefix.root().to_parts();
            if prefix_root == pinned.root {
                self.store.pin(PinnedRoot {
                    tree_size: rebuilt_size,
                    root: rebuilt_root,
                })?;
                self.recovered_to = Some(rebuilt_size);
                return Ok(());
            }
        }
        Err(StoreError::PinMismatch {
            pinned_size: pinned.tree_size,
            pinned_root: STANDARD.encode(pinned.root),
            rebuilt_size,
            rebuilt_root: STANDARD.encode(rebuilt_root),
        })
    }

    /// The tree size an interrupted append was recovered to at open, if one
    /// was. `None` means the log opened clean.
    ///
    /// Returned rather than logged: a library writing to stderr decides for its
    /// caller how a recovery gets reported, and a repair the operator never
    /// hears about is indistinguishable from one that never happened.
    pub fn recovered_to(&self) -> Option<u64> {
        self.recovered_to
    }

    /// Appends raw leaf bytes: stores the leaf durably, extends the tree, then
    /// advances the pin. Returns the new leaf's index and its RFC 6962 leaf
    /// hash.
    ///
    /// # Errors
    ///
    /// Whatever the store returns — including
    /// [`StoreError::LeafAlreadyWritten`] if another writer took this index.
    /// [`StoreError::Poisoned`] if an earlier append on this handle failed
    /// after storing its leaf (see the module docs on append order).
    pub fn append(&mut self, leaf_bytes: &[u8]) -> StoreResult<(u64, [u8; 32])> {
        if self.poisoned {
            return Err(StoreError::Poisoned);
        }
        let index = self.tree.len();
        self.store.put_leaf(index, leaf_bytes)?;
        // Past this point the leaf is durable but the pin is not yet advanced,
        // so any failure leaves the recoverable one-leaf-ahead state. The
        // handle is poisoned so a caller cannot append again and push storage
        // two ahead, which recovery would refuse to repair.
        self.poisoned = true;
        self.tree.append_raw(leaf_bytes);
        self.leaves.push(leaf_bytes.to_vec());
        let (root, tree_size) = self.tree.root().to_parts();
        self.store.pin(PinnedRoot { tree_size, root })?;
        self.poisoned = false;
        Ok((index, raw_leaf_hash(leaf_bytes)))
    }

    /// The log's origin, fixed when its store was created.
    pub fn origin(&self) -> &str {
        self.store.origin()
    }

    /// The current tree — rebuilt and reconciled at open, extended by append.
    pub fn tree(&self) -> &AppendOnlyTree<RawLeaf> {
        &self.tree
    }

    /// The underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The raw bytes of the leaf at `index`, if it exists.
    pub fn leaf_bytes(&self, index: u64) -> Option<&[u8]> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.leaves.get(i))
            .map(Vec::as_slice)
    }

    /// Rebuilds the tree over the first `old_size` leaves — the prefix a
    /// consistency proof starts from.
    ///
    /// # Errors
    ///
    /// [`StoreError::LeafWouldLeaveGap`] if `old_size` exceeds the current tree
    /// size: a prefix longer than the log is a request for leaves that would
    /// have to be invented, which is the same refusal as writing past the end.
    pub fn prefix_tree(&self, old_size: u64) -> StoreResult<AppendOnlyTree<RawLeaf>> {
        let count = usize::try_from(old_size)
            .ok()
            .filter(|&n| n <= self.leaves.len())
            .ok_or_else(|| StoreError::LeafWouldLeaveGap {
                index: old_size,
                next: self.tree.len(),
            })?;
        Ok(AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(
            &self.leaves[..count],
        ))
    }
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod tests;
