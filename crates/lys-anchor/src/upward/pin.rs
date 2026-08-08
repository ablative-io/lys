//! [`pin()`] — publish this anchor's checkpoint and have a parent anchor record
//! it.
//!
//! # The whole function, and why it is this short on purpose
//!
//! ```text
//! let checkpoint = child.publish_checkpoint()?;
//! let recorded   = parent.submit(Submission { statement: checkpoint.note.as_bytes() }, context)?;
//! ```
//!
//! Two core calls and no third thing. Every property this module claims comes
//! from that shape rather than from a check written here:
//!
//! - **The outcome is the core's type.** `recorded` is a
//!   [`SubmissionOutcome`] — the value [`Anchor::submit`] returns to every
//!   submitter — not a federation twin that happens to carry the same fields.
//!   A twin would be a second path that could drift while still producing
//!   "equivalent" results, and equivalence is a claim two diverging
//!   implementations keep satisfying right up until they do not.
//! - **The parent cannot tell.** The two arguments handed to `submit` are the
//!   two arguments a submission has. There is no cascade flag because
//!   [`Submission`] has one field, and adding one would make the parent's
//!   receipts endorsements rather than observations.
//! - **The checkpoint returned is the checkpoint submitted.** This function
//!   publishes it and hands both halves back together, so a caller cannot end
//!   up holding a receipt for one set of note bytes and a note stating
//!   something else. A caller who published separately and submitted separately
//!   could — an append between the two calls yields a checkpoint at a different
//!   size — and [`bundle_for`](super::bundle::bundle_for) refuses the bundle
//!   that results rather than emitting one nothing can verify.
//!
//! # This is the in-process composition, and the wire is the note
//!
//! Both anchors are values in one process here, because there is no transport
//! (BUILD-PLAN increment 12, blocked). That is a limitation of the plumbing and
//! not of the design: **the only thing that crosses between the two halves is
//! `checkpoint.note`**, a byte string. When a transport exists, the child sends
//! those bytes and the parent runs [`observe()`](crate::witness::observe()) or
//! `submit` on them; nothing in this file has to change shape for that, because
//! nothing in it is a shared object.
//!
//! # If the parent refuses
//!
//! A parent's admission policy can decline, and then
//! [`AnchorError::NotAdmitted`](crate::AnchorError::NotAdmitted) comes back
//! carrying nothing — the parent's refusal is one indistinguishable value for
//! every rule any policy might have had, and a cascade is not exempt from that.
//! **Nothing was recorded anywhere.** The child published a checkpoint, which
//! is not an append, so both logs are exactly as they were.

use lys_log_store::LeafStore;

use crate::admission::{AdmissionPolicy, SubmitterContext};
use crate::anchor::{Anchor, PublishedCheckpoint};
use crate::error::AnchorResult;
use crate::keys::InProcessSigner;
use crate::wire::{Submission, SubmissionOutcome};

// Scaffolding for both files in this module. Declared here rather than in
// `mod.rs`, which carries only module declarations, re-exports and docs.
#[cfg(test)]
#[path = "fixture.rs"]
pub(super) mod fixture;

/// One rung of a cascade: a child anchor's published checkpoint, and the parent
/// anchor's receipt for it.
///
/// Both halves are needed and neither is redundant, for the reason
/// `lys-core`'s `BundleLink` gives: a receipt's payload is the **detached**
/// Merkle root, so checking one requires the leaf bytes it proves — and here
/// those bytes are a signed checkpoint, which is what lets the *next* rung be
/// compared against this one.
///
/// # What is signed, and by whom
///
/// - **`checkpoint.note`** is signed by the **child**, under the child's own
///   origin. It says what the child's log held at a size.
/// - **`recorded.receipt`** is signed by the **parent**, over a root the parent
///   derived from its own tree. It says those exact note bytes were a leaf in
///   the parent's tree.
/// - Everything else in `recorded` is a report, not a claim —
///   [`wire::submission`](crate::wire::submission) says which is which.
///
/// Nothing in this value is signed by both, and there is no field in which
/// either party could record an opinion about the other.
#[derive(Clone, Debug)]
pub struct UpwardPin {
    /// The child's own signed checkpoint — the exact bytes that were submitted
    /// upward, published by this same call so the two cannot disagree.
    pub checkpoint: PublishedCheckpoint,

    /// What the parent reported after recording those bytes: the core
    /// [`SubmissionOutcome`], unchanged and unwrapped.
    ///
    /// Its type is the one every submitter gets. That is the DP14 requirement
    /// held structurally — the cascade is *the same mechanism*, so it returns
    /// *the same value*, and a reader who wants to check that reads this line
    /// rather than a paragraph asserting equivalence.
    pub recorded: SubmissionOutcome,
}

/// Publishes `child`'s checkpoint and submits it to `parent`.
///
/// `context` is what the **caller** established about the child as a submitter,
/// and it is passed to the parent's admission policy exactly as any other
/// submission's would be. This function establishes nothing of its own and
/// asserts nothing of its own: an anchor pinning upward is a submitter like any
/// other, and a parent that admits only authenticated peers refuses a cascade
/// on the same rule it refuses everyone else on.
///
/// The leaf the parent records is `checkpoint.note` **verbatim** — see the
/// [module docs](super) for why introducing no format here is the point rather
/// than an economy.
///
/// # Errors
///
/// - [`AnchorError::Checkpoint`](crate::AnchorError::Checkpoint) if the child
///   could not sign a checkpoint over its own log. Nothing was submitted.
/// - Otherwise exactly [`Anchor::submit`]'s errors, from the parent:
///   [`AnchorError::NotAdmitted`](crate::AnchorError::NotAdmitted) if the
///   parent's policy declined — carrying nothing, for every reason — and
///   storage or receipt failures. This function contributes no refusal of its
///   own.
///
/// **The child's log is untouched on every path**, success or failure:
/// publishing is not an append.
pub fn pin<CS, CK, CP, PS, PK, PP>(
    child: &Anchor<CS, CK, CP>,
    parent: &mut Anchor<PS, PK, PP>,
    context: SubmitterContext<'_>,
) -> AnchorResult<UpwardPin>
where
    CS: LeafStore,
    CK: InProcessSigner,
    CP: AdmissionPolicy,
    PS: LeafStore,
    PK: InProcessSigner,
    PP: AdmissionPolicy,
{
    let checkpoint = child.publish_checkpoint()?;
    // The core path, reached rather than reimplemented: two arguments, both of
    // them the ones an ordinary submission carries. The parent has no way to
    // learn that this one came from an anchor.
    let recorded = parent.submit(
        Submission {
            statement: checkpoint.note.as_bytes(),
        },
        context,
    )?;
    Ok(UpwardPin {
        checkpoint,
        recorded,
    })
}

#[cfg(test)]
#[path = "pin_tests.rs"]
mod tests;
