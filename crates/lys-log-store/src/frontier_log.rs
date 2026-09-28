//! [`FrontierLog`] — an RFC 6962 append-only log over any [`LeafStore`] that
//! holds only its tree's frontier.
//!
//! # Why a second log type
//!
//! [`Log`](crate::Log) holds every leaf and the whole tree, rebuilt from every
//! stored leaf at open. That is what its callers' proofs are built from, and it
//! is a start whose cost grows with the log. `FrontierLog` holds the frontier
//! instead: the size, the root, and what an append needs to move them. It is
//! opened either from nothing, reading every leaf once, or from a frontier the
//! caller already trusts (a checked [`Snapshot`](crate::Snapshot)), reading only
//! the leaves after it. Leaves are read from the store when they are asked for,
//! and the full tree an inclusion proof needs is built on the first proof and
//! kept, never at open.
//!
//! # The same integrity routine as `Log`
//!
//! Opening reconciles the tree with the store's pin exactly as
//! [`Log::open`](crate::Log::open) does: equal is clean, exactly one extra leaf
//! whose prefix reaches the pinned root is an interrupted append and is
//! repaired and reported, and anything else is
//! [`StoreError::PinMismatch`]. Appends store the leaf before the pin, and a
//! handle whose pin write failed is poisoned.
//!
//! # What a resumed open does not check
//!
//! A resumed open does not reread the leaves before the frontier, so damage to
//! a leaf inside that prefix is not seen at open. It is seen when the proof
//! tree is first built, which rebuilds from every leaf and refuses a root that
//! is not the frontier's; and the frontier itself was checked against the
//! pinned root through every leaf after it before the log was handed out.

use std::sync::OnceLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::Ed25519Identity;
use lys_core::merkle::{AppendOnlyTree, RawLeaf, RootHash};

use crate::error::{StoreError, StoreResult};
use crate::frontier::Frontier;
use crate::snapshot::seal;
use crate::store::{LeafStore, PinnedRoot};

/// Leaves a start read, in log order, beginning at index `from`.
#[derive(Clone, PartialEq, Eq)]
pub struct Tail {
    /// The index of the first leaf.
    pub from: u64,
    /// The raw leaf bytes.
    pub leaves: Vec<Vec<u8>>,
}

impl std::fmt::Debug for Tail {
    /// Summarizes the tail by position and count, not content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tail")
            .field("from", &self.from)
            .field("leaves", &self.leaves.len())
            .finish()
    }
}

/// The leaves read from a frontier onwards, and the tree they extend it to.
pub(crate) struct Reading {
    frontier: Frontier,
    tail: Tail,
    /// The root at the pinned size, if the reading passed through it.
    root_at_pin: Option<[u8; 32]>,
}

impl Reading {
    /// Reads every leaf of `store` after `frontier`, extending it.
    pub(crate) fn from<S: LeafStore>(store: &S, frontier: Frontier) -> StoreResult<Self> {
        let pinned = store.pinned();
        let extent = store.extent();
        let from = frontier.size();
        let mut frontier = frontier;
        let mut root_at_pin = (from == pinned.tree_size).then(|| frontier.root());
        let mut leaves = Vec::new();
        for index in from..extent {
            let leaf = store
                .leaf(index)?
                .ok_or(StoreError::LeafMissingWithinExtent { index, extent })?;
            frontier.push(&leaf);
            leaves.push(leaf);
            if frontier.size() == pinned.tree_size {
                root_at_pin = Some(frontier.root());
            }
        }
        Ok(Self {
            frontier,
            tail: Tail { from, leaves },
            root_at_pin,
        })
    }

    /// How many leaves the reading read.
    pub(crate) fn replayed(&self) -> u64 {
        self.frontier.size().saturating_sub(self.tail.from)
    }

    /// The pin the reading repairs to, `None` if it matches the pin already.
    ///
    /// # Errors
    ///
    /// [`StoreError::PinMismatch`] for any divergence but one interrupted
    /// append.
    pub(crate) fn reconcile(&self, pinned: PinnedRoot) -> StoreResult<Option<PinnedRoot>> {
        let (size, root) = (self.frontier.size(), self.frontier.root());
        if size == pinned.tree_size && root == pinned.root {
            return Ok(None);
        }
        if size == pinned.tree_size.saturating_add(1) && self.root_at_pin == Some(pinned.root) {
            return Ok(Some(PinnedRoot {
                tree_size: size,
                root,
            }));
        }
        Err(StoreError::PinMismatch {
            pinned_size: pinned.tree_size,
            pinned_root: STANDARD.encode(pinned.root),
            rebuilt_size: size,
            rebuilt_root: STANDARD.encode(root),
        })
    }
}

/// An append-only Merkle log over a [`LeafStore`] holding only its frontier.
pub struct FrontierLog<S: LeafStore> {
    store: S,
    frontier: Frontier,
    proofs: OnceLock<AppendOnlyTree<RawLeaf>>,
    recovered_to: Option<u64>,
    poisoned: bool,
}

impl<S: LeafStore> std::fmt::Debug for FrontierLog<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrontierLog")
            .field("origin", &self.store.origin())
            .field("frontier", &self.frontier)
            .field("recovered_to", &self.recovered_to)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl<S: LeafStore> FrontierLog<S> {
    /// Opens a log over `store` from nothing: reads every leaf once, builds
    /// the frontier, and reconciles it with the pin. Answers the log and
    /// every leaf it read.
    ///
    /// # Errors
    ///
    /// As [`Log::open`](crate::Log::open).
    pub fn open(store: S) -> StoreResult<(Self, Tail)> {
        let reading = Reading::from(&store, Frontier::new())?;
        Self::from_reading(store, reading)
    }

    /// Opens a log over `store` from `frontier`, a tree the caller has already
    /// checked is this log's at its size: reads only the leaves after it, and
    /// reconciles the extended tree with the pin. Answers the log and the
    /// leaves it read.
    ///
    /// # Errors
    ///
    /// [`StoreError::PinMismatch`] when the leaves after `frontier` do not
    /// reach the pinned root from it, and as [`Log::open`](crate::Log::open).
    pub fn resume(store: S, frontier: Frontier) -> StoreResult<(Self, Tail)> {
        let reading = Reading::from(&store, frontier)?;
        Self::from_reading(store, reading)
    }

    /// Builds the log from a reading, repairing the pin when the reading
    /// found one interrupted append.
    pub(crate) fn from_reading(mut store: S, reading: Reading) -> StoreResult<(Self, Tail)> {
        let repair = reading.reconcile(store.pinned())?;
        let recovered_to = match repair {
            Some(pin) => {
                store.pin(pin)?;
                Some(pin.tree_size)
            }
            None => None,
        };
        let log = Self {
            store,
            frontier: reading.frontier,
            proofs: OnceLock::new(),
            recovered_to,
            poisoned: false,
        };
        Ok((log, reading.tail))
    }

    /// The tree size an interrupted append was recovered to at open, if one
    /// was. `None` means the log opened clean.
    pub fn recovered_to(&self) -> Option<u64> {
        self.recovered_to
    }

    /// The number of leaves.
    pub fn len(&self) -> u64 {
        self.frontier.size()
    }

    /// Whether the log holds no leaf.
    pub fn is_empty(&self) -> bool {
        self.frontier.is_empty()
    }

    /// The current root, with its size.
    pub fn root(&self) -> RootHash {
        self.frontier.root_hash()
    }

    /// The current frontier.
    pub fn frontier(&self) -> &Frontier {
        &self.frontier
    }

    /// The log's origin, fixed when its store was created.
    pub fn origin(&self) -> &str {
        self.store.origin()
    }

    /// The underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Appends raw leaf bytes: stores the leaf durably, extends the tree,
    /// then advances the pin. Returns the new leaf's index and its RFC 6962
    /// leaf hash.
    ///
    /// # Errors
    ///
    /// As [`Log::append`](crate::Log::append).
    pub fn append(&mut self, leaf_bytes: &[u8]) -> StoreResult<(u64, [u8; 32])> {
        if self.poisoned {
            return Err(StoreError::Poisoned);
        }
        let index = self.frontier.size();
        self.store.put_leaf(index, leaf_bytes)?;
        // The leaf is durable and the pin is not yet advanced; a failure from
        // here leaves the one-leaf-ahead state, so the handle is poisoned.
        self.poisoned = true;
        let leaf_hash = self.frontier.push(leaf_bytes);
        if let Some(tree) = self.proofs.get_mut() {
            tree.append_raw(leaf_bytes);
        }
        self.store.pin(PinnedRoot {
            tree_size: self.frontier.size(),
            root: self.frontier.root(),
        })?;
        self.poisoned = false;
        Ok((index, leaf_hash))
    }

    /// The raw bytes of the leaf at `index`, read from the store, or `None`
    /// past the end of the log.
    ///
    /// # Errors
    ///
    /// Whatever the store returns while reading, and
    /// [`StoreError::LeafMissingWithinExtent`] if it has no leaf inside the log.
    pub fn leaf_bytes(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        if index >= self.len() {
            return Ok(None);
        }
        self.store
            .leaf(index)?
            .map(Some)
            .ok_or(StoreError::LeafMissingWithinExtent {
                index,
                extent: self.store.extent(),
            })
    }

    /// Every leaf from `from` to the end of the log, read from the store.
    ///
    /// # Errors
    ///
    /// As [`FrontierLog::leaf_bytes`].
    pub fn leaves_from(&self, from: u64) -> StoreResult<Tail> {
        let mut leaves = Vec::new();
        for index in from..self.len() {
            let leaf = self
                .leaf_bytes(index)?
                .ok_or(StoreError::LeafMissingWithinExtent {
                    index,
                    extent: self.len(),
                })?;
            leaves.push(leaf);
        }
        Ok(Tail { from, leaves })
    }

    /// The whole tree, for inclusion and consistency proofs.
    ///
    /// Built from every stored leaf the first time it is asked for, then kept
    /// and extended by each append. The rebuilt root must be the frontier's.
    ///
    /// # Errors
    ///
    /// [`StoreError::PinMismatch`] if the stored leaves rebuild to another
    /// root than the log holds, and whatever the store returns while reading.
    pub fn proof_tree(&self) -> StoreResult<&AppendOnlyTree<RawLeaf>> {
        if let Some(tree) = self.proofs.get() {
            return Ok(tree);
        }
        let tail = self.leaves_from(0)?;
        let tree = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(&tail.leaves);
        let (rebuilt_root, rebuilt_size) = tree.root().to_parts();
        if rebuilt_root != self.frontier.root() || rebuilt_size != self.frontier.size() {
            return Err(StoreError::PinMismatch {
                pinned_size: self.frontier.size(),
                pinned_root: STANDARD.encode(self.frontier.root()),
                rebuilt_size,
                rebuilt_root: STANDARD.encode(rebuilt_root),
            });
        }
        Ok(self.proofs.get_or_init(|| tree))
    }

    /// Writes a signed snapshot of `state`, the owner's folded state of every
    /// leaf the log holds now, under `domain`.
    ///
    /// The caller must hold `state` as the fold of exactly [`len`](Self::len)
    /// leaves: the snapshot binds the state to this size and root, and a start
    /// from it replays only the leaves after them.
    ///
    /// # Errors
    ///
    /// [`StoreError::Poisoned`] while an append is half complete, and whatever
    /// the store returns while writing.
    pub fn write_snapshot(
        &mut self,
        domain: &str,
        state: &[u8],
        key: &Ed25519Identity,
    ) -> StoreResult<u64> {
        if self.poisoned {
            return Err(StoreError::Poisoned);
        }
        let sealed = seal(domain, self.store.origin(), &self.frontier, state, key);
        self.store.put_snapshot(&sealed)?;
        Ok(self.frontier.size())
    }
}

#[cfg(test)]
#[path = "frontier_log_tests.rs"]
mod tests;
