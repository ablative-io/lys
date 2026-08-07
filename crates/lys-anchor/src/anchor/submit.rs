//! [`Anchor::submit`] — append a statement and issue the receipt for it — and
//! [`Anchor::receipt_for`], which issues one for a leaf already logged.
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
//! - **`submit` interprets nothing.** No parse, no validation, no size rule.
//!   Admission policy is a later increment and will be a separate,
//!   explicitly-configured object; until it exists there is no code path here
//!   in which the content of a statement changes the outcome.
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

use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;
use crate::wire::{Submission, SubmissionOutcome};

use super::open::Anchor;
use super::proof_nodes::proof_nodes;

impl<S: LeafStore, K: InProcessSigner> Anchor<S, K> {
    /// Appends `submission`'s statement bytes to the log and returns the
    /// anchor's signed receipt for them.
    ///
    /// The bytes are stored verbatim as one leaf. Two submissions of identical
    /// bytes produce two leaves at two indices and two receipts; nothing here
    /// de-duplicates, and the [module docs](self) say why that is the only
    /// answer an append-only log can give.
    ///
    /// # Errors
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
    pub fn submit(&mut self, submission: Submission<'_>) -> AnchorResult<SubmissionOutcome> {
        let (leaf_index, leaf_hash) = self.log.append(submission.statement)?;
        let receipt = self.receipt_for(leaf_index)?;
        Ok(SubmissionOutcome {
            leaf_index,
            // Read from the tree, the same source `publish_checkpoint` reads,
            // rather than counted or remembered here.
            tree_size: self.tree_size(),
            leaf_hash,
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
