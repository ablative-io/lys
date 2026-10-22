//! An agent's own call on a person's account, relayed from the MCP message
//! it signed, judged by its own grants (AGENTS-019 R3, R5).
//!
//! These routes admit an agent's signature, so the door leaves a relayed
//! call to the handler (`openapi_table::handler_judges`), and
//! `routes::administrator` admits a relayed agent on the understanding that
//! its grant was judged. Here it is: the route's declared act on
//! `account:<person>` is exercised through the agent's own grants and its
//! use recorded, naming the act, before anything is read or changed. A run
//! pass's call was judged at the door already (`route_actions::admit`), and
//! a person's session holds the administrator's standing; neither is judged
//! again.

use lys_identity::grants::{Action, ExerciseRequest, GrantError, Resource, Route};
use lys_identity::{Actor, AuthMethod, IdentityId};

use crate::error::ServerError;
use crate::grants::{Decision, decide};
use crate::routes::AppState;
use crate::session::now;

/// Exercise `action` on person `person`'s account for the agent whose MCP
/// message relayed this call; nothing for any other caller.
///
/// # Errors
/// Refuses an inactive agent, an act its grants do not reach (`NotHeld`,
/// `Revoked`, `Expired`), or a use that could not be recorded.
pub(super) fn exercised(
    state: &AppState,
    actor: &Actor,
    person: &str,
    action: &str,
) -> Result<(), ServerError> {
    let AuthMethod::AgentSignature(agent) = actor.provenance().method() else {
        return Ok(());
    };
    if crate::agent_signature::relayed_agent() != Some(agent) {
        return Ok(());
    }
    let request = ExerciseRequest {
        caller: IdentityId::Agent(agent),
        route: Route::Api,
        resource: Resource::new("account", person)?,
        action: Action::new(action)?,
    };
    crate::grants_reads::with_read_grants(
        state,
        None,
        now(),
        |_| Ok(vec![request.clone()]),
        |mut judged| {
            crate::caller_admission::active_caller(judged.directory, actor)?;
            judged.apps.admit_kind(None, "account")?;
            judged.apps.admit_action("account", action)?;
            let (permit, _) = decide(&mut judged, &request, now(), None, Decision::Exercise)?;
            match permit.use_event {
                Some(Ok(_)) => Ok(()),
                Some(Err(error)) => Err(GrantError::LogUnavailable {
                    reason: format!("the account act's use was not recorded: {error}"),
                }
                .into()),
                None => Err(GrantError::LogUnavailable {
                    reason: "the account act has no recorded use event".to_owned(),
                }
                .into()),
            }
        },
    )
}
