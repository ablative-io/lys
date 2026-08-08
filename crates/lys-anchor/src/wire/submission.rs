//! [`Submission`] — the bytes offered to an anchor — and
//! `SubmissionOutcome` — what it says back.
//!
//! # The anchor does not know what it notarized, and that is the design
//!
//! [`Submission`] carries **one** field: the statement bytes. It deliberately
//! carries no `content_type`, no `producer` and no `kind`. The moment it does,
//! the anchor knows something about meaning — and an anchor that knows what a
//! statement means has, by that fact, taken a position on which statements are
//! well-formed. Its refusals then become statements about the submitter's
//! bytes, its receipts become endorsements rather than observations, and every
//! consumer downstream has to decide how much of that opinion to inherit.
//!
//! An anchor's claim is narrow on purpose: *these bytes were at this index in
//! my tree when I signed this root.* Nothing in it says the statement is true,
//! that whoever sent it was entitled to, or that the signature the statement
//! may itself carry is valid. Those are the reader's questions and the anchor
//! has no way to answer them.
//!
//! # What is signed, and what is only reported
//!
//! Of the four fields in `SubmissionOutcome`, exactly one is a signed claim:
//!
//! - **`receipt`** — the anchor's `COSE_Sign1` over the Merkle root it derived
//!   from the leaf and its inclusion path. This is the artifact. Everything
//!   else in the struct is a convenience that a holder of the receipt could
//!   obtain or check without being told.
//! - **`leaf_index`** and **`tree_size`** are repeated from the receipt's
//!   unprotected header, where they are authenticated *by consequence*:
//!   altering either yields a root the anchor never signed. `tree_size`
//!   carries a documented residual malleability — at some `(index, size)`
//!   pairs two sizes share one walk — so a consumer relying on it must
//!   cross-check the anchor's published checkpoint at that size.
//! - **`leaf_hash`** is signed by nothing at all. It is
//!   `SHA-256(0x00 ‖ statement)`, RFC 6962's leaf hash, which anybody holding
//!   the statement recomputes with standard tooling
//!   (`(printf '\000'; cat statement) | shasum -a 256`). It is reported
//!   because a submitter who has just handed over bytes wants the handle the
//!   log will know them by, not because they should take the anchor's word
//!   for it.
//!
//!   ⚠️ **That pipeline fails silently, and it fails to a plausible answer.**
//!   If `cat` cannot read the file — wrong path, an unquoted name containing a
//!   space — the pipeline still exits `0` (a pipeline reports its *last*
//!   command) and still prints a valid SHA-256: `6e340b9c…fa01d`, the hash of
//!   the prefix byte alone. A verifier who compares that against a `leaf_hash`
//!   and sees a mismatch concludes the anchor is wrong, when in fact nothing
//!   was read. Quote the path, and confirm the pipeline on a **non-empty**
//!   fixture of known hash first — an empty one cannot tell a working `cat`
//!   from a broken one, since both contribute nothing.
//!
//! The anchor's **origin appears nowhere in the receipt**. A receipt says
//! which key signed, not which log; binding a receipt to a named log is the
//! checkpoint's job, and a consumer who needs both needs both artifacts.

/// One statement offered to an anchor, verbatim.
///
/// The bytes are stored as the leaf exactly as supplied — not canonicalized,
/// not parsed, not length-prefixed by this crate. A submitter who wants their
/// statement to be self-describing puts that description *inside* the bytes,
/// where the anchor's signature ends up covering it, rather than in a field
/// beside them where it would not be.
#[derive(Clone, Copy, Debug)]
pub struct Submission<'a> {
    /// The statement bytes, exactly as the submitter supplied them. The anchor
    /// does not interpret them.
    pub statement: &'a [u8],
}

/// What an anchor reports after admitting a submission.
///
/// See the [module docs](self) for which of these fields is a signed claim
/// (one: `receipt`) and which are conveniences the holder can recompute.
#[cfg(feature = "unstable-anchor")]
#[derive(Clone, Debug)]
pub struct SubmissionOutcome {
    /// The index the statement was appended at. Never `0` for a submission —
    /// index 0 is the genesis leaf, written when the anchor was created.
    pub leaf_index: u64,
    /// The size of the anchor's tree once the statement was in it, read from
    /// the tree rather than counted here.
    pub tree_size: u64,
    /// RFC 6962's `SHA-256(0x00 ‖ statement)`, the hash the log knows this
    /// leaf by. Recomputable by anyone holding the statement.
    pub leaf_hash: [u8; 32],
    /// The anchor's signed inclusion receipt — the only value here that is a
    /// claim rather than a report.
    pub receipt: lys_core::receipt::AnchorReceipt,
}
