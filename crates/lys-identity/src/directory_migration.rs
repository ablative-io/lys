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
///
/// # Errors
/// - `IdentityError::LogUnavailable` if opening, reading or folding the log fails.
/// - `IdentityError::LeafNotAnEvent` if a stored leaf is missing or cannot be verified.
/// - `IdentityError::DirectorySnapshotUnmigrated` if an old snapshot requires explicit migration.
/// - `IdentityError::OperationReused` if a replayed event repeats an operation.
/// - `IdentityError::AlreadyRegistered` if replay registers an identity twice.
/// - `IdentityError::AlreadyBootstrapped` if replay repeats setup for a bound login.
/// - `IdentityError::IdentityUnknown` if a replayed change names an absent identity.
/// - `IdentityError::BindingTaken` if replay binds one login to different people.
/// - `IdentityError::BindingMalformed` if an issuer move produces an invalid login binding.
/// - `IdentityError::StateMismatch` if a replayed transition starts from another state.
/// - `IdentityError::LinkSourceSeen` if replay repeats a link source operation.
/// - `IdentityError::ChangeMismatch` if replayed identity or reporting fields disagree.
/// - `IdentityError::AnswersToUnknown` if a reporting target is absent or of another kind.
/// - `IdentityError::AnswersToInactive` if the immediate reporting target is inactive.
/// - `IdentityError::AnswersToCycle` if reporting returns to an earlier identity.
/// - `IdentityError::NoAccountablePerson` if the reporting chain cannot reach an active person.
/// - `IdentityError::DraftChangeInvalid` if a replayed draft payload or decision is invalid.
/// - `IdentityError::DraftHashMismatch` if a decision or correction names another creation.
/// - `IdentityError::DraftNotFound` if a replayed decision names an absent draft.
/// - `IdentityError::DraftNotPending` if a replayed decision is outside the draft's current state.
/// - `IdentityError::EventMalformed` if a tail event has an invalid envelope or payload shape.
/// - `IdentityError::EventNotCanonical` if a tail event is not its canonical encoding.
/// - `IdentityError::VersionUnsupported` if a tail event version is unsupported.
/// - `IdentityError::IdentifierMalformed` if a tail draft contains an invalid textual id.
/// - `IdentityError::ProfileInvalid` if a tail event contains an invalid display profile.
/// - `IdentityError::TransitionRefused` if a tail event contains an undefined transition.
/// - `IdentityError::ReasonRequired` if a tail suspension or retirement lacks a reason.
/// - `IdentityError::SignerMismatch` if a tail event names another service key.
/// - `IdentityError::SignatureInvalid` if a tail signature does not verify.
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
    // Every version from two to the one before this build's is read by
    // migrate_v2 or decode; a hand-kept list of versions is what refused the
    // installed version-five snapshot on 20 October 2026.
    if !(2..directory_state::STATE_VERSION).contains(&version) {
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
    let mut projection = if version == 2 {
        directory_state::migrate_v2(&owner, frontier.size()).map_err(unavailable)?
    } else {
        directory_state::decode(&owner, frontier.size()).map_err(unavailable)?
    };
    let (mut log, tail) = FrontierLog::resume(store, frontier.clone()).map_err(unavailable)?;
    for leaf in tail.leaves {
        let signed = verify_event(&leaf, &key.public_key_bytes())?;
        projection.apply_entry(signed.entry(), frontier.size())?;
        frontier.push(&leaf);
        checkpoints.record(&frontier);
    }
    let owner = directory_state::encode(&projection, frontier.size()).map_err(unavailable)?;
    let state = checkpoints::wrap(&checkpoints, &owner).map_err(unavailable)?;
    log.write_snapshot(DOMAIN, &state, key)
        .map_err(unavailable)?;
    Ok(true)
}
