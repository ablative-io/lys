//! An agent's act judged by its own grants in the handler, its use recorded
//! (AGENTS-019 R4). A route whose handler judges the agent
//! (`openapi_table::judged_by_handler`) is not judged at the door, so a
//! handler that admits an agent by a grant exercises it here, as the door
//! would, never only asks it.

use lys_identity::grants::{ExerciseRequest, GrantError, Permit};
use lys_log_store::LeafStore;

use crate::error::ServerError;
use crate::grants::{Decision, Judged, decide};
use crate::session::now;

/// Exercise `request` under `judged`, its use recorded: `false` when no live
/// grant of the caller's carries it.
///
/// # Errors
/// Refuses a grant read that fails for any reason but the act's absence, or
/// a use that could not be recorded.
pub(crate) fn exercised<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    request: &ExerciseRequest,
) -> Result<bool, ServerError> {
    match decide(judged, request, now(), None, Decision::Exercise) {
        Ok((permit, _)) => recorded(permit).map(|()| true),
        Err(error) => Ok(crate::grants::allowed::<()>(Err(error))?),
    }
}

fn recorded(permit: Permit) -> Result<(), ServerError> {
    match permit.use_event {
        Some(Ok(_)) => Ok(()),
        Some(Err(error)) => Err(GrantError::LogUnavailable {
            reason: format!("the agent's act's use was not recorded: {error}"),
        }
        .into()),
        None => Err(GrantError::LogUnavailable {
            reason: "the agent's act has no recorded use event".to_owned(),
        }
        .into()),
    }
}
