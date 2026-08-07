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
//!
//! # The submit path arrived, and was judged against that constraint
//!
//! `Anchor::submit` takes bytes from a stranger, so the paragraph above is now
//! live rather than anticipatory. Its refusals are still detailed, and the
//! reason is specific rather than a plea for exemption: **no variant reachable
//! from `submit` is a function of the submitted bytes.** The anchor does not
//! parse, validate, size-check or otherwise decide anything about a statement
//! — it appends it — so there is no verdict on the stranger's input for a
//! distinguishable refusal to disclose. What the path can report is that
//! storage failed, or that the log is too small to produce a conforming
//! receipt: facts about the operator's own machine that are identical for
//! every possible submission.
//!
//! `Anchor::receipt_for` does take a stranger-choosable index, and
//! [`AnchorError::NoSuchLeaf`] names the tree size in its message. That is not
//! a leak either, and not because the number is unimportant: the tree size is
//! the second line of every checkpoint the anchor signs and hands out. A
//! refusal cannot disclose what the artifact it exists to support already
//! publishes.
//!
//! **This reasoning expires with the first variant whose outcome depends on a
//! statement's content, and one is already scheduled.** An `AdmissionPolicy`
//! refusal *is* a verdict on the submitted bytes, and a distinguishable one
//! hands a submitter an oracle for the policy — probe until the message
//! changes, and the rule has been read out without ever being published. When
//! that variant is written it must collapse: one refusal for every reason a
//! submission was not admitted, with the detail going to the operator's log
//! and not to the caller. This note is here so that is decided before the code
//! is, rather than argued about after.

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

    /// A receipt was asked for on a log that holds only its genesis leaf.
    ///
    /// **Not an internal limitation being passed on.** RFC 9942 types an
    /// inclusion path as one-or-more nodes, and the sole leaf of a one-leaf
    /// tree has an empty path, so no conforming receipt exists to issue —
    /// `lys-core` refuses to sign one and is right to. Named here rather than
    /// forwarded because this layer can see the condition coming: it knows the
    /// tree size before it asks, and "your log has nothing in it but genesis"
    /// is the sentence an operator can act on, where a message about CDDL
    /// cardinality is not.
    ///
    /// The remedy is to submit something. The condition disappears at the
    /// first submission and can never return, because a log does not shrink.
    #[error(
        "the log for {origin} holds only its genesis leaf ({tree_size}): a conforming receipt needs a tree of at least two leaves, because a one-leaf tree's inclusion path is empty — submit a statement first"
    )]
    TreeTooSmallForReceipt {
        /// The origin of the log a receipt was requested from.
        origin: String,
        /// The log's size, which is 1 whenever this variant is produced. Kept
        /// as a field rather than written into the text as a literal so the
        /// message reports what was read, not what was expected.
        tree_size: u64,
    },

    /// A receipt was asked for at an index the log does not have.
    ///
    /// Reported with the size rather than as a bare "not found": an index past
    /// the end and an index inside a log that has since been truncated are
    /// very different situations for an operator, and only the size
    /// distinguishes them. Naming it discloses nothing — the size is the
    /// second line of every checkpoint this anchor publishes and signs.
    ///
    /// **Never a reason to append.** The obliging behaviour — log the
    /// statement and hand back a receipt for its new index — would answer a
    /// question about the past with an event in the present, and a caller
    /// asking "what was at index 9" is not asking for index 9 to be created.
    #[error("the log for {origin} has no leaf at index {leaf_index}: it holds {tree_size} leaves")]
    NoSuchLeaf {
        /// The origin of the log that was asked.
        origin: String,
        /// The index that was requested.
        leaf_index: u64,
        /// The log's size when it was asked.
        tree_size: u64,
    },

    /// An inclusion proof's byte encoding was not a whole number of 32-byte
    /// digests.
    ///
    /// A tree emits whole SHA-256 digests, so this cannot arise from a proof
    /// this crate produced — which is the reason it is a named refusal and not
    /// an assumption. The alternative to checking is `chunks_exact` silently
    /// dropping the short tail, and a receipt signed over a silently shortened
    /// path is internally consistent, verifies against itself, and attests to
    /// a root no tree ever held. A malformed proof must be a failure to issue,
    /// never an issued artifact about a fiction.
    #[error(
        "an inclusion proof of {byte_len} bytes is not a whole number of 32-byte digests, so it is not an RFC 6962 path"
    )]
    MalformedInclusionPath {
        /// The length that could not be split into digests.
        byte_len: usize,
    },

    /// The anchor could not prove inclusion of one of its own leaves, or could
    /// not sign the receipt over it.
    ///
    /// Covers both halves of issuing a receipt because both are `lys-core`
    /// refusals about the same request and neither is reachable for an
    /// in-range index on a well-formed anchor: the index was checked against
    /// the log before the proof was requested, and the path handed to the
    /// signer is the one the anchor's own tree produced. Propagated with its
    /// cause rather than treated as impossible — a precondition that "cannot"
    /// fail is exactly the one nobody notices changing.
    #[error(
        "failed to issue a receipt for leaf {leaf_index} of {origin} at tree size {tree_size}: {source}"
    )]
    Receipt {
        /// The origin of the log the receipt was for.
        origin: String,
        /// The index the receipt was requested for.
        leaf_index: u64,
        /// The tree size the receipt would have been issued against.
        tree_size: u64,
        /// `lys-core`'s reason for refusing to prove or to sign.
        source: TrustError,
    },
}

/// Convenience alias for `Result<T, AnchorError>`.
pub type AnchorResult<T> = Result<T, AnchorError>;
