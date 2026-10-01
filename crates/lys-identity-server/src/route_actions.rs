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
) -> Result<(), Response> {
    let Some(agent) =
        crate::agent_pass::holder(state, headers).map_err(IntoResponse::into_response)?
    else {
        return Ok(());
    };
    // The MCP envelope carries no resource act; its dispatched tool route is judged.
    if path == "/mcp" && matches!(method, "GET" | "POST" | "DELETE") {
        return Ok(());
    }
    if crate::kept_responsibilities::KEPT.contains(&(method, path)) {
        return Err((
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "refusal":"ResponsibilityKept", "reason":"a person keeps this responsibility",
                "fields":{"route":path}
            })),
        )
            .into_response());
    }
    let (resource, action) =
        crate::openapi_table::token_scope(method, path).map_err(IntoResponse::into_response)?;
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
        crate::grants::decide(
            &mut judged,
            &request,
            crate::session::now(),
            None,
            crate::grants::Decision::Exercise,
        )?;
        Ok(())
    })
    .map_err(IntoResponse::into_response)
}
