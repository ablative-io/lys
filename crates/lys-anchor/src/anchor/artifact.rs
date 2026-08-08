//! [`Anchor::inclusion_artifact`] — the JSON proof of inclusion, the artifact a
//! stranger with no lys tooling can check.
//!
//! # This module is not behind `unstable-anchor`, and that is the whole point
//!
//! `submit` and the COSE receipt it returns are gated: they depend on
//! `lys_core::receipt`, a draft wire format that is not ratified and must stay
//! changeable. `lys_core::tlog` is not gated, so nothing forces this artifact
//! to be — and DP2 says it must not be. The rule it protects is the repository's
//! first one: *a receipt that only specialised tooling can check violates
//! "verification must outlive the vendor"*. Leaving the JSON proof behind the
//! same gate as the receipt would have inverted exactly that. It would have
//! meant the artifact any stock tooling can verify is the *opt-in* one, and the
//! draft binary artifact is what a default build gets.
//!
//! So the property is expressed in the feature graph rather than in prose: the
//! shape consumers get by default includes the inclusion artifact and excludes
//! the draft receipt. That is checkable by building, not by reading.
//!
//! # Invariants
//!
//! - **Every byte of the artifact comes from `lys-core`.** This module assembles
//!   no JSON, encodes no base64 and signs no note; it selects a leaf and calls
//!   `build_inclusion_artifact`. A second copy of a canonical encoder is the one
//!   thing a suite that only tests itself can never catch drifting, and the
//!   artifact is a frozen wire contract, so the copy count stays at one.
//! - **The artifact is self-verified before it is returned, by `lys-core`, over
//!   the third party's verification path.** That is why the builder takes the
//!   leaf bytes, and why they are read back out of the log at the index the
//!   caller named rather than passed in by the caller. An artifact this method
//!   returns has already been checked the way its reader will check it.
//! - **The embedded checkpoint is signed under the log's own origin,** read
//!   through to storage. `verify_checkpoint` refuses a note whose body origin
//!   differs from the verifier's name, so one log's artifact can never be
//!   accepted for another.
//! - **Producing an artifact is not an append.** `&self`, no write, no leaf. An
//!   anchor does not log the proofs it hands out.
//! - **A genesis-only anchor still gets an artifact.** `receipt_for` refuses a
//!   tree of size 1 because RFC 9942 types a receipt's inclusion path as
//!   one-or-more nodes and a one-leaf tree's path is empty. The JSON artifact
//!   has no such cardinality rule: an empty `hashes` array is the correct proof
//!   that the sole leaf of a one-leaf tree is its root, and `lys-core` builds,
//!   self-verifies and accepts it. The two surfaces differ here, and this
//!   module does not invent the receipt's restriction to make them look alike —
//!   a refusal copied from a constraint that does not apply is a refusal with
//!   no reason behind it.
//!
//! # A receipt and an artifact taken at two moments will disagree, and neither is wrong
//!
//! `receipt_for` already documents that a receipt is a statement about a tree
//! *at a size*, so two calls at two sizes yield two valid, non-superseding
//! receipts. The same is true here and the consequence is louder, because the
//! artifact carries a **freshly signed checkpoint** rather than a bare size:
//!
//! ```text
//! outcome  = anchor.submit(…)?          // receipt over the tree at size N
//! anchor.submit(…)?                     // somebody else's statement, size N+1
//! artifact = anchor.inclusion_artifact(outcome.leaf_index)?   // size N+1
//! ```
//!
//! Both describe the same leaf. They declare different `tree_size` values and
//! commit to different roots, and both verify — against different trees. Nothing
//! is corrupt; they are photographs of one leaf taken at two moments, and an
//! append-only log guarantees the later one contains the earlier.
//!
//! **What a caller who needs them to agree must do:** take both against the same
//! tree, by calling `inclusion_artifact` immediately after `submit` returns with
//! no append in between — and then *check* the agreement instead of assuming it,
//! because on a shared anchor "in between" is not a property this crate can
//! promise. The check is available to the caller and cheap: the artifact's
//! `tree_size` must equal the outcome's, and the root inside the artifact's
//! verified checkpoint must equal the receipt's `reconstructed_root(statement)`.
//! Both come from the caller's own two values, so this is a comparison a caller
//! can make and this module cannot make for them.
//!
//! ## Why `submit` does not simply return the artifact too
//!
//! It would remove the window, and it is refused anyway. `SubmissionOutcome` is
//! behind `unstable-anchor`; putting an ungated artifact inside a gated struct
//! makes the artifact unreachable in a default build, which is precisely the
//! inversion the top of this file exists to prevent. The convenience would be
//! bought by giving up the property.
//!
//! The honest fix for a caller who needs an atomic pair is not a bundled return
//! value but an artifact *at a stated size*: `Log::prefix_tree` can rebuild the
//! tree at any historical size, so an `inclusion_artifact_at(leaf_index,
//! tree_size)` is buildable and would let a caller name the moment instead of
//! racing it. It is deliberately not built here — the anchor's method set is
//! specified without it, and adding an unrequested second entry point to a
//! surface this young is how a wire contract acquires a variant nobody asked
//! for. The route is written down so the next person weighing the window has
//! the option in front of them rather than rediscovering it.

use lys_core::tlog::{InclusionProofArtifact, build_inclusion_artifact};
use lys_log_store::LeafStore;

use crate::admission::AdmissionPolicy;
use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;

use super::open::Anchor;

impl<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy> Anchor<S, K, P> {
    /// Builds the `lys/log-inclusion-proof/v1` JSON artifact for the leaf
    /// already at `leaf_index`, against the log's current size.
    ///
    /// The artifact is self-contained: the RFC 6962 inclusion path and a signed
    /// checkpoint over the root it leads to, so a reader needs the leaf bytes
    /// and this anchor's public key and nothing else. It is emitted regardless
    /// of whether a receipt was or could be issued — see the
    /// [module docs](self) for why that is not a convenience.
    ///
    /// Like a receipt, an artifact is a statement about a tree *at a size*, and
    /// unlike a receipt it embeds a freshly signed checkpoint saying so. Two
    /// calls at two sizes give two valid artifacts with different `tree_size`
    /// values and different roots; neither supersedes the other. The
    /// [module docs](self) say what a caller who needs one to agree with a
    /// receipt has to do about it.
    ///
    /// # Errors
    ///
    /// - [`AnchorError::NoSuchLeaf`] if `leaf_index` is not in the log. The same
    ///   variant `receipt_for` returns, for the same condition — a second name
    ///   for one fact is a second thing to keep in step.
    /// - [`AnchorError::InclusionArtifact`] if `lys-core` refuses to build the
    ///   artifact or its own self-verification of it fails. Unreachable for an
    ///   in-range index on a well-formed anchor, and propagated with its cause
    ///   rather than assumed away.
    ///
    /// There is no size-1 refusal here. A genesis-only anchor has a valid
    /// inclusion artifact for leaf 0, with an empty path.
    pub fn inclusion_artifact(&self, leaf_index: u64) -> AnchorResult<InclusionProofArtifact> {
        let tree_size = self.tree_size();
        let Some(leaf) = self.log.leaf_bytes(leaf_index) else {
            return Err(AnchorError::NoSuchLeaf {
                origin: self.origin().to_string(),
                leaf_index,
                tree_size,
            });
        };
        build_inclusion_artifact(
            self.log.tree(),
            leaf,
            self.origin(),
            self.signer.identity(),
            leaf_index,
        )
        .map_err(|source| AnchorError::InclusionArtifact {
            origin: self.origin().to_string(),
            leaf_index,
            tree_size,
            source,
        })
    }
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
