//! [`Anchor::publish_checkpoint`] — the anchor signing about its own log.
//!
//! This is the smallest thing that makes "an anchor is itself a lys instance"
//! true in code rather than in a diagram: the artifact it emits is an ordinary
//! C2SP signed-note checkpoint, indistinguishable in form from one any other
//! lys log publishes, and checkable by `lys-core`'s `verify_checkpoint` and by
//! Go's `sumdb/note` without either of them knowing an anchor produced it.
//!
//! # What is signed, and what is not
//!
//! - **Signed:** the checkpoint body — origin, tree size, base64 root hash —
//!   *including its trailing newline*. That is the whole of the body bytes and
//!   nothing else.
//! - **Not signed:** the signature line itself, and any further signature line
//!   another party appends later. That is precisely what makes cosigning a
//!   published checkpoint possible without invalidating the original.
//! - **Not present at all:** any timestamp. A checkpoint says what the log held
//!   at a size, never when. Two publications of an unchanged log are therefore
//!   byte-identical, which is asserted rather than assumed.
//!
//! # The note is signed under the origin, and that binding is the point
//!
//! The signed-note *key name* is the log's origin, read through
//! [`Anchor::origin`] to storage. `verify_checkpoint` refuses a note whose body
//! origin differs from the verifier's name, so a key that signs for two logs
//! cannot have one log's checkpoint accepted by a verifier configured for the
//! other. This module chooses no name of its own; there is none available to
//! choose.
//!
//! # Publishing is not an append
//!
//! An anchor does not log its own checkpoints. `publish_checkpoint` takes
//! `&self`, the log is untouched, and the emitted note is the caller's to keep
//! or hand on. A checkpoint that appended would change the tree it had just
//! committed to, so the artifact would describe a state that no longer existed
//! the instant it was produced.

use lys_core::checkpoint::{CheckpointBody, sign_note};
use lys_log_store::LeafStore;

use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;

use super::open::Anchor;

/// A checkpoint the anchor has signed: the complete note, and the body inside
/// it.
///
/// `body` is the parsed form of exactly the bytes `note` was signed over — it
/// is the value the note was built from, not a second computation of the same
/// facts, so the two cannot disagree. A verifier should still parse `note`
/// itself: `body` is a convenience for the producer, and a consumer who trusts
/// it is trusting the sender rather than the signature.
#[derive(Clone, Debug)]
pub struct PublishedCheckpoint {
    /// The complete signed note: body, blank line, one signature line.
    pub note: String,
    /// The body those bytes commit to.
    pub body: CheckpointBody,
}

impl<S: LeafStore, K: InProcessSigner> Anchor<S, K> {
    /// Signs a C2SP checkpoint over the log's current root, under the log's own
    /// origin.
    ///
    /// The body and the signature both come from `lys-core`
    /// (`CheckpointBody::from_root` and `checkpoint::sign_note`); this crate
    /// assembles no note bytes of its own, because a second copy of a canonical
    /// encoder is the one thing a suite that only tests itself can never catch
    /// drifting.
    ///
    /// # Errors
    ///
    /// [`AnchorError::Checkpoint`] if `lys-core` refuses to encode the body or
    /// to sign the note. Both are unreachable for a well-formed anchor — the
    /// origin was validated when the store was created, and the body is
    /// machine-generated — and are propagated rather than assumed away.
    pub fn publish_checkpoint(&self) -> AnchorResult<PublishedCheckpoint> {
        let origin = self.origin();
        let checkpoint_error = |source| AnchorError::Checkpoint {
            origin: origin.to_string(),
            source,
        };
        let body =
            CheckpointBody::from_root(origin, &self.log.tree().root()).map_err(checkpoint_error)?;
        let note =
            sign_note(&body.encode(), origin, self.signer.identity()).map_err(checkpoint_error)?;
        Ok(PublishedCheckpoint { note, body })
    }
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
