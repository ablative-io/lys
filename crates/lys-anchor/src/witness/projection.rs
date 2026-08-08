//! [`WitnessProjection`] — the witness's memory of other logs, folded out of
//! its own leaves.
//!
//! # Invariants
//!
//! - **Derived, rebuildable, and never authoritative.** Nothing here is stored.
//!   A projection is a fold over leaves the anchor already holds, recomputed on
//!   demand, and discarding one loses nothing: the leaves are the record and
//!   this is a view of them. That is exactly the standing the storage pin has,
//!   and for the same reason — a cached summary that could disagree with the
//!   log would be a second copy of the truth, and the log would stop being it.
//! - **It refuses nothing and holds no `&mut Anchor`.** Every entry point here
//!   takes `&Anchor`. There is no path from a projection to an append, to a
//!   refusal, or to a signature.
//! - **The fold rule is one function, and it is public.** A leaf is in the
//!   projection if and only if [`checkpoint_in_leaf`] reads a checkpoint body
//!   out of it. Everything else — genesis, ordinary statements, malformed
//!   bytes — is skipped in silence, because an anchor that treated an
//!   unparseable leaf as an error would be an anchor with an opinion about what
//!   its submitters may log.
//! - **Last recorded wins.** For an origin appearing at several indices, the
//!   projection holds the one at the highest index — *what this witness last
//!   observed*, which is not the same as the largest tree size it ever saw. A
//!   rollback therefore moves the memory backwards, and the observation after
//!   it is compared against the rolled-back state. That is the honest reading
//!   of "what I last saw", and the record of the larger size it replaced is
//!   still in the log, at its own index, for anybody re-deriving the history.
//!
//! # ⚠️ No signature is checked here, and the consequence is real
//!
//! [`checkpoint_in_leaf`] **parses**. It does not verify the note's signature,
//! it holds no keys, and it cannot: signature verification of a submission is
//! the admission policy's business, applied uniformly to every submission,
//! because a witness path with an admission rule of its own would stop being
//! additive.
//!
//! So **anyone who can submit to this anchor can put an entry into this
//! projection**, under any origin string they like, and thereby provoke a
//! `Conflicting` report about a log that did nothing wrong. What that costs is
//! bounded and worth stating precisely:
//!
//! - It cannot alter any signed artifact. Nothing derived here reaches a
//!   signature; a witness's receipt is identical whatever this view says.
//! - It cannot make the anchor refuse anything. There is no refusal on this
//!   path.
//! - It produces a **wrong report to the operator**, and the report names two
//!   leaves. Both are durable, both are the submitted bytes verbatim, and
//!   checking which of them carries a valid signature from the origin's key is
//!   a thing the operator does with the notes in hand. A witness's report is a
//!   pointer to evidence, not a verdict — which is what "a witness records; it
//!   does not audit" means when it has to be paid for.
//!
//! An operator who wants only authenticated origins in this view gets it by
//! choosing an admission policy that admits only those submissions, which is
//! where that decision belongs and where it applies to every path equally.

use std::collections::BTreeMap;

use lys_core::checkpoint::CheckpointBody;
use lys_log_store::LeafStore;

use crate::admission::AdmissionPolicy;
use crate::anchor::Anchor;
use crate::keys::InProcessSigner;

/// One origin's `(size, root)` as this witness last recorded it.
///
/// Deliberately the same shape as the storage layer's `PinnedRoot`, and with
/// the same standing: a derived summary of leaves that remain the record. The
/// origin travels with it because a witness holds several, and a `(size, root)`
/// pair without the log it belongs to is a pair of numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginState {
    /// The origin named in the checkpoint body — the submitter's log identity,
    /// read out of the recorded bytes and never supplied alongside them.
    pub origin: String,
    /// The tree size that checkpoint committed to.
    pub tree_size: u64,
    /// The 32-byte RFC 6962 root hash that checkpoint committed to.
    pub root: [u8; 32],
}

/// Reads the checkpoint body out of a leaf that holds a signed checkpoint note,
/// or `None` if the leaf is not one.
///
/// The split is the C2SP signed-note split: everything up to and including the
/// newline before the note's **last** blank line is the body, and
/// [`CheckpointBody::parse`] must accept it. A leaf that is not valid UTF-8,
/// has no blank line, or whose body is not a checkpoint yields `None`.
///
/// ⚠️ **This does not verify anything.** No signature is checked, no key is
/// consulted, and a note whose signature block is nonsense still yields its
/// body — see the [module docs](self) for what that costs and where the
/// mitigation belongs. The function is public so the rule the projection folds
/// by can be stated, checked, and disagreed with, rather than being an
/// implementation detail that only agrees with itself; its second party is
/// `lys-core`'s `verify_note`, which returns the body it verified, and the
/// tests hold the two against each other.
pub fn checkpoint_in_leaf(leaf: &[u8]) -> Option<CheckpointBody> {
    let text = std::str::from_utf8(leaf).ok()?;
    // The LAST blank line, matching Go's `bytes.LastIndex` and `lys-core`'s
    // note parser: the body keeps its trailing newline and the signature block
    // is everything after the blank line.
    let split = text.rfind("\n\n")?;
    CheckpointBody::parse(text.get(..=split)?).ok()
}

/// What a witness remembers about the logs whose checkpoints it has recorded.
///
/// Built by folding the anchor's own leaves; see the [module docs](self) for
/// the fold rule, for why nothing here is authoritative, and for what an
/// unverified parse costs.
#[derive(Clone, Debug, Default)]
pub struct WitnessProjection {
    /// Ordered so iteration is stable across rebuilds — a report that lists
    /// origins in hash order would differ run to run for no reason.
    latest: BTreeMap<String, OriginState>,
}

impl WitnessProjection {
    /// Folds every leaf the anchor holds into a projection.
    pub fn rebuild<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy>(
        anchor: &Anchor<S, K, P>,
    ) -> Self {
        Self::rebuild_prefix(anchor, anchor.tree_size())
    }

    /// Folds the anchor's first `leaves` leaves — indices `0..leaves` — into a
    /// projection.
    ///
    /// This is what the observation path uses, with `leaves` set to the index
    /// the note was just recorded at, so the comparison is against everything
    /// the witness recorded **before** this submission and never against the
    /// submission itself. Passing a `leaves` above the log's size is not an
    /// error and folds what exists; the log is the bound.
    pub fn rebuild_prefix<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy>(
        anchor: &Anchor<S, K, P>,
        leaves: u64,
    ) -> Self {
        let mut latest = BTreeMap::new();
        for index in 0..leaves.min(anchor.tree_size()) {
            let Some(leaf) = anchor.leaf_bytes(index) else {
                continue;
            };
            let Some(body) = checkpoint_in_leaf(leaf) else {
                continue;
            };
            // Ascending index, so a later entry replaces an earlier one and the
            // survivor is the last recorded — not the largest.
            latest.insert(
                body.origin().to_string(),
                OriginState {
                    origin: body.origin().to_string(),
                    tree_size: body.tree_size(),
                    root: body.root_hash(),
                },
            );
        }
        Self { latest }
    }

    /// What this witness last recorded for `origin`, or `None` if it has
    /// recorded nothing for it.
    pub fn latest(&self, origin: &str) -> Option<&OriginState> {
        self.latest.get(origin)
    }

    /// Every origin this witness has recorded a checkpoint for, in lexical
    /// order.
    pub fn origins(&self) -> impl Iterator<Item = &str> {
        self.latest.keys().map(String::as_str)
    }

    /// How many origins this projection holds.
    pub fn len(&self) -> usize {
        self.latest.len()
    }

    /// Whether this projection holds nothing — a witness that has recorded no
    /// checkpoint at all, which is the state every anchor starts in.
    pub fn is_empty(&self) -> bool {
        self.latest.is_empty()
    }
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
