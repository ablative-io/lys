//! [`Anchor::submit`] — append a statement and issue the receipt for it — and
//! [`Anchor::receipt_for`], which issues one for a leaf already logged.
//!
//! # `submit` is `append` plus `receipt_for`, and the split is deliberate
//!
//! The policy consultation, the append and the tree read live in
//! [`anchor::append`](super::append), which is **not** behind
//! `unstable-anchor`. `submit` calls it and adds the one thing that is a draft
//! format — the receipt. The gate belongs on the noun: an anchor that could not
//! append without the feature was frozen at tree size 1 in a default build, and
//! `append`'s module docs carry the full account.
//!
//! Everything the invariants below assert about the *order* of admission and
//! append is therefore held by `append` and asserted in its tests; it is
//! restated here because `submit` is where a reader looks for it, not because
//! there are two copies of the code.
//!
//! # Invariants
//!
//! - **The statement is appended before anything is signed, and the receipt is
//!   built from the tree that resulted.** The order is not an implementation
//!   detail: a receipt produced before the append would vouch for a root that
//!   did not yet contain the leaf, and one produced from a remembered size
//!   would vouch for a tree state nobody could reproduce from storage.
//! - **The anchor signs a root it derived, never one it was handed.** The
//!   inclusion path comes from this anchor's own tree and `sign_receipt`
//!   recomputes the root from it. Nothing a submitter sends participates in
//!   the signed value except the leaf bytes themselves.
//! - **A duplicate submission is not a duplicate.** Submitting identical bytes
//!   twice appends twice, at two indices, and yields two receipts that each
//!   verify at their own index. This crate performs no de-duplication, holds
//!   no index of seen statements, and has nowhere to keep one: recognising a
//!   repeat would require deciding that two byte strings *mean* the same
//!   thing, and an append-only log's answer is that it saw two events.
//! - **`submit` interprets nothing, and the admission policy is not `submit`
//!   interpreting something.** There is still no parse, no validation and no
//!   size rule *in this file*: the one decision that depends on a submission's
//!   content is delegated whole to the [`AdmissionPolicy`] the operator
//!   constructed the anchor with, and this module's entire part in it is to ask
//!   before appending and to convert a refusal into a value carrying nothing.
//!   An anchor with [`AcceptAll`](crate::AcceptAll) has, as before, no code
//!   path in which a statement's content changes the outcome.
//! - **What was established about the submitter travels beside the statement,
//!   never inside it.** [`SubmitterContext`] is a second argument to `submit`,
//!   not a field of [`Submission`], and it never reaches a leaf: the log holds
//!   the statement bytes and nothing about who sent them. That keeps
//!   [`Submission`] to the one field §3.2 specified, and it puts the
//!   provenance of a credential — asserted by the submitter, or authenticated
//!   by a transport — in a type whose variants a policy has to tell apart.
//! - **Admission is decided before the append, and a refusal leaves no trace.**
//!   The order is the invariant: a policy consulted after the append would have
//!   an append-only log holding entries it refused, with no way to remove them.
//!   So the log is what was admitted — and it records nothing about *why*,
//!   because it holds no rule and no policy identity.
//! - **The refusal is one value.** Every reason any policy has for refusing
//!   arrives here as [`NotAdmitted`](crate::admission::NotAdmitted), which
//!   carries nothing, and leaves as
//!   [`AnchorError::NotAdmitted`], which also carries nothing. That is the
//!   collapse `error`'s module docs demanded before this path existed: an
//!   admission decision is a function of the submitted bytes, so a
//!   distinguishable refusal would be an oracle for the policy.
//!
//! # Why a receipt needs two leaves, and why that is genesis's job
//!
//! `lys-core` refuses to sign a receipt over a tree of size 1: RFC 9942 types
//! the inclusion path as one-or-more nodes, and the sole leaf of a one-leaf
//! tree has an empty path, so the artifact could not conform. The remedy is
//! structural and already taken — an anchor's leaf 0 is its genesis leaf,
//! written at creation and impossible to add afterwards — so by the time any
//! submission exists the tree is at least size 2 and every path has at least
//! one node. [`Anchor::receipt_for`] can still be asked for a receipt on an
//! anchor that holds only genesis, and refuses it by name rather than letting
//! `lys-core`'s message surface for a condition this layer can see coming.

use lys_core::receipt::{AnchorReceipt, sign_receipt};
use lys_log_store::LeafStore;

use crate::admission::{AdmissionPolicy, SubmitterContext};
use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;
use crate::wire::{Submission, SubmissionOutcome};

use super::open::Anchor;
use super::proof_nodes::proof_nodes;

impl<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy> Anchor<S, K, P> {
    /// Puts `submission` to the anchor's admission policy, under `context`,
    /// and — if it is admitted — appends its statement bytes to the log and
    /// returns the anchor's signed receipt for them.
    ///
    /// `context` is what the *caller* established about the submitter, and the
    /// caller is whoever is in front of this anchor: a transport that
    /// authenticated a peer, or a local process that authenticated nobody and
    /// says so with [`SubmitterContext::Unidentified`]. It is read only by the
    /// policy and is stored nowhere — the leaf is the statement bytes,
    /// verbatim, and carries nothing about who sent them.
    ///
    /// Two submissions of identical bytes produce two leaves at two indices and
    /// two receipts; nothing here de-duplicates, and the [module docs](self)
    /// say why that is the only answer an append-only log can give.
    ///
    /// # Errors
    ///
    /// [`AnchorError::NotAdmitted`] if the policy refused, in which case
    /// **nothing was appended** and the returned value is identical for every
    /// reason any policy might have had. It is the same value a different
    /// policy would have produced, so it discloses neither the rule nor the
    /// policy.
    ///
    /// [`AnchorError::Store`] if the append fails — including
    /// `StoreError::Poisoned`, which means an earlier append on this handle
    /// stored its leaf and then failed before advancing the pin. Note the
    /// consequence: an append that succeeded followed by a receipt that could
    /// not be issued leaves the statement **logged**. That is the correct
    /// residue for an append-only log — the alternative would be removing a
    /// leaf, which `LeafStore` offers no way to do and should not — and the
    /// remedy is [`Anchor::receipt_for`] at the returned index, which is why
    /// that entry point exists separately.
    ///
    /// Otherwise whatever `receipt_for` returns.
    pub fn submit(
        &mut self,
        submission: Submission<'_>,
        context: SubmitterContext<'_>,
    ) -> AnchorResult<SubmissionOutcome> {
        // The policy consultation, the append and the tree read are all
        // `append`'s, reached rather than repeated: two copies of the
        // admission-then-append order would be two places for the order to
        // drift, and the order is the invariant.
        let appended = self.append(submission, context)?;
        let receipt = self.receipt_for(appended.leaf_index)?;
        Ok(SubmissionOutcome {
            leaf_index: appended.leaf_index,
            tree_size: appended.tree_size,
            leaf_hash: appended.leaf_hash,
            receipt,
        })
    }

    /// Issues a receipt for the leaf already at `leaf_index`, against the
    /// log's current size.
    ///
    /// A receipt is a statement about a tree *at a size*, so calling this
    /// twice for one leaf at two different sizes yields two different, equally
    /// valid receipts with different roots. Neither supersedes the other.
    ///
    /// # Errors
    ///
    /// - [`AnchorError::TreeTooSmallForReceipt`] if the log holds only its
    ///   genesis leaf — see the [module docs](self).
    /// - [`AnchorError::NoSuchLeaf`] if `leaf_index` is not in the log.
    /// - [`AnchorError::Receipt`] if `lys-core` refuses to prove inclusion or
    ///   to sign. Both are unreachable for an in-range index on a well-formed
    ///   anchor, and are propagated with their cause rather than assumed away.
    /// - [`AnchorError::MalformedInclusionPath`] if the proof's byte length is
    ///   not a whole number of digests, which a tree cannot produce.
    pub fn receipt_for(&self, leaf_index: u64) -> AnchorResult<AnchorReceipt> {
        let tree_size = self.tree_size();
        if tree_size < 2 {
            return Err(AnchorError::TreeTooSmallForReceipt {
                origin: self.origin().to_string(),
                tree_size,
            });
        }
        let Some(leaf) = self.log.leaf_bytes(leaf_index) else {
            return Err(AnchorError::NoSuchLeaf {
                origin: self.origin().to_string(),
                leaf_index,
                tree_size,
            });
        };
        let receipt_error = |source| AnchorError::Receipt {
            origin: self.origin().to_string(),
            leaf_index,
            tree_size,
            source,
        };
        let proof = self
            .log
            .tree()
            .prove_inclusion(leaf_index)
            .map_err(receipt_error)?;
        let path = proof_nodes(proof.as_bytes())?;
        sign_receipt(leaf, leaf_index, tree_size, &path, self.signer.identity())
            .map_err(receipt_error)
    }
}

#[cfg(test)]
#[path = "submit_tests.rs"]
mod tests;
