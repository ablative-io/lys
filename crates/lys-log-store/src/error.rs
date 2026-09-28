//! [`StoreError`] — the storage layer's error type.
//!
//! # Every failure here is attributable
//!
//! A transparency log's storage layer has exactly one job beyond durability:
//! never lose or reorder a leaf *quietly*. So this type has no variant meaning
//! "something went wrong" — each names a specific violated precondition and
//! the index or path it happened at. A caller can always say which leaf, and
//! which rule.
//!
//! That is why [`StoreError::LeafAlreadyWritten`] and
//! [`StoreError::LeafWouldLeaveGap`] are separate variants rather than one
//! "bad index": they are different failures with different causes — a re-used
//! index means two writers believe they own the same position, while a skipped
//! index means a caller lost track of its own extent. Collapsing them would
//! save a variant and cost the diagnosis.
//!
//! # Not a non-oracle boundary
//!
//! `lys-core` deliberately collapses *verification* failures into
//! indistinguishable variants, so an attacker probing a verifier learns
//! nothing about which check rejected them. This type is the opposite by
//! design, and legitimately so: it reports on **local trusted state** that the
//! operator already owns. There is nothing here an attacker could learn that
//! they could not learn by reading the directory. Detail is the product.

use std::path::PathBuf;

/// Errors returned by a [`LeafStore`](crate::LeafStore) implementation.
///
/// # Stability
///
/// `#[non_exhaustive]`: a storage backend that discovers a new way to refuse a
/// write must be able to say so without a major version bump. Adding a variant
/// to an exhaustive error enum is a breaking change, which creates pressure to
/// smuggle new failures into an existing variant's free-text — and a failure
/// reported under the wrong name is worse than a new name to match on.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// A filesystem or backend operation failed. Carries the operation and
    /// path in `context` so the failure is actionable without a backtrace.
    #[error("{context}: {source}")]
    Io {
        /// Description of the operation that failed, including any path.
        context: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A leaf was linked under its final name, but the leaves directory could
    /// not be flushed afterwards, so whether the name survives a crash is
    /// uncertain.
    ///
    /// The link is the commit point: the leaf is written and the store's extent
    /// already covers it. What is unknown is only its durability, which is why
    /// this is neither [`StoreError::Io`] (the write did not fail) nor
    /// [`StoreError::LeafAlreadyWritten`] (no other writer is involved). The
    /// handle refuses further appends with [`StoreError::ReopenRequired`];
    /// reopening flushes the directory before counting the leaf.
    #[error(
        "leaf {index} was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: {source}"
    )]
    LeafDurabilityUncertain {
        /// The index of the leaf whose durability is uncertain.
        index: u64,
        /// The error from flushing the directory.
        #[source]
        source: std::io::Error,
    },

    /// The store handle refused an append because an earlier leaf's
    /// durability is uncertain.
    ///
    /// Appending past a leaf that might not survive a crash could leave a gap
    /// after one, so the handle stops accepting leaves until the store is
    /// reopened, which flushes the leaves directory and counts that leaf.
    #[error(
        "refusing to append: the durability of leaf {index} written on this handle is uncertain; reopen the store"
    )]
    ReopenRequired {
        /// The index of the leaf whose durability is uncertain.
        index: u64,
    },

    /// Every temporary name tried for a leaf was already taken, so the leaf was
    /// not written.
    ///
    /// A taken name is a leftover and is never replaced, so the write moves on
    /// to the next one. Finding every name in the bound taken means something
    /// other than leftovers is creating names in the leaves directory, which is
    /// worth stopping for rather than trying past.
    #[error(
        "refusing to write leaf {index}: all {attempts} temporary names tried in {} were already taken, and a taken name is never replaced",
        path.display()
    )]
    LeafTempNamesTaken {
        /// The index of the leaf that was not written.
        index: u64,
        /// How many names were tried.
        attempts: u32,
        /// The leaves directory the names were tried in.
        path: PathBuf,
    },

    /// A leaf already exists at this index, so the write was refused.
    ///
    /// **This is the write-once rule firing, and it is the intended
    /// behaviour** — not a bug to be worked around. Storage never overwrites a
    /// leaf, because for an append-only log a re-written leaf is a second
    /// history for a position that already had one. Two callers believing they
    /// own the same index is exactly the condition worth reporting loudly.
    #[error(
        "refusing to overwrite the leaf already stored at index {index}: a stored leaf is never rewritten"
    )]
    LeafAlreadyWritten {
        /// The occupied index.
        index: u64,
    },

    /// The write would have skipped one or more indices, leaving a hole.
    ///
    /// Refused because a gap is unrepresentable in an RFC 6962 tree: leaves
    /// are ordered by index with no absent positions, so a hole is not a log
    /// with a missing entry but a log that cannot be hashed at all.
    #[error(
        "refusing to write leaf {index} while the next free index is {next}: that leaves a gap"
    )]
    LeafWouldLeaveGap {
        /// The index the caller asked to write.
        index: u64,
        /// The only index that would have been accepted.
        next: u64,
    },

    /// A pinned root was rejected because it would move the pin backwards.
    ///
    /// The pin only ever advances. A backwards pin is how a truncation would
    /// be made to *look* consistent — a shorter set of leaves plus a matching
    /// shorter pin passes the open-time integrity check. Nothing in this crate
    /// needs to pin backwards, so the operation simply does not exist.
    #[error(
        "refusing to pin tree size {requested} over the pinned size {pinned}: the pin only advances"
    )]
    PinWentBackwards {
        /// The currently pinned tree size.
        pinned: u64,
        /// The smaller size that was requested.
        requested: u64,
    },

    /// A second, different root was offered for a tree size already pinned.
    ///
    /// **This is equivocation, and monotonicity does not catch it** — the size
    /// does not go backwards, so a size-only check waves it through. An
    /// append-only tree has exactly one root at any given size, so two roots at
    /// one size is a claim that the log's history is two different things. That
    /// is the precise property this crate exists to make unrepresentable, and it
    /// would otherwise arrive through the one operation permitted to repeat: an
    /// idempotent re-pin. Re-pinning the *identical* root stays permitted,
    /// because a no-op must not be an error.
    #[error(
        "refusing a second root for tree size {tree_size}: {held} is already pinned there, not {offered} — an append-only tree has one root per size"
    )]
    PinRootChanged {
        /// The tree size at which both roots were claimed.
        tree_size: u64,
        /// The base64 root already held at that size.
        held: String,
        /// The base64 root that was offered for it.
        offered: String,
    },

    /// The path is not an initialized store.
    #[error("not an initialized log store: {}", path.display())]
    NotInitialized {
        /// The path that was checked.
        path: PathBuf,
    },

    /// The path is already an initialized store, and initialization never
    /// runs twice: the origin is pinned at creation and a store's identity is
    /// not re-decided later.
    #[error("already an initialized log store: {}", path.display())]
    AlreadyInitialized {
        /// The path that was checked.
        path: PathBuf,
    },

    /// The stored state failed its integrity check or a structural rule.
    /// Carries the specific discrepancy.
    #[error("log store at {} is corrupt: {reason}", path.display())]
    Corrupt {
        /// The store that failed the check.
        path: PathBuf,
        /// The specific discrepancy or structural violation.
        reason: String,
    },

    /// The stored leaves rebuild to a different tree than the pinned root
    /// describes, and it is not the one divergence a crash can produce.
    ///
    /// Backend-independent by construction: it reports the two trees rather
    /// than a path, because the disagreement is between a set of leaves and a
    /// pin, wherever those happen to live.
    #[error(
        "stored leaves rebuild to tree size {rebuilt_size} with root {rebuilt_root}, but the pinned state is tree size {pinned_size} with root {pinned_root}"
    )]
    PinMismatch {
        /// Tree size the pin claims.
        pinned_size: u64,
        /// Base64 of the root the pin claims.
        pinned_root: String,
        /// Tree size the stored leaves actually rebuild to.
        rebuilt_size: u64,
        /// Base64 of the root the stored leaves actually rebuild to.
        rebuilt_root: String,
    },

    /// A store reported an extent covering `index`, but had no leaf there.
    ///
    /// This is a broken [`LeafStore`](crate::LeafStore) implementation rather
    /// than damaged data: `extent()` promises every index below it is present.
    /// Named separately so the diagnosis points at the backend and not at the
    /// operator's directory.
    #[error("store reports extent {extent} but has no leaf at index {index}")]
    LeafMissingWithinExtent {
        /// The index that was promised and absent.
        index: u64,
        /// The extent the store reported.
        extent: u64,
    },

    /// A stored frontier does not have one subtree root per set bit of its
    /// size, so it is not the frontier of any tree of that size.
    #[error(
        "a frontier for tree size {size} holds {nodes} subtree roots, but a tree of that size has {expected}"
    )]
    FrontierMalformed {
        /// The tree size the frontier claims.
        size: u64,
        /// The number of subtree roots it holds.
        nodes: usize,
        /// The number of set bits of `size`.
        expected: u32,
    },

    /// The log refused further use because an earlier append failed partway.
    ///
    /// An append writes the leaf durably and *then* advances the pin. If the
    /// pin write fails, storage is one leaf ahead of the pin — recoverable on
    /// reopen, but only by exactly one leaf. Continuing to append on the same
    /// handle would put storage two or more ahead, past what recovery can
    /// repair, so the handle stops working instead. Reopen the log.
    #[error(
        "log handle is unusable: an earlier append failed after storing its leaf; reopen the log to recover"
    )]
    Poisoned,

    /// A `lys-core` operation failed — in practice, origin validation, since
    /// a store's origin doubles as its checkpoint note's key name and must
    /// satisfy the same rules.
    #[error(transparent)]
    Trust(#[from] lys_core::TrustError),

    /// Serializing the store's own local-state JSON failed.
    #[error("failed to serialize {what}: {source}")]
    Serialize {
        /// What was being serialized, e.g. "log state".
        what: &'static str,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// A write was refused because the store was opened read-only.
    ///
    /// A reader's handle never changes the evidence it reads: every act that
    /// would write a file is refused by name before anything is touched.
    #[error(
        "refusing to {operation} in the log store at {}: it was opened read-only",
        path.display()
    )]
    ReadOnly {
        /// The store's directory.
        path: PathBuf,
        /// The act that was refused, e.g. "write a leaf" or "pin".
        operation: &'static str,
    },

    /// A read-only open was refused because the store holds exactly one leaf
    /// past its pin.
    ///
    /// That is the shape an append interrupted between storing its leaf and
    /// advancing the pin leaves behind, and repairing it is a write, so only a
    /// writable open performs it. Whether the extra leaf is a clean interrupted
    /// append or sits over a damaged prefix is decided by that writable open,
    /// which repairs or refuses with [`StoreError::PinMismatch`].
    #[error(
        "refusing to open the log store at {} read-only: it holds {extent} leaves but its pin is at tree size {pinned_size}, an interrupted append that only a writable open repairs",
        path.display()
    )]
    RepairPending {
        /// The store's directory.
        path: PathBuf,
        /// The pinned tree size.
        pinned_size: u64,
        /// The number of leaves counted.
        extent: u64,
    },
}

/// Convenience alias for `Result<T, StoreError>`.
pub type StoreResult<T> = Result<T, StoreError>;
