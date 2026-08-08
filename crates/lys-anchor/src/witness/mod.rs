//! Witnessing — recording another log's checkpoint, and reporting what this
//! anchor's own memory says about it.
//!
//! A witness is **a durable memory, not an auditor.** The whole of the surface
//! here is: take a checkpoint note as an ordinary submission, then say how it
//! sits against the `(size, root)` this same witness recorded for that origin
//! earlier. The first half is the core submit path, reached through
//! [`Anchor::submit`] and changed in no way by being reached from here; the
//! second half is a report nobody signs.
//!
//! # Record, then check, then report — and the order is the invariant
//!
//! [`observe()`] appends first. The record is **unconditional**: no relation this
//! module can compute prevents the append, there is no branch between the
//! submission and the comparison, and an [`Observation`] cannot be built at all
//! without the [`SubmissionOutcome`](crate::wire::SubmissionOutcome) of a
//! submission that already completed.
//!
//! The reason is not tidiness. A witness that refused to record an equivocating
//! checkpoint would destroy the only evidence the detection mechanism runs on:
//! equivocation is caught by **two durable memories disagreeing**, and a
//! memory that declines to hold the second half of the disagreement has nothing
//! to disagree with. So the conflicting checkpoint is logged, with a receipt,
//! exactly like any other statement — and the accusation is a report the
//! operator reads, pointing at two leaves anybody can fetch and check.
//!
//! # The witness endorses nothing, and that is carried by shape
//!
//! **The artifact a witness emits is the artifact an ordinary submission emits,
//! produced by the same function with no extra input.** [`observe()`] calls
//! [`Anchor::submit`]. There is no witness receipt type, no witness field, no
//! flag, and nowhere at all that "I also checked this" could be encoded — so a
//! reader cannot infer endorsement from a receipt, because the receipt does not
//! distinguish the two cases. That is a property of the shape rather than of
//! this paragraph, and `observe_tests` asserts it byte-for-byte.
//!
//! [`Observation::previous`] and [`Observation::relation`] sit **outside every
//! signature**, deliberately. They are this witness's report of what it
//! observed, and nothing in them reaches a signed byte.
//!
//! # What a witness is explicitly NOT asserting
//!
//! - **Not that the submitter's log is honest.** The witness holds one
//!   `(size, root)` pair. A log that forked before the witness ever saw it
//!   presents a perfectly consistent history from that point.
//! - **Not anything about the stretch it never saw.** Between two observations,
//!   and before the first, a witness knows nothing. Consistency between two
//!   roots is a statement about two hashes, not about the entries between them.
//!   This is the sharpest limit here.
//! - **Not that the leaf contents are true.** The log makes no claim about
//!   whether an entry is true, only that it is recorded.
//! - **Not well-formedness.** Nothing here parses a submission in order to
//!   judge it; [`projection`] parses recorded leaves only to build the memory,
//!   and a leaf that does not parse is simply not in it.
//! - **Not a timestamp.** No time enters the receipt. Time comes from the
//!   ordering of the witness's own log and from other parties' corroboration.
//! - **Not that the witness itself is honest.** A witness's own log carries the
//!   same limit every log carries: with no third party keeping a memory of
//!   *it*, it can equivocate undetectably too.
//! - **Not that the check in [`observe()`] happened at all.** The receipt is
//!   identical either way, by construction. The durable evidence is the
//!   recorded note; the check is a report.
//!
//! # Federation calls the core; the core never reaches back
//!
//! Everything in this module is behind the off-by-default `federation` feature,
//! and no item outside it names anything inside it. The compiler is the second
//! party for that claim: a core path that reached a federation item would break
//! the default build, which is a gate.
//!
//! [`Anchor::submit`]: crate::Anchor::submit

pub mod observe;
pub mod projection;
pub mod report;

pub use observe::observe;
pub use projection::{OriginState, WitnessProjection, checkpoint_in_leaf};
pub use report::{Observation, Relation};
