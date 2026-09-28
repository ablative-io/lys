//! Refusals about proofs over the anchor's own log: receipts, inclusion
//! paths and inclusion artifacts, and the leaves they are asked for.

use lys_core::error::TrustError;

/// Why a receipt, an inclusion path or an inclusion artifact could not be
/// produced for a leaf of the anchor's own log.
///
/// `#[non_exhaustive]` for the reason [`AnchorError`](super::AnchorError)
/// is: a new precondition in this family gets its own name without a major
/// version bump.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProofError {
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

    /// The anchor could not build the JSON inclusion artifact for one of its
    /// own leaves.
    ///
    /// Separate from [`ProofError::Receipt`] rather than folded into it,
    /// because the two failures are not the same failure wearing two names.
    /// A receipt fails at proving or at COSE signing; an artifact additionally
    /// fails at the 2^53 JSON-number bound, at checkpoint encoding under the
    /// origin, and — the one that matters — at `lys-core`'s **build-time
    /// self-verification**, which runs the third party's whole verification
    /// path over the artifact before it is returned. An operator told "failed
    /// to issue a receipt" for a self-verification failure has been pointed at
    /// the wrong artifact and the wrong code.
    ///
    /// **Reachable in practice only if this crate is wrong.** The index is
    /// checked against the log first, the leaf bytes handed to the builder are
    /// the ones read back out of that same log at that same index, and the
    /// origin was validated when the store was created. It is propagated with
    /// its cause instead of being treated as impossible, for the reason the
    /// neighbouring variants give: a precondition that "cannot" fail is
    /// exactly the one nobody notices changing.
    #[error(
        "failed to build an inclusion artifact for leaf {leaf_index} of {origin} at tree size {tree_size}: {source}"
    )]
    InclusionArtifact {
        /// The origin of the log the artifact was for.
        origin: String,
        /// The index the artifact was requested for.
        leaf_index: u64,
        /// The tree size the artifact would have been built against.
        tree_size: u64,
        /// `lys-core`'s reason for refusing to build or to self-verify it.
        source: TrustError,
    },
}
