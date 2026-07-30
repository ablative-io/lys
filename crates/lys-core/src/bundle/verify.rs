//! Verification of `lys/verification-bundle/v1` containers.
//!
//! # The one interesting failure mode
//!
//! Every artifact in a bundle verifies standalone, so the container adds no
//! cryptography — and exactly one thing it must do is not obvious:
//! **check that the artifacts are about each other.** A pile of individually
//! valid receipts proves nothing about their relationship, and a notarization
//! that is never tied to the inclusion proof is two unrelated true facts
//! presented as one chain. Skipping the link checks yields a verifier that
//! reports success having established far less than a reader would assume,
//! which is the failure this crate exists to prevent.
//!
//! So the chain is checked as follows, and none of it is optional:
//!
//! 1. `links[0].checkpoint` must be **byte-identical** to
//!    `inclusion_proof.checkpoint`. This is the join: it makes the first
//!    notarization about *the log the leaf is in*, rather than about some other
//!    log whose checkpoint also happens to verify.
//! 2. Each `links[i]`'s receipt must verify against `links[i].checkpoint` as its
//!    proven leaf, under the anchor key the **caller** supplied for that link.
//! 3. Where a next link exists, `links[i+1].checkpoint` must be anchor `i`'s own
//!    signed checkpoint, and its root and tree size must equal the root
//!    receipt `i` reconstructs and the size it claims.
//!
//! Step 3 is the rung of the ladder. It forces two independently signed
//! statements to agree: the anchor's *receipt* signature over a reconstructed
//! root, and the anchor's *note* signature over its own published checkpoint.
//! It is also what makes the receipt format's documented `tree_size`
//! malleability harmless here — a relabelled size no longer matches a
//! note-signed one.
//!
//! # Trust comes from the caller, never from the bundle
//!
//! [`verify_bundle`] takes the log's verifier key and one anchor key per link.
//! The bundle supplies none of them. A container that named the keys to trust
//! would be asserting who is trustworthy, and packaging must never become a
//! trust statement.
//!
//! One anchor key serves both roles per link: its 32 key bytes verify that
//! anchor's *receipts*, and the same key verifies that anchor's *checkpoints*.
//! That is a binding worth enforcing rather than a convenience — an anchor
//! signing receipts with a key that never appears in its published checkpoints
//! could not be cross-checked against its own log by anyone.
//!
//! # Non-oracle
//!
//! Every failure collapses to
//! [`TrustError::BundleVerification`].
//! A malformed container, a broken inclusion proof, a receipt from the wrong
//! anchor and a chain whose links do not join are deliberately
//! indistinguishable: a bundle verifier is a network-exposed surface where a
//! distinguishable error becomes a parsing oracle. Callers diagnosing their own
//! bundle verify the embedded artifacts individually, where the errors are
//! actionable.
//!
//! [`TrustError::BundleVerification`]: crate::error::TrustError::BundleVerification

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::bundle::artifact::{MAX_LINKS, VERIFICATION_BUNDLE_FORMAT, VerificationBundle};
use crate::checkpoint::{CheckpointBody, NoteVerifierKey, verify_checkpoint};
use crate::error::{TrustError, TrustResult};
use crate::receipt::verify_receipt_bytes;
use crate::tlog::verify_inclusion_artifact;

/// Non-oracle failure for every rejected bundle (see module docs).
fn reject() -> TrustError {
    TrustError::BundleVerification
}

/// What one verified rung of the chain establishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notarization {
    /// The Merkle root the anchor vouched for, **recomputed** from the
    /// notarized checkpoint and the receipt's inclusion path — never read from
    /// the artifact, which does not carry it.
    anchor_root: [u8; 32],
    /// Size of the anchor's tree when it signed that root. Where a further link
    /// follows, this has been checked against the anchor's own note-signed
    /// checkpoint; on the final link it is the receipt's claim alone.
    anchor_tree_size: u64,
    /// Index at which the notarized checkpoint sat in the anchor's tree.
    leaf_index: u64,
}

impl Notarization {
    /// The Merkle root the anchor vouched for.
    pub fn anchor_root(&self) -> [u8; 32] {
        self.anchor_root
    }

    /// Size of the anchor's tree when it signed that root.
    pub fn anchor_tree_size(&self) -> u64 {
        self.anchor_tree_size
    }

    /// Index at which the notarized checkpoint sat in the anchor's tree.
    pub fn leaf_index(&self) -> u64 {
        self.leaf_index
    }
}

/// Evidence that a bundle verified: the leaf, the log checkpoint it was proven
/// against, and one [`Notarization`] per chain link, in order.
///
/// Fields are private and the only constructor is [`verify_bundle`], so
/// **holding a `VerifiedBundle` is itself the evidence that every check passed**
/// — following the `VerifiedRequest` precedent. A value of this type cannot be
/// assembled by a caller who skipped a step.
#[derive(Clone, Debug)]
pub struct VerifiedBundle {
    leaf: Vec<u8>,
    log_checkpoint: CheckpointBody,
    notarizations: Vec<Notarization>,
}

impl VerifiedBundle {
    /// The verified leaf bytes.
    pub fn leaf(&self) -> &[u8] {
        &self.leaf
    }

    /// The log checkpoint the leaf was proven against — origin, tree size and
    /// root, all covered by the log's own signature.
    pub fn log_checkpoint(&self) -> &CheckpointBody {
        &self.log_checkpoint
    }

    /// The verified notarization chain, ascending.
    ///
    /// **An empty slice means nobody notarized this log.** The inclusion proof
    /// still holds, so the leaf is genuinely in a log whose owner signed that
    /// checkpoint — but a self-signed log is tamper-evident to one holder and
    /// cannot show that its owner keeps only one history. Callers that need
    /// notarization must check this is non-empty; the type deliberately does not
    /// pretend an unnotarized bundle failed.
    pub fn notarizations(&self) -> &[Notarization] {
        &self.notarizations
    }
}

/// Verify a bundle end to end.
///
/// `log_verifier` is the verifier key for the log the leaf was logged in.
/// `anchors` supplies one key per chain link, in the same order as
/// `bundle.links` — each verifying both that link's receipt and (where a
/// further link follows) that anchor's own checkpoint.
///
/// # Errors
///
/// Returns [`TrustError::BundleVerification`] for every failure
/// (non-oracle; see the module docs).
///
/// Note that `anchors.len()` must equal `bundle.links.len()` exactly, and a
/// mismatch is a verification failure rather than an argument error. It is a
/// substantive refusal: a bundle claiming more notarization than the verifier is
/// prepared to check cannot be checked, and silently verifying a prefix would
/// report success for less than the bundle asserts.
pub fn verify_bundle(
    bundle: &VerificationBundle,
    log_verifier: &NoteVerifierKey,
    anchors: &[NoteVerifierKey],
) -> TrustResult<VerifiedBundle> {
    // The format string is read before anything else: an unrecognised container
    // is refused rather than parsed hopefully.
    if bundle.format != VERIFICATION_BUNDLE_FORMAT {
        return Err(reject());
    }
    if bundle.links.len() > MAX_LINKS || anchors.len() != bundle.links.len() {
        return Err(reject());
    }
    // The slot exists in v1 so adding time attestation later is not a v2. Until
    // something can verify one, a populated slot is refused rather than ignored
    // — carrying an unverified attestation is how a reader comes to believe it.
    if bundle.counter_anchor.is_some() {
        return Err(reject());
    }

    let leaf = STANDARD.decode(&bundle.leaf).map_err(|_err| reject())?;

    // The inclusion proof carries its own checkpoint and is verified against it,
    // including the origin/key-name binding.
    let log_checkpoint = verify_inclusion_artifact(&bundle.inclusion_proof, &leaf, log_verifier)
        .map_err(|_err| reject())?;

    let mut notarizations = Vec::with_capacity(bundle.links.len());
    for (index, link) in bundle.links.iter().enumerate() {
        // The join, and it applies to link 0 only: link 0 must notarize the very
        // checkpoint the inclusion proof was verified against, or the
        // notarization is about some other log whose checkpoint also happens to
        // verify. Every later link's checkpoint was already validated by the
        // previous iteration's rung check below — it required that text to be
        // the previous anchor's own note-signed checkpoint with a matching root,
        // which is strictly more than an equality check here could add.
        if index == 0 && link.checkpoint != bundle.inclusion_proof.checkpoint {
            return Err(reject());
        }

        let receipt_bytes = STANDARD.decode(&link.receipt).map_err(|_err| reject())?;
        let anchor = anchors.get(index).ok_or_else(reject)?;
        let receipt = verify_receipt_bytes(
            &receipt_bytes,
            link.checkpoint.as_bytes(),
            &anchor.public_key(),
        )
        .map_err(|_err| reject())?;

        // Recompute rather than read: the root is a detached payload and appears
        // nowhere in the receipt.
        let anchor_root = receipt
            .reconstructed_root(link.checkpoint.as_bytes())
            .map_err(|_err| reject())?;

        // The rung: where a next link exists, its checkpoint must be *this*
        // anchor's own published checkpoint, signed by this anchor, stating
        // exactly the root and size this anchor's receipt vouched for. Two
        // independently signed statements forced to agree.
        if let Some(next) = bundle.links.get(index + 1) {
            let next_body =
                verify_checkpoint(next.checkpoint.as_bytes(), anchor).map_err(|_err| reject())?;
            if next_body.root_hash() != anchor_root || next_body.tree_size() != receipt.tree_size {
                return Err(reject());
            }
        }

        notarizations.push(Notarization {
            anchor_root,
            anchor_tree_size: receipt.tree_size,
            leaf_index: receipt.leaf_index,
        });
    }

    Ok(VerifiedBundle {
        leaf,
        log_checkpoint,
        notarizations,
    })
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
