#![cfg(test)]
//! Gates on [`observe`] — one rule per case, and **every negative case opens
//! with a positive control as its first assertion**.
//!
//! # The vacuity trap this file is shaped around, named
//!
//! This repo has already shipped and caught the defect these controls exist
//! for: drifting a Go content-type constant by one character left a whole
//! conformance test green, because every assertion in it said the tool
//! *refused* something — and a verifier that refuses everything satisfies all
//! of them at once. The witness equivalent is a `relate` that answers one
//! relation to everything: a suite made only of `assert_eq!(…, Conflicting)`
//! would be perfectly green against it.
//!
//! So each case below first asserts that the machinery **does not** answer its
//! target relation to an input that should not get it, and only then asserts
//! that the input built for it does. The controls are written as `assert_ne!`
//! against the case's own target rather than `assert_eq!` against some other
//! relation, deliberately: an `assert_eq!` control would be a second assertion
//! of *another* case's rule, and a drift of that rule would then fail two tests
//! instead of one.
//!
//! # One rule per case
//!
//! | rule | case |
//! |---|---|
//! | the record precedes the check | [`a_conflicting_checkpoint_is_still_appended`] |
//! | identical resubmission is a no-op | [`resubmitting_the_identical_checkpoint_is_identical`] |
//! | rollback observed | [`a_smaller_tree_size_is_rollback`] |
//! | equivocation observed | [`the_same_size_with_a_different_root_is_conflicting`] |
//! | consistency actually checked | [`a_forged_consistency_path_is_unrelated`] |
//! | no prior record implies nothing | [`a_first_sighting_reports_previous_none`] |
//! | the check never reaches the artifact | [`the_receipt_is_byte_identical_whatever_the_relation`] |
//!
//! [`a_conflicting_checkpoint_is_still_appended`] deliberately **does not**
//! assert the relation, and [`the_receipt_is_byte_identical_whatever_the_relation`]
//! deliberately does not either. Both would otherwise be second assertions of
//! rules that already have a case, and a rule with two guards is proven by
//! neither.
//!
//! # Where the second party comes from
//!
//! - **A real second log with its own key.** Every note observed here was
//!   produced by `lys-core`'s `sign_note` over a `CheckpointBody` built from a
//!   `Log`'s own root — including the equivocating and rolled-back ones, which
//!   are genuinely signed by the child. A witness cannot dismiss those as
//!   forgeries, which is exactly the case that matters.
//! - **`lys-core`'s consistency verifier**, which was written before this crate
//!   existed and does not know what a witness is.
//! - **A parallel anchor driven by the plain submit path**, for the byte
//!   identity claim: two independently driven logs, one witnessing and one not,
//!   must produce the same bytes.
//!
//! The independence axis is *implementation*, not platform: one machine, one
//! toolchain, one dependency resolution.

use lys_log_store::FileLeafStore;
use tempfile::TempDir;

use crate::admission::{AcceptAll, SubmitterContext};
use crate::anchor::Anchor;
use crate::keys::FileSigner;
use crate::wire::Submission;

use super::super::report::Relation;
use super::super::report::fixture::{
    CHILD_ORIGIN, Child, OTHER_CHILD_ORIGIN, flip_root, witness_anchor,
};
use super::*;

mod receipt;
mod relations;

/// A witness anchor over a fresh temp directory, with the directory returned so
/// it outlives the anchor.
fn staged() -> (TempDir, Anchor<FileLeafStore, FileSigner, AcceptAll>) {
    let dir = TempDir::new().unwrap();
    let anchor = witness_anchor(dir.path());
    (dir, anchor)
}

/// Observes `note` with no consistency proof, under no established identity.
fn see(
    anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>,
    note: &[u8],
) -> super::super::report::Observation {
    observe(anchor, note, None, SubmitterContext::Unidentified).unwrap()
}

/// Observes `note` with `proof`.
fn see_with(
    anchor: &mut Anchor<FileLeafStore, FileSigner, AcceptAll>,
    note: &[u8],
    proof: &[u8],
) -> super::super::report::Observation {
    observe(anchor, note, Some(proof), SubmitterContext::Unidentified).unwrap()
}
