//! Pass calls exercise the declared action through the grants' authority.

use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityId;
use lys_identity::grants::{ExerciseRequest, GrantError, Route};

use crate::routes::AppState;

/// A pass is judged before its handler and never acquires a person's authority. So is
/// an agent whose signed MCP message relayed this call: the route's action is exercised
/// through the agent's own grants, as a run pass's is.
pub(crate) fn admit(
    state: &AppState,
    method: &str,
    path: &str,
    headers: &HeaderMap,
) -> Result<(), Box<Response>> {
    let pass = crate::agent_pass::holder(state, headers)
        .map_err(|error| Box::new(error.into_response()))?;
    let Some(agent) = pass.or_else(crate::agent_signature::relayed_agent) else {
        return Ok(());
    };
    // The MCP envelope carries no resource act; its dispatched tool route is judged.
    if path == "/mcp" && matches!(method, "GET" | "POST" | "DELETE") {
        return Ok(());
    }
    if state.kept_responsibilities.keeps(method, path) {
        return Err(Box::new(crate::who_can_grant::refusal(
            crate::who_can_grant::AuthorityRefusal::ResponsibilityKept { route: path },
            &[],
        )));
    }
    // An agent acts within its grants and otherwise asks, so asking never
    // needs a grant: an active pass held by an active agent is enough, and
    // the request records the asker.
    if method == "POST" && path == "/requests" {
        return crate::grants::with_grants(state, |judged| {
            let actor = crate::routes::signed_in(state, headers)?;
            crate::caller_admission::active_caller(judged.directory, &actor)?;
            Ok(())
        })
        .map_err(|error| Box::new(error.into_response()));
    }
    // An agent's ask for its own pass to an app (AGENTS-006 R1) exercises no
    // grant: the route takes the run pass as its proof of who asks, refuses
    // another agent's, and issues only the asker's own rights.
    if method == "POST" && crate::agent_app_pass::asks_for_pass(path) {
        return Ok(());
    }
    let (resource, action) = crate::openapi_table::token_scope(method, path)
        .map_err(|error| Box::new(error.into_response()))?;
    // A person's credential is never an agent's, whatever it holds
    // (AGENTS-019 R3): refused by that name before any grant is read.
    if lys_identity::grants::WITHHELD_FROM_AGENTS.contains(&action.as_str()) {
        return Err(Box::new(
            crate::error::ServerError::from(GrantError::WithheldFromAgents {
                relation: action.to_string(),
                withheld: action.to_string(),
            })
            .into_response(),
        ));
    }
    let refusal = crate::grants_reads::with_read_grants(
        state,
        None,
        crate::session::now(),
        |judged| {
            let mut asked = crate::grants_reads::resource_questions(judged, &resource, &action);
            asked.push(ExerciseRequest {
                caller: IdentityId::Agent(agent),
                route: Route::Api,
                resource: resource.clone(),
                action: action.clone(),
            });
            Ok(asked)
        },
        |mut judged| {
            let actor = crate::routes::signed_in(state, headers)?;
            crate::caller_admission::active_caller(judged.directory, &actor)?;
            judged.apps.admit_kind(None, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            let request = ExerciseRequest {
                caller: IdentityId::Agent(agent),
                route: Route::Api,
                resource: resource.clone(),
                action: action.clone(),
            };
            let at = crate::session::now();
            let decision = crate::grants::decide(
                &mut judged,
                &request,
                at,
                None,
                crate::grants::Decision::Exercise,
            );
            let (permit, _) = match decision {
                Ok(permit) => permit,
                Err(error) if names_grantors(&error) => {
                    let holders = crate::who_can_grant::for_judged(
                        &mut judged,
                        agent,
                        &request.resource,
                        &request.action,
                        at,
                    )?;
                    return Ok(Some(Box::new(crate::who_can_grant::refusal(
                        error, &holders,
                    ))));
                }
                Err(error) => return Err(error.into()),
            };
            match permit.use_event {
                Some(Ok(_)) => Ok(None),
                Some(Err(error)) => Err(lys_identity::grants::GrantError::LogUnavailable {
                    reason: format!("the granted call's use was not recorded: {error}"),
                }
                .into()),
                None => Err(lys_identity::grants::GrantError::LogUnavailable {
                    reason: "the granted call has no recorded use event".to_owned(),
                }
                .into()),
            }
        },
    )
    .map_err(|error| Box::new(error.into_response()))?;
    match refusal {
        Some(refusal) => Err(refusal),
        None => Ok(()),
    }
}

fn names_grantors(error: &GrantError) -> bool {
    matches!(
        error,
        GrantError::NotHeld { .. } | GrantError::Revoked { .. } | GrantError::Expired { .. }
    )
}

#[cfg(test)]
#[path = "route_refusal_tests.rs"]
mod tests;
