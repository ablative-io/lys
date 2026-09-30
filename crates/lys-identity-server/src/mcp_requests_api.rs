//! A caller may ask only for an agent whose provisioning it already sees.
//! Recording a request grants nothing and does not change the reviewed profile.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};

use crate::agent_sight::seen_agent;
use crate::error::ServerError;
use crate::grants::caller;
use crate::mcp_requests_store::{McpRequest, McpRequestState, McpRequestStore, server_name};
use crate::provisioning_api::with_provisioning;
use crate::routes::{AppState, with_directory};
use crate::session::now;
use crate::tree_views::latest_reviewed;

/// An operation asking for an already declared server by name.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct McpAskBody {
    operation: String,
    server: String,
}

/// Pending requests for the admitted agent, in the order recorded.
#[derive(Serialize, utoipa::ToSchema)]
pub struct McpRequestList {
    /// The agent's recorded requests.
    pub requests: Vec<McpRequest>,
}

/// Request recording and read-back, with the same visibility as provisioning.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/mcp-requests", get(list).post(ask))
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::McpRequestsUnavailable {
        reason: reason.into(),
    }
}

fn with_requests<T>(
    state: &AppState,
    act: impl FnOnce(&mut McpRequestStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .mcp_requests
        .as_ref()
        .ok_or_else(|| unavailable("the configuration names no requests_dir"))?;
    let mut store = store
        .lock()
        .map_err(|error| unavailable(format!("request lock: {error}")))?;
    let before = store.start().to_string();
    let answer = store.settle().and_then(|()| act(&mut store));
    if store.start().to_string() != before {
        (state.say)(&format!("mcp-requests log recovered: {}", store.start()));
    }
    answer
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<McpRequestList>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent.to_string();
    with_requests(&state, |store| {
        Ok(Json(McpRequestList {
            requests: store.listed(&agent)?,
        }))
    })
}

async fn ask(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<McpAskBody>, JsonRejection>,
) -> Result<Json<McpRequest>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent;
    let by = with_directory(&state, |directory| {
        Ok(caller(&state, &headers, directory.projection()?)?.to_string())
    })?;
    let Json(body) = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| ServerError::RequestMalformed {
            reason: format!("operation: {error}"),
        })?
        .to_string();
    let server = server_name(&body.server)?;
    let agent_id = agent.to_string();
    with_requests(&state, |requests| {
        if let Some(kept) = requests.replay(&operation, &agent_id, &server, &by)? {
            return Ok(Json(kept));
        }
        with_provisioning(&state, |profiles| {
            let known = profiles.profiles().iter().any(|profile| {
                profile.versions.iter().any(|version| {
                    version.reviewed.is_some()
                        && version
                            .settings
                            .mcp_servers
                            .iter()
                            .any(|held| held.name == server)
                })
            });
            if !known {
                return Err(ServerError::McpServerUnknown { server });
            }
            let profile = profiles.profile(&agent_id);
            let version = profile.and_then(latest_reviewed).ok_or_else(|| {
                ServerError::ProfileNotReviewed {
                    version: profile.map_or(0, crate::provisioning_store::Profile::latest),
                }
            })?;
            if version
                .settings
                .mcp_servers
                .iter()
                .any(|held| held.name == server)
            {
                return Err(ServerError::McpServerHeld { server });
            }
            requests
                .ask(McpRequest {
                    id: operation,
                    agent: agent_id,
                    server,
                    profile_version: version.number,
                    state: McpRequestState::Pending,
                    asked_by: by,
                    asked_at: now(),
                })
                .map(Json)
        })
    })
}
