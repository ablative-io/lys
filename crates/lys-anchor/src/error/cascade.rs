//! Refusals about assembling a verification bundle from a cascade of
//! anchors notarizing one another.

/// Why a verification bundle could not be assembled from a cascade.
///
/// `#[non_exhaustive]` for the reason [`AnchorError`](super::AnchorError)
/// is: a new precondition in this family gets its own name without a major
/// version bump.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CascadeError {
    /// A cascade was handed to bundle assembly with more links than
    /// `lys-core`'s `MAX_LINKS` cap allows.
    ///
    /// Refused at assembly rather than emitted, because the workspace already
    /// knows the answer: `verify_bundle` rejects a bundle past the cap, so
    /// producing one would hand a caller an artifact nothing can accept. The
    /// cap exists so an untrusted bundle cannot ask a verifier for unbounded
    /// work, and a chain this deep is pathological on its own terms — each link
    /// is one anchor notarizing the one below it.
    ///
    /// **`max` is carried as a field rather than written into the text as a
    /// literal**, so the message reports the cap the code actually enforced
    /// instead of the one this sentence remembers.
    #[error(
        "refusing to assemble a verification bundle for {origin} from a cascade of {links} links: a verifier accepts at most {max}, so a deeper chain would produce an artifact nothing can check"
    )]
    CascadeTooDeep {
        /// The origin of the log the bundle was being assembled for.
        origin: String,
        /// The number of links the cascade held.
        links: usize,
        /// The cap, as read from `lys-core` at the point of refusal.
        max: usize,
    },

    /// The first link of the cascade does not notarize the checkpoint the
    /// freshly built inclusion artifact carries.
    ///
    /// A bundle's first link is what joins the notarization to the inclusion
    /// proof: `verify_bundle` requires `links[0].checkpoint` to be
    /// **byte-identical** to `inclusion_proof.checkpoint`, or the notarization
    /// is about some other log whose checkpoint also happens to verify.
    ///
    /// **The usual cause is an append between the pin and the assembly.** An
    /// inclusion artifact embeds a checkpoint signed over the tree at the moment
    /// it is built, so a statement admitted in between moves the artifact to a
    /// later size while the pinned checkpoint still states the earlier one. Both
    /// notes are valid and neither is corrupt; they are photographs of one log
    /// taken at two moments. The remedy is to pin again from the current tree,
    /// or to assemble the bundle from the artifact taken at the size that was
    /// pinned.
    ///
    /// Both sizes are named because that is what distinguishes this from a pin
    /// against an entirely different log, and neither discloses anything: a
    /// tree size is the second line of every checkpoint this anchor signs and
    /// publishes.
    #[error(
        "the cascade's first link notarizes a checkpoint of {origin} at tree size {pinned_tree_size}, but the inclusion artifact for leaf {leaf_index} carries one at tree size {artifact_tree_size}: a bundle's first link must notarize the very checkpoint its inclusion proof was verified against"
    )]
    CascadeJoinMismatch {
        /// The origin of the log the bundle was being assembled for.
        origin: String,
        /// The index the bundle was being assembled for.
        leaf_index: u64,
        /// The tree size the pinned checkpoint committed to.
        pinned_tree_size: u64,
        /// The tree size the inclusion artifact's checkpoint committed to.
        artifact_tree_size: u64,
    },
}
