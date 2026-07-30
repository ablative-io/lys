//! [`LeafStore`] — what a transparency log requires of durable storage.
//!
//! # What is deliberately absent, and why
//!
//! There is **no `fork`, no `branch`, no `merge`, no `delete`, no `truncate`,
//! and no way to rewrite a stored leaf.** These are not omissions awaiting a
//! later release. For an append-only log they are not features:
//!
//! - **A branch is equivocation with a nicer name.** The single property a
//!   transparency log exists to provide is that its operator *cannot* hold two
//!   histories and show each observer whichever suits. Two branches of one log
//!   are two histories, both internally consistent, both signable — which is
//!   precisely the attack the log is built to make impossible. A storage API
//!   offering `fork` hands that attack back as a supported operation.
//! - **A merge cannot exist without deciding whose history won**, and there is
//!   no honest answer: both sides may already have been shown to someone, and
//!   whichever is discarded was witnessed. An append-only log has no conflict
//!   resolution because it must have no conflicts.
//! - **Deletion and truncation break the only promise the log makes.** An
//!   entry that can be removed was never evidence of anything.
//!
//! This absence is load-bearing, so it is stated here rather than left to be
//! inferred from the method list: a future implementor adding `fork` because
//! their backend supports it would be removing the reason the log exists,
//! while passing every test in this crate.
//!
//! # The index is a cross-check, not a convenience
//!
//! [`LeafStore::put_leaf`] takes the index to write *and* refuses anything but
//! the next free one — which looks redundant, since the store already knows its
//! own [`extent`](LeafStore::extent) and could simply append.
//!
//! It is not redundant. The caller derives that index from **its own Merkle
//! tree's length**, a counter maintained independently of the store's extent.
//! Passing it forces the two to agree at every write. If they have diverged —
//! the tree has five leaves, storage has six — the write fails at the position
//! where the disagreement is still attributable, instead of appending at six
//! and producing a tree that silently disagrees with what is on disk.
//!
//! It is the same discipline the verification bundle applies to chain links:
//! two independently maintained statements about the same fact, forced to match.
//!
//! # Rejection is a feature, and gaps are its price
//!
//! Refusing a write means a caller *can* observe a position that has no leaf —
//! it asked, it was told no. That is fine, and it is the whole point: a
//! **reported** gap is attributable to a specific refused write, whereas a
//! silent one is indistinguishable from a leaf that was never offered. This
//! crate is organised so that the only reachable route to a missing leaf runs
//! through an error the caller received.

use crate::error::StoreResult;

/// The pinned `(tree_size, root)` a store carries alongside its leaves.
///
/// # What this does and does not prove
///
/// The pin lets an open detect that stored leaves no longer rebuild to the
/// tree they last rebuilt to — local bit-rot, a partially applied write, or
/// tampering with the stored bytes. That is worth having and it is *all* it
/// is: an actor able to rewrite both the leaves and the pin can present a
/// consistent shorter log, and no purely local check can catch that.
///
/// The log's actual tamper-evidence is carried by **emitted** artifacts —
/// signed checkpoints and proofs already in someone else's hands — not by
/// this. Treating the pin as cryptographic assurance would be believing a
/// claim the party being judged is free to set, which is the one thing lys
/// refuses to do anywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedRoot {
    /// Number of leaves the pinned root covers.
    pub tree_size: u64,
    /// The RFC 6962 Merkle tree hash over those leaves.
    pub root: [u8; 32],
}

/// Durable, append-only, write-once storage for a log's leaves.
///
/// # Contract an implementation must honour
///
/// 1. **Durable on return.** When [`put_leaf`](Self::put_leaf) or
///    [`pin`](Self::pin) returns `Ok`, the bytes have reached stable storage.
///    Returning `Ok` for a write still sitting in a buffer is the defect this
///    contract exists to forbid: a caller that is told a leaf is durable will
///    build on it, and a crash then removes an entry the log has already
///    counted — a hole manufactured by the storage layer, arriving from the one
///    direction the log's own checks cannot attribute to anybody.
/// 2. **Write-once.** A stored leaf is never rewritten. A second write to an
///    occupied index is [`StoreError::LeafAlreadyWritten`], never an overwrite
///    and never a silent success.
/// 3. **Contiguous.** Every index in `0..extent()` holds a leaf. An
///    implementation must establish this when it opens, not assume it.
/// 4. **Monotonic pin.** [`pin`](Self::pin) never moves the pinned tree size
///    backwards.
/// 5. **Immutable origin.** Set once, when the store is created, and never
///    changed thereafter.
///
/// [`StoreError::LeafAlreadyWritten`]: crate::StoreError::LeafAlreadyWritten
pub trait LeafStore {
    /// The log's origin, fixed when the store was created.
    ///
    /// It doubles as the key name of the log's signed checkpoints, so it is
    /// validated against the checkpoint rules at creation — see
    /// [`validate_origin`](crate::validate_origin).
    fn origin(&self) -> &str;

    /// The number of stored leaves, which is also the index of the next free
    /// position.
    ///
    /// Infallible by design: contiguity is established when the store opens,
    /// so by the time a caller holds a store this number is already known and
    /// already checked. An implementation that would need I/O to answer this
    /// has not finished opening.
    fn extent(&self) -> u64;

    /// The raw bytes stored at `index`, or `None` if `index >= extent()`.
    ///
    /// The bytes are returned verbatim: they are the RFC 6962 leaf preimage,
    /// so the store must not frame, pad, or re-encode them.
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>>;

    /// Stores `bytes` at `index`, durably, refusing anything but the next free
    /// index (see the module docs on why the index is passed at all).
    ///
    /// # Errors
    ///
    /// [`StoreError::LeafAlreadyWritten`] if `index < extent()`,
    /// [`StoreError::LeafWouldLeaveGap`] if `index > extent()`, and
    /// [`StoreError::Io`] if the write cannot be made durable.
    ///
    /// [`StoreError::LeafAlreadyWritten`]: crate::StoreError::LeafAlreadyWritten
    /// [`StoreError::LeafWouldLeaveGap`]: crate::StoreError::LeafWouldLeaveGap
    /// [`StoreError::Io`]: crate::StoreError::Io
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()>;

    /// The currently pinned `(tree_size, root)`.
    ///
    /// Not optional: a store pins the empty tree when it is created, so a
    /// store that exists has a pin. An absent pin is a corrupt store, reported
    /// when opening, rather than a state callers must handle.
    fn pinned(&self) -> PinnedRoot;

    /// Durably records a new pinned root.
    ///
    /// This is the **only** operation on a store that replaces previously
    /// stored data, and the only reason it is allowed to is that the pin is
    /// local integrity state rather than log content — see [`PinnedRoot`].
    ///
    /// # Errors
    ///
    /// [`StoreError::PinWentBackwards`] if `pin.tree_size` is below the
    /// current pinned size, and [`StoreError::Io`] if the write cannot be made
    /// durable.
    ///
    /// [`StoreError::PinWentBackwards`]: crate::StoreError::PinWentBackwards
    /// [`StoreError::Io`]: crate::StoreError::Io
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()>;
}
