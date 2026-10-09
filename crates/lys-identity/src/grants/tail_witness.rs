//! Authenticate a complete tail inside its provider's current-head reading.

use lys_log_store::witness::TailWitness;
use lys_log_store::{Frontier, PinnedRoot};

use super::{GrantError, SignedGrantEvent, verify_grant_event};
use crate::IdentityError;

pub(super) fn unavailable(reason: impl Into<String>) -> GrantError {
    GrantError::Identity(IdentityError::LogUnavailable {
        reason: reason.into(),
    })
}

pub(super) fn authenticate(
    witness: &TailWitness,
    settled: &Frontier,
    origin: &str,
    key: &[u8; 32],
) -> Result<Vec<SignedGrantEvent>, GrantError> {
    if witness.origin != origin {
        return Err(unavailable("tail witness names a different grant log"));
    }
    let lower = PinnedRoot {
        tree_size: settled.size(),
        root: settled.root(),
    };
    if witness.lower != lower {
        return Err(unavailable(
            "tail witness differs from the trusted grant frontier",
        ));
    }
    let count = u64::try_from(witness.leaves.len())
        .map_err(|error| unavailable(format!("tail leaf count cannot be represented: {error}")))?;
    if witness.upper.tree_size.checked_sub(lower.tree_size) != Some(count) {
        return Err(unavailable(
            "tail witness does not contain the complete grant range",
        ));
    }
    let mut frontier = settled.clone();
    let mut events = Vec::with_capacity(witness.leaves.len());
    for leaf in &witness.leaves {
        if leaf.index != frontier.size() {
            return Err(unavailable(
                "grant tail indexes are omitted, repeated or reordered",
            ));
        }
        let event = verify_grant_event(&leaf.bytes, key)?;
        frontier.push(&leaf.bytes);
        events.push(event);
    }
    if frontier.size() != witness.upper.tree_size || frontier.root() != witness.upper.root {
        return Err(unavailable(
            "authenticated grant tail does not reach its certified root",
        ));
    }
    Ok(events)
}
