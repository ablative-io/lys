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
