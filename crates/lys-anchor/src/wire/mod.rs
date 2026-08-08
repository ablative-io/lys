//! The values that cross an anchor's boundary: what a submitter hands in, and
//! what they are handed back.
//!
//! # Invariants
//!
//! - **Nothing in this module describes what a statement means.** There is no
//!   content type, no producer, no kind, and no room reserved for one. An
//!   anchor that knew any of those would be making a judgement about the bytes
//!   it notarizes, and its receipts would then be worth only as much as that
//!   judgement.
//! - **Nothing here is a wire encoding.** These are in-memory Rust values.
//!   The only artifact that leaves an anchor with a frozen byte form is the
//!   receipt, whose encoding belongs to `lys-core`; nothing in this module
//!   serializes anything, so nothing here can drift away from a format.
//! - **Nothing here is authoritative on its own.** Every field of
//!   `SubmissionOutcome` except the receipt is a convenience a holder can
//!   recompute or check for themselves, and the receipt is the only value in
//!   it that carries a signature. [`AppendOutcome`] is the same value with that
//!   one field absent, so *nothing* in it is authoritative — which is stated on
//!   the type rather than left to be inferred from the missing field.

pub mod submission;

#[cfg(feature = "unstable-anchor")]
pub use submission::SubmissionOutcome;
pub use submission::{AppendOutcome, Submission};
