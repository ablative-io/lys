//! Pass calls exercise the declared action through the grants' authority.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityId;
use lys_identity::grants::{ExerciseRequest, Route};

use crate::routes::AppState;

/// A pass is judged before its handler and never acquires a person's authority.
pub(crate) fn admit(
    state: &AppState,
    method: &str,
    path: &str,
    headers: &HeaderMap,
) -> Result<(), Box<Response>> {
    let Some(agent) = crate::agent_pass::holder(state, headers)
        .map_err(|error| Box::new(error.into_response()))?
    else {
        return Ok(());
    };
    // The MCP envelope carries no resource act; its dispatched tool route is judged.
    if path == "/mcp" && matches!(method, "GET" | "POST" | "DELETE") {
        return Ok(());
    }
    if state.kept_responsibilities.keeps(method, path) {
        return Err(Box::new(
            (
                StatusCode::FORBIDDEN,
                axum::Json(serde_json::json!({
                    "refusal":"ResponsibilityKept", "reason":"a person keeps this responsibility",
                    "fields":{"route":path}
                })),
            )
                .into_response(),
        ));
    }
    let (resource, action) = crate::openapi_table::token_scope(method, path)
        .map_err(|error| Box::new(error.into_response()))?;
    crate::grants::with_grants(state, |mut judged| {
        let actor = crate::routes::signed_in(state, headers)?;
        crate::caller_admission::active_caller(judged.directory, &actor)?;
        judged.apps.admit_kind(None, resource.kind())?;
        judged.apps.admit_action(resource.kind(), action.as_str())?;
        let request = ExerciseRequest {
            caller: IdentityId::Agent(agent),
            route: Route::Api,
            resource,
            action,
        };
        let (permit, _) = crate::grants::decide(
            &mut judged,
            &request,
            crate::session::now(),
            None,
            crate::grants::Decision::Exercise,
        )?;
        match permit.use_event {
            Some(Ok(_)) => Ok(()),
            Some(Err(error)) => Err(lys_identity::grants::GrantError::LogUnavailable {
                reason: format!("the granted call's use was not recorded: {error}"),
            }
            .into()),
            None => Err(lys_identity::grants::GrantError::LogUnavailable {
                reason: "the granted call has no recorded use event".to_owned(),
            }
            .into()),
        }
    })
    .map_err(|error| Box::new(error.into_response()))
}
