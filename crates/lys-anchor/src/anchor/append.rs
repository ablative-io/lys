//! [`Anchor::append`] — put a statement to the admission policy and, if it is
//! admitted, add it to the log.
//!
//! # Why this is not behind `unstable-anchor`, and what went wrong when it was
//!
//! Until this file existed, an anchor's only mutator was `Anchor::submit`, and
//! `submit` is gated because its **return value** carries a draft wire format:
//! an `AnchorReceipt`. The gate was therefore on the verb for the sake of the
//! noun, and the cost was not cosmetic — **a default-features anchor was frozen
//! at tree size 1 forever.** `create` wrote genesis and nothing else could
//! append, so the only leaf the *ungated*
//! [`Anchor::inclusion_artifact`](crate::Anchor::inclusion_artifact) could ever
//! describe was genesis.
//!
//! (`submit` is named above in plain backticks rather than as an intra-doc
//! link, deliberately. **A link from ungated docs to a gated item resolves
//! under `--all-features` and breaks the default `cargo doc`**, so the
//! feature-full doc run is structurally blind to it — the exact regression
//! CLAUDE.md added the second doc gate for. It was written as a link first, and
//! the default doc run is what caught it.)
//!
//! That makes an availability claim recorded elsewhere in this crate wrong as
//! it was written: ungating the JSON artifact was said to put DP2 "in the
//! feature graph instead of only in prose", and it did not. **A reachable API
//! over an unreachable state is not availability.** The artifact was reachable;
//! the second leaf was not.
//!
//! Appending is not the draft. It is `Log::append` — an RFC 6962 operation that
//! predates this crate and freezes no format — and nothing it returns has a
//! byte encoding at all. So the two are split: the append is here and ungated,
//! and `submit` is this followed by `receipt_for`, unchanged in behaviour.
//!
//! # Invariants
//!
//! - **Admission runs here, not in `submit`.** DP23's policy is the operator's
//!   rule about which statements enter the log, and it is a property of the
//!   append rather than of the receipt: an ungated append that skipped the
//!   policy would give a default build a write path the operator never
//!   authorised, which is a worse defect than the one this file fixes. The
//!   policy is consulted **before** the append and a refusal leaves no trace,
//!   for the reason `submit`'s docs already give — an append-only log cannot
//!   remove an entry it should not have taken.
//! - **The refusal is one value.** Every reason any policy has arrives as
//!   [`NotAdmitted`](crate::admission::NotAdmitted), which carries nothing, and
//!   leaves as [`AnchorError::NotAdmitted`], which also carries nothing. An
//!   admission decision is a function of the submitted bytes, so a
//!   distinguishable refusal would be an oracle for the rule.
//! - **This method cannot sign, and the compiler is what says so.** Its `impl`
//!   block names no bound on the anchor's signer parameter. An anchor whose
//!   `K` is [`NoSigner`](crate::NoSigner) has no signing method in scope at
//!   all, and `append` is still callable on it if its policy is real — so
//!   "appending signs nothing" is a resolution fact rather than a claim in a
//!   comment.
//! - **Nothing the anchor returns from here is signed.** [`AppendOutcome`]
//!   holds three numbers the anchor read off its own tree. `wire::submission`
//!   states the consequence on the type.
//! - **A duplicate append is not a duplicate.** Appending identical bytes twice
//!   appends twice, at two indices. This crate performs no de-duplication and
//!   has nowhere to keep an index of seen statements; an append-only log's
//!   answer to a repeat is that it saw two events.

use lys_log_store::LeafStore;

use crate::admission::{AdmissionPolicy, SubmitterContext};
use crate::error::{AnchorError, AnchorResult};
use crate::wire::{AppendOutcome, Submission};

use super::open::Anchor;

impl<S: LeafStore, K, P: AdmissionPolicy> Anchor<S, K, P> {
    /// Puts `submission` to the anchor's admission policy, under `context`,
    /// and — if it is admitted — appends its statement bytes to the log.
    ///
    /// The statement is stored verbatim: not canonicalized, not parsed, not
    /// length-prefixed by this crate. `context` is what the *caller*
    /// established about the submitter; it is read only by the policy and is
    /// stored nowhere, so the leaf carries nothing about who sent it.
    ///
    /// **Nothing is signed, and nothing in the returned value is a claim.** A
    /// caller who wants an artifact a stranger can check asks for one
    /// afterwards: [`Anchor::inclusion_artifact`] in any build, or
    /// `Anchor::receipt_for` in a build with `unstable-anchor`. Both are
    /// statements about the tree *at the size they are taken at*, so a caller
    /// who needs one to describe exactly the tree this append produced must
    /// take it before anything else appends, and check the sizes agree rather
    /// than assume it.
    ///
    /// # Errors
    ///
    /// [`AnchorError::NotAdmitted`] if the policy refused, in which case
    /// **nothing was appended** and the returned value is identical for every
    /// reason any policy might have had. It discloses neither the rule nor the
    /// policy.
    ///
    /// [`AnchorError::Store`] if the append fails — including
    /// `StoreError::Poisoned`, which means an earlier append on this handle
    /// stored its leaf and then failed before advancing the pin.
    pub fn append(
        &mut self,
        submission: Submission<'_>,
        context: SubmitterContext<'_>,
    ) -> AnchorResult<AppendOutcome> {
        // Before the append, and the refusal is discarded rather than mapped:
        // `NotAdmitted` carries nothing, so there is nothing to carry across,
        // and the variant it becomes has nowhere to put it if there were.
        self.policy
            .admit(&submission, &context)
            .map_err(|_refusal| AnchorError::NotAdmitted)?;
        let (leaf_index, leaf_hash) = self.log.append(submission.statement)?;
        Ok(AppendOutcome {
            leaf_index,
            // Read from the tree, the same source `publish_checkpoint` reads,
            // rather than counted or remembered here.
            tree_size: self.tree_size(),
            leaf_hash,
        })
    }
}

#[cfg(test)]
#[path = "append_tests.rs"]
mod tests;
