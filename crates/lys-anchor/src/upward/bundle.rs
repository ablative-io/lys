//! [`bundle_for`] — package a leaf, its inclusion proof, and a cascade into a
//! `lys/verification-bundle/v1`.
//!
//! # This crate assembles; `lys-core`'s `verify_bundle` judges
//!
//! Every byte of the container comes from `lys-core` — `VerificationBundle::new`
//! sets the frozen format string and the base64 alphabet, `BundleLink::new`
//! encodes the receipt, and the inclusion artifact is embedded verbatim as the
//! structure it already is. This module assembles no JSON and encodes no
//! base64, for the reason [`anchor::artifact`](crate::anchor::artifact) gives:
//! a second copy of a canonical encoder is the one thing a suite that only
//! tests itself can never catch drifting.
//!
//! The division of labour goes further than encoding, and it is deliberate:
//!
//! - **`verify_bundle` checks the relationships, and this module does not
//!   duplicate it.** The join at link 0, each receipt against its own leaf, and
//!   the rung that forces `links[i+1].checkpoint` to be anchor `i`'s own
//!   note-signed checkpoint stating exactly the root and size receipt `i`
//!   vouched for — all of that is the judge's, and it is mandatory there rather
//!   than advisory. A producer that re-implemented those checks would be a
//!   second copy of the verifier, agreeing with itself.
//! - **This module refuses exactly the two things it is in a position to have
//!   caused**, both of them local to assembly and neither of them a
//!   relationship between links.
//!
//! # The two refusals, and why they are the producer's and not the judge's
//!
//! 1. **A cascade deeper than `MAX_LINKS`** (32, `lys-core`'s cap so an
//!    untrusted bundle cannot ask a verifier for unbounded work). `verify_bundle`
//!    rejects such a bundle, so emitting one would be handing a caller an
//!    artifact this workspace already knows nothing can accept. Refused at
//!    assembly with the depth named, which is a fact about the operator's own
//!    chain.
//! 2. **A first link whose checkpoint is not the one the freshly built
//!    inclusion artifact carries.** This is a race *this crate creates*: an
//!    inclusion artifact embeds a checkpoint signed over the tree **at the
//!    moment it is built**, so an append between [`pin()`](super::pin::pin()) and
//!    this call yields an artifact at a later size, and the join `verify_bundle`
//!    requires — `links[0].checkpoint` byte-identical to
//!    `inclusion_proof.checkpoint` — no longer holds. The judge would reject it
//!    as [one indistinguishable failure][non-oracle]; the operator would learn
//!    only "bundle verification failed" about a bundle their own anchor moved
//!    underneath them. So it is named here, where the caller can act on it, and
//!    the judge still checks it for the stranger, who is the only reader whose
//!    check counts.
//!
//! [non-oracle]: crate::AnchorError::CascadeJoinMismatch
//!
//! # An empty cascade is a bundle, and it says something true
//!
//! `chain` may be empty, and the result is a bundle with no links: *this leaf
//! is in this log, and nobody has notarized that log*. `verify_bundle` accepts
//! it and reports it as an empty notarization chain rather than pretending it
//! failed. That is a weaker claim, not an invalid one, and refusing to package
//! it here would leave the weaker claim unpackageable — which is how an
//! operator ends up assembling one by hand.
//!
//! # Nothing here signs anything, and there is nowhere for a signature to go
//!
//! The bundle is **never signed**, by absence rather than by convention:
//! `VerificationBundle` has no signature field and `lys-core` ships no `sign`
//! for it. A signature over the container would invite a verifier to check the
//! wrapper and skip the contents, and in a forgery that failure has a specific
//! shape — a green check that verified nothing. The `counter_anchor` slot is
//! likewise left empty, because `verify_bundle` **refuses** a populated one
//! until something exists that can check a time attestation.

use lys_core::bundle::{BundleLink, MAX_LINKS, VerificationBundle};
use lys_log_store::LeafStore;

use crate::admission::AdmissionPolicy;
use crate::anchor::Anchor;
use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;

use super::pin::UpwardPin;

impl UpwardPin {
    /// Renders this rung as the chain link a verification bundle carries.
    ///
    /// The checkpoint travels as the note verbatim, trailing newline included,
    /// and the receipt as `lys-core`'s own base64 of its exact tagged
    /// `COSE_Sign1` bytes. Both encodings are `BundleLink::new`'s, so a caller
    /// cannot get the alphabet or the padding subtly wrong and this crate does
    /// not hold a second opinion about either.
    pub fn to_bundle_link(&self) -> BundleLink {
        BundleLink::new(
            &self.checkpoint.note,
            &self.recorded.receipt.to_cose_bytes(),
        )
    }
}

/// Assembles the verification bundle for the leaf at `leaf_index` in `anchor`,
/// notarized by `chain`.
///
/// `chain` is **ascending**, and its order is the cascade's own: `chain[0]` is
/// the pin of *this* anchor's checkpoint to its parent, `chain[1]` the pin of
/// that parent's checkpoint to its own parent, and so on. That is the order
/// `pin` produces them in when a cascade is walked upward, and the order
/// `verify_bundle` validates rather than assumes.
///
/// For the bundle to verify, each parent must publish the checkpoint it hands
/// on **immediately after** recording its child's, with no append in between:
/// the rung requires `chain[i+1].checkpoint` to state exactly the root and size
/// `chain[i]`'s receipt vouched for. That is a sequencing requirement on the
/// cascade rather than on this call, and it is checked by the judge.
///
/// # Errors
///
/// - [`AnchorError::CascadeTooDeep`] if `chain` is longer than `MAX_LINKS`.
/// - [`AnchorError::NoSuchLeaf`] if `leaf_index` is not in the log — propagated
///   from [`Anchor::inclusion_artifact`], which is the one place that refusal is
///   made. A second guard here would be a check no drift could distinguish from
///   that one, and two guards on one rule leave the rule proven by neither.
/// - [`AnchorError::InclusionArtifact`] if the artifact could not be built or
///   failed `lys-core`'s own self-verification.
/// - [`AnchorError::CascadeJoinMismatch`] if `chain[0]`'s checkpoint is not the
///   one the artifact carries — see the [module docs](self) for the race that
///   causes it and the remedy.
pub fn bundle_for<S, K, P>(
    anchor: &Anchor<S, K, P>,
    leaf_index: u64,
    chain: &[UpwardPin],
) -> AnchorResult<VerificationBundle>
where
    S: LeafStore,
    K: InProcessSigner,
    P: AdmissionPolicy,
{
    if chain.len() > MAX_LINKS {
        return Err(AnchorError::CascadeTooDeep {
            origin: anchor.origin().to_string(),
            links: chain.len(),
            max: MAX_LINKS,
        });
    }

    // Refuses a missing index; the artifact self-verifies before it is returned.
    let artifact = anchor.inclusion_artifact(leaf_index)?;

    if let Some(first) = chain.first() {
        if first.checkpoint.note != artifact.checkpoint {
            return Err(AnchorError::CascadeJoinMismatch {
                origin: anchor.origin().to_string(),
                leaf_index,
                pinned_tree_size: first.checkpoint.body.tree_size(),
                artifact_tree_size: artifact.tree_size,
            });
        }
    }

    // The artifact was built for this index, so the leaf is in the log; the
    // `else` is the refusal this crate owes rather than an assumption it makes,
    // and it reports the same fact under the same name the artifact would have.
    let Some(leaf) = anchor.leaf_bytes(leaf_index) else {
        return Err(AnchorError::NoSuchLeaf {
            origin: anchor.origin().to_string(),
            leaf_index,
            tree_size: anchor.tree_size(),
        });
    };

    let links = chain.iter().map(UpwardPin::to_bundle_link).collect();
    Ok(VerificationBundle::new(leaf, artifact, links))
}

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod tests;
