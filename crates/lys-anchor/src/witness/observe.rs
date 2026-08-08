//! [`observe`] — record a checkpoint note, then say what this witness's own
//! memory makes of it.
//!
//! # The order, and why it is the whole design
//!
//! 1. **Record.** The note goes through [`Anchor::submit`] — the core path, the
//!    same one any other statement takes, with no extra argument and no witness
//!    variant. It is appended durably and receipted before anything in this
//!    file looks at it.
//! 2. **Check.** Against the projection over the leaves recorded *before* this
//!    one, and against nothing else. The witness compares two things it
//!    personally holds; it fetches nothing, trusts nothing the submitter said
//!    about its own history, and reaches past its own two observations by no
//!    route at all.
//! 3. **Report.** The [`Observation`] goes to the caller. Nothing from step 2
//!    enters a signed artifact, and nothing from step 2 can undo step 1.
//!
//! There is no branch between 1 and 2. A relation is computed from a leaf that
//! is already durable, so **every value this function can return has already
//! been recorded** — including `Conflicting`, which is the point: refusing to
//! record an equivocating checkpoint destroys the evidence the detection runs
//! on, and equivocation is only ever caught by two durable memories
//! disagreeing.
//!
//! # What this function refuses
//!
//! Nothing of its own. Its errors are [`Anchor::submit`]'s errors —
//! [`AnchorError::NotAdmitted`](crate::AnchorError::NotAdmitted) if the
//! anchor's admission policy declined, and storage or receipt failures. The
//! witness path adds **no admission rule**, which is what keeps it additive:
//! signature verification of the note, if it is done at all, is the policy's
//! business and applies to every submission equally.
//!
//! A note that cannot be read as a checkpoint at all is not an error either. It
//! is recorded like any other statement and reported as a submission the
//! witness compared nothing against — an anchor that erred on unparseable bytes
//! would be an anchor with an opinion about what a submitter may log.

use lys_core::merkle::{ConsistencyProof, RootHash, verify_consistency};
use lys_log_store::LeafStore;

use crate::admission::{AdmissionPolicy, SubmitterContext};
use crate::anchor::Anchor;
use crate::error::AnchorResult;
use crate::keys::InProcessSigner;
use crate::wire::Submission;

use super::projection::{OriginState, WitnessProjection, checkpoint_in_leaf};
use super::report::{Observation, Relation};

/// Records `note` in `anchor`'s log and reports how it sits against what this
/// witness last recorded for the same origin.
///
/// `note` is the submitter's signed checkpoint, and it is stored **verbatim**:
/// the leaf is exactly these bytes, so the receipt's proven leaf is the note a
/// third party can re-verify for themselves. `consistency_proof`, when
/// supplied, is the RFC 6962 `PROOF(m, D[n])` byte encoding from the size this
/// witness last recorded to the size the note states; it participates in the
/// report and in nothing else. Omitting it is permitted and yields
/// [`Relation::Unrelated`] for any size increase, because a witness with no
/// proof holds no proof.
///
/// `context` is what the caller established about the submitter and is passed
/// straight to the admission policy, exactly as [`Anchor::submit`] would — this
/// path establishes nothing extra and asserts nothing extra.
///
/// The returned [`Observation`]'s receipt is **byte-identical** to the one
/// `anchor.submit(Submission { statement: note }, context)` would have returned
/// at the same tree state. There is no field in it in which witnessing could be
/// encoded, which is how the no-endorsement property is held by shape rather
/// than by wording.
///
/// # Errors
///
/// Exactly [`Anchor::submit`]'s: the admission refusal, which carries nothing,
/// and storage or receipt failures. When any of them occurs **nothing was
/// recorded and no comparison was made**. This function contributes no refusal
/// of its own.
pub fn observe<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy>(
    anchor: &mut Anchor<S, K, P>,
    note: &[u8],
    consistency_proof: Option<&[u8]>,
    context: SubmitterContext<'_>,
) -> AnchorResult<Observation> {
    // 1. RECORD — unconditional, through the core path, before anything below
    //    this line has looked at the bytes.
    let recorded = anchor.submit(Submission { statement: note }, context)?;

    // 2. CHECK — on durable state, against the leaves recorded before this one.
    let comparison = checkpoint_in_leaf(note).and_then(|body| {
        WitnessProjection::rebuild_prefix(anchor, recorded.leaf_index)
            .latest(body.origin())
            .cloned()
            .map(|previous| {
                let relation = relate(&previous, &body, consistency_proof);
                (previous, relation)
            })
    });

    // 3. REPORT.
    let (previous, relation) = match comparison {
        Some((previous, relation)) => (Some(previous), Some(relation)),
        None => (None, None),
    };
    Ok(Observation {
        recorded,
        previous,
        relation,
    })
}

/// Compares the checkpoint just recorded against the state this witness held
/// for the same origin.
///
/// Both arguments are things the witness holds: `previous` came out of its own
/// leaves and `observed` came out of the leaf it just wrote. Nothing else is
/// consulted, and there is no return value that refuses anything.
fn relate(
    previous: &OriginState,
    observed: &lys_core::checkpoint::CheckpointBody,
    consistency_proof: Option<&[u8]>,
) -> Relation {
    let size = observed.tree_size();
    if size == previous.tree_size {
        // An append-only tree has exactly one root per size, so at an unchanged
        // size the root either repeats or the history is two different things.
        return if observed.root_hash() == previous.root {
            Relation::Identical
        } else {
            Relation::Conflicting
        };
    }
    if size < previous.tree_size {
        return Relation::Rollback;
    }
    // A larger size, so the only remaining question is whether the new root is
    // provably built on the old one. Every way of not knowing that — no proof,
    // malformed proof, failing proof — is the same amount of knowledge.
    let Some(bytes) = consistency_proof else {
        return Relation::Unrelated;
    };
    let Ok(proof) = ConsistencyProof::try_from_bytes(bytes.to_vec()) else {
        return Relation::Unrelated;
    };
    let old_root = RootHash::from_parts(previous.root, previous.tree_size);
    if verify_consistency(&old_root, &observed.to_root(), &proof).is_ok() {
        Relation::Extends
    } else {
        Relation::Unrelated
    }
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
