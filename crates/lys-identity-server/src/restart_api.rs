//! An operator restart ends the selected session on its exit signal before
//! starting a new session through the existing reviewed launch path.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId, OperationId};
use lys_runner::Act;
use serde::Deserialize;
use serde_json::Value;

use crate::error::ServerError;
use crate::launch_api::{Chosen, admitted, start_caller, start_for, start_profile};
use crate::provisioning_api::with_provisioning;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runner_api::{Carried, perform};
use crate::runner_sessions::driven;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RestartBody {
    session: String,
    operation: String,
}

/// The operator restart route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/restart", post(restart))
}

/// The restart route's schemas, derived from the types it takes and answers.
pub(crate) fn openapi(api: &mut lys_openapi::Api) -> crate::openapi_types::Entry {
    (
        crate::openapi_table::POST,
        "/agents/{id}/restart",
        Some(api.schema::<RestartBody>()),
        Some(api.schema::<crate::launch_api::StartCommandView>()),
    )
}

async fn restart(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<RestartBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id).map_err(|error| ServerError::RequestMalformed {
        reason: format!("the restart agent does not read: {error}"),
    })?;
    let caller = start_caller(&state, &headers, &actor, &agent.to_string())?;
    let Json(given) = given.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let parsed = |id: &str| {
        OperationId::from_str(id)
            .map(|id| id.to_string())
            .map_err(|error| ServerError::RequestMalformed {
                reason: format!("the restart id does not read: {error}"),
            })
    };
    let session = parsed(&given.session)?;
    let operation = parsed(&given.operation)?;
    if session == operation {
        return Err(ServerError::RuntimeReportReused { operation });
    }
    let driven = driven(&state, &session)?;
    if driven.agent != agent.to_string() {
        return Err(ServerError::RuntimeSessionUnknown);
    }
    if admitted(&state, &operation, &driven.agent)?.is_some() {
        return start_for(&state, &headers, &actor, agent, &driven.machine, &operation).await;
    }
    with_directory(&state, |directory| {
        let record = directory
            .projection()?
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        crate::start_checks::active(record.state())
    })?;
    let profile = with_provisioning(&state, |store| {
        store
            .profile(&driven.agent)
            .and_then(|profile| {
                profile
                    .versions
                    .iter()
                    .rev()
                    .find(|version| version.reviewed.is_some())
                    .cloned()
            })
            .ok_or_else(|| ServerError::Runner {
                refusal: "profile_version_unreviewed".to_owned(),
                words: format!(
                    "agent {} has no reviewed profile version; session {session} keeps running",
                    driven.agent
                ),
            })
    })?;
    let Json(stopped) = perform(
        &state,
        (&driven, caller, "end"),
        Carried::default(),
        Act::End { session },
    )
    .await?;
    if stopped["answer"]["kind"] != "ended" {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "the restart's end answered without the session's exit".to_owned(),
        });
    }
    crate::agent_pass::end_session(&state, &given.session)?;
    start_profile(
        &state,
        &headers,
        &actor,
        agent,
        &driven.machine,
        &operation,
        Chosen {
            profile: Some(profile),
            directory: None,
        },
    )
    .await
}
