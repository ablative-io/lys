//! Explicit snapshot migration changes the projection while preserving signed history.

use lys_core::Ed25519Identity;
use lys_log_store::{FrontierLog, LeafStore, unseal};

use crate::checkpoints;
use crate::directory_state;
use crate::error::IdentityError;
use crate::signer::verify_event;

const DOMAIN: &str = "lys/identity-directory/v1";

fn unavailable(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::LogUnavailable {
        reason: format!("directory snapshot migration: {error}"),
    }
}

/// Upgrade a signed version-two snapshot before install or upgrade opens the directory.
/// No leaf or historical receipt is rewritten; a failed snapshot write is returned.
pub fn migrate<S: LeafStore>(store: S, key: &Ed25519Identity) -> Result<bool, IdentityError> {
    let Some(sealed) = store.snapshot().map_err(unavailable)? else {
        return Ok(false);
    };
    let snapshot =
        unseal(&sealed, DOMAIN, store.origin(), &key.public_key_bytes()).map_err(unavailable)?;
    let (mut frontier, state) = snapshot.into_parts();
    let (mut checkpoints, owner) =
        checkpoints::unwrap(&state, frontier.size()).map_err(unavailable)?;
    let version = directory_state::version(&owner).map_err(unavailable)?;
    if version == directory_state::STATE_VERSION {
        return Ok(false);
    }
    if version != 2 {
        return Err(IdentityError::DirectorySnapshotUnmigrated {
            found: version,
            expected: directory_state::STATE_VERSION,
        });
    }
    if frontier.size() > store.pinned().tree_size || store.extent() < store.pinned().tree_size {
        return Err(unavailable(
            "the snapshot, pin and available leaves disagree",
        ));
    }
    if checkpoints
        .before(frontier.size())
        .is_some_and(|held| held.size() == frontier.size() && held.root() != frontier.root())
    {
        return Err(unavailable(
            "the snapshot checkpoint differs from its signed frontier",
        ));
    }
    for index in store.pinned().tree_size..store.extent() {
        let leaf = store
            .leaf(index)
            .map_err(unavailable)?
            .ok_or_else(|| unavailable(format!("unpinned leaf {index} is missing")))?;
        verify_event(&leaf, &key.public_key_bytes()).map_err(|error| {
            IdentityError::LeafNotAnEvent {
                index,
                reason: error.to_string(),
            }
        })?;
    }
    let mut projection =
        directory_state::migrate_v2(&owner, frontier.size()).map_err(unavailable)?;
    let (mut log, tail) = FrontierLog::resume(store, frontier.clone()).map_err(unavailable)?;
    for leaf in tail.leaves {
        let signed = verify_event(&leaf, &key.public_key_bytes())?;
        projection.apply(signed.event(), frontier.size())?;
        frontier.push(&leaf);
        checkpoints.record(&frontier);
    }
    let owner = directory_state::encode(&projection, frontier.size()).map_err(unavailable)?;
    let state = checkpoints::wrap(&checkpoints, &owner).map_err(unavailable)?;
    log.write_snapshot(DOMAIN, &state, key)
        .map_err(unavailable)?;
    Ok(true)
}
