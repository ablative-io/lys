//! [`AnchorError`] — the anchor's error type.
//!
//! # Every failure named here is about the operator's own log
//!
//! `lys-core` deliberately collapses *verification* failures into
//! indistinguishable variants, so an attacker probing a verifier learns nothing
//! about which check rejected them. Nothing in this type is reachable that way:
//! each variant reports on local state the operator already owns and could read
//! off their own disk, exactly as `StoreError` argues for itself. Detail is the
//! product here.
//!
//! **That is a property of the current surface, not a licence for the next
//! variant.** The moment this crate grows a path a stranger can drive — a
//! submission, a verification, anything taking bytes from someone else — a
//! distinguishable refusal on that path is a parsing oracle, and the variant
//! that carries it must collapse instead. The constraint is written down here
//! so a future addition is judged against it rather than pattern-matched onto
//! the variants below.

use lys_core::error::TrustError;
use lys_log_store::StoreError;

/// Errors returned by an [`Anchor`](crate::Anchor) operation.
///
/// # Stability
///
/// `#[non_exhaustive]`: an anchor that discovers a new precondition must be
/// able to name it without a major version bump. The alternative is pressure to
/// smuggle a new failure into an existing variant's free text, and a failure
/// reported under the wrong name is worse than a new name to match on.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnchorError {
    /// The underlying log or its storage refused the operation.
    ///
    /// Transparent because the storage layer's message already names the index,
    /// path or pin involved, and restating it here would only add a second
    /// place for that wording to drift.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// The log holds no leaves, so it has no genesis leaf and never can.
    ///
    /// **Terminal, not a state to initialize out of.** `LeafStore` offers no
    /// `insert` and no way to rewrite a leaf, so position 0 cannot be filled
    /// after any other entry exists — and it cannot be filled *now* either
    /// without deciding, on the log's behalf, what its first entry says.
    /// [`Anchor::create`](crate::Anchor::create) is where genesis bytes are
    /// supplied, by the caller, once.
    #[error(
        "the log for {origin} has no genesis leaf: an anchor's leaf 0 is written when it is created and can never be inserted afterwards"
    )]
    NoGenesisLeaf {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
    },

    /// Creation was asked to write genesis into a log that already has entries.
    ///
    /// Appending here would put the genesis bytes at whatever the next free
    /// index happens to be, producing a log whose leaf 0 is something else
    /// entirely while every later check passes. Refused rather than appended:
    /// there is exactly one position genesis can occupy, and it is taken.
    #[error(
        "refusing to write genesis into the log for {origin}: it already holds {tree_size} leaves, and genesis is leaf 0 or nothing"
    )]
    GenesisAlreadyWritten {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
        /// The number of leaves already present.
        tree_size: u64,
    },

    /// The anchor's signing key could not be loaded from its file.
    ///
    /// The path is carried because `lys-core`'s own reason does not name it —
    /// `std::fs` errors do not include the path they were raised for — and an
    /// operator holding "failed to read identity key: No such file or
    /// directory" has been told everything except the one fact they need.
    ///
    /// **A missing file reaches here rather than being repaired.** Generating a
    /// key for a caller who asked to load one produces an anchor that publishes
    /// under an identity nobody was ever told about, and reports success while
    /// doing it.
    #[error("failed to load the anchor's signing key from {path}: {source}")]
    SignerKey {
        /// The key file path, as it was given.
        path: String,
        /// `lys-core`'s reason for refusing the key file.
        source: TrustError,
    },

    /// The anchor could not sign a checkpoint over its own log.
    ///
    /// Every precondition `lys-core` checks here is already satisfied by
    /// construction — the origin was validated when the store was created, and
    /// the body is machine-generated — so this variant reports something
    /// genuinely unexpected rather than a routine refusal. It is propagated
    /// with its cause instead of being treated as impossible, because a
    /// precondition that "cannot" fail is exactly the one nobody notices
    /// changing.
    #[error("failed to publish a checkpoint for {origin}: {source}")]
    Checkpoint {
        /// The origin the checkpoint was being published for.
        origin: String,
        /// `lys-core`'s reason for refusing to encode or sign it.
        source: TrustError,
    },
}

/// Convenience alias for `Result<T, AnchorError>`.
pub type AnchorResult<T> = Result<T, AnchorError>;
