//! The variables routes (AGENTS-001 R2): an agent reads and patches its own
//! agent map and its own session's map by its signed request; its
//! responsible person and the administrator read and patch them signed in;
//! another agent is refused naming the grant it would need, the
//! `variables.set` or `variables.read` action on the agent. Every patch
//! carries the revision read and is refused `variables_stale` when the map
//! has moved.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Uri};
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, IdentityId};
use serde::Deserialize;

use crate::agent_signature::signed_agent;
use crate::error::ServerError;
use crate::grants::{caller, with_grants};
use crate::read_api::own_person;
use crate::routes::{AppState, is_administrator, signed_in, with_directory};
use crate::runtime_api::with_runtime;
use crate::session::now;
use crate::variables_state::{Read, Scope, VariablesError};
use crate::variables_store::{Patch, VariablesKept};

/// A patch to a scope's variables.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = VariablesPatchBody)]
#[serde(deny_unknown_fields)]
pub struct PatchBody {
    /// The revision read; 0 for a scope never patched.
    pub revision: u64,
    /// The values, by name; null removes a key.
    #[schema(schema_with = crate::variables_state::any_json_values)]
    pub values: BTreeMap<String, serde_json::Value>,
    /// When the keys set expire, in seconds since the Unix epoch.
    #[serde(default)]
    pub expires_at: Option<u64>,
}

/// The variables routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/variables", get(read_agent).post(patch_agent))
        .route(
            "/runtime/sessions/{id}/variables",
            get(read_session).post(patch_session),
        )
        .route("/me/variables", get(read_own_agent).post(patch_own_agent))
        .route(
            "/me/session/variables",
            get(read_own_session).post(patch_own_session),
        )
}

/// The agent and session of the run asking by its pass; a request with no
/// pass is refused by name, since these routes are a Lys-started run's own.
fn own_run(state: &AppState, headers: &HeaderMap) -> Result<(String, String), ServerError> {
    match crate::agent_pass::run_session(state, headers)? {
        Some((agent, session)) => Ok((agent.to_string(), session)),
        None => Err(ServerError::NotAdmitted {
            reason: "the me routes are a Lys-started run's own: they carry its run pass",
        }),
    }
}

async fn read_own_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Read>, ServerError> {
    let (agent, _session) = own_run(&state, &headers)?;
    let read = crate::variables_store::read(variables(&state)?, &Scope::Agent { id: agent })?;
    Ok(Json(read))
}

async fn read_own_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Read>, ServerError> {
    let (_agent, session) = own_run(&state, &headers)?;
    let read = crate::variables_store::read(variables(&state)?, &Scope::Session { id: session })?;
    Ok(Json(read))
}

async fn patch_own_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<PatchBody>, JsonRejection>,
) -> Result<Json<Read>, ServerError> {
    let (agent, _session) = own_run(&state, &headers)?;
    let body = given
        .map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })?;
    let author = IdentityId::Agent(
        AgentId::from_str(&agent).map_err(|_unread| ServerError::AgentNotVisible)?,
    );
    patched(&state, Scope::Agent { id: agent }, author, body)
}

async fn patch_own_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<PatchBody>, JsonRejection>,
) -> Result<Json<Read>, ServerError> {
    let (agent, session) = own_run(&state, &headers)?;
    let body = given
        .map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })?;
    let author = IdentityId::Agent(
        AgentId::from_str(&agent).map_err(|_unread| ServerError::AgentNotVisible)?,
    );
    patched(&state, Scope::Session { id: session }, author, body)
}

pub(crate) fn variables(state: &AppState) -> Result<&VariablesKept, ServerError> {
    state.variables.as_ref().ok_or_else(|| {
        VariablesError::Unavailable {
            reason: "the configuration names no variables_dir".to_owned(),
        }
        .into()
    })
}

/// The agent a session is held under, from the runtime reports.
pub(crate) fn session_agent(state: &AppState, session: &str) -> Result<String, ServerError> {
    if state.runtime.is_none() {
        return Err(ServerError::SessionUnknown);
    }
    with_runtime(state, |store| {
        store
            .session(session)
            .and_then(|tracked| tracked.agent.clone())
            .ok_or(ServerError::SessionUnknown)
    })
}

/// What asks: an agent by its signed request, or the signed-in caller.
fn asker(
    state: &AppState,
    headers: &HeaderMap,
    (method, uri, bytes): (&str, &Uri, &[u8]),
) -> Result<IdentityId, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let target = uri
            .path_and_query()
            .map_or(uri.path(), |target| target.as_str());
        match signed_agent(state, projection, headers, (method, target, bytes))? {
            Some(agent) => Ok(IdentityId::Agent(agent)),
            None => caller(state, headers, projection),
        }
    })
}

/// Admit `asker` to `action` (`variables.read` or `variables.set`) on
/// `agent`'s variables: the agent itself; a holder of a grant of the action
/// on the agent; the agent's responsible person; or the administrator.
pub(crate) fn admit(
    state: &AppState,
    headers: &HeaderMap,
    asker: IdentityId,
    agent: &str,
    action: &str,
) -> Result<(), ServerError> {
    let agent_id = AgentId::from_str(agent).map_err(|_unread| ServerError::AgentNotVisible)?;
    if let IdentityId::Agent(asking) = asker {
        if asking == agent_id {
            return Ok(());
        }
        let granted = with_grants(state, |held| {
            let request = ExerciseRequest {
                caller: asker,
                route: Route::Api,
                resource: Resource::new("agent", agent)?,
                action: Action::new(action)?,
            };
            Ok(crate::grants::allowed(held.grants.explain(
                held.directory,
                &request,
                now(),
                None,
            ))?)
        })?;
        if granted {
            return Ok(());
        }
        return Err(ServerError::NotPermitted {
            reason: format!(
                "agent {asking} holds no grant of {action} on agent {agent}, so it may not touch its variables"
            ),
        });
    }
    let actor = signed_in(state, headers)?;
    if is_administrator(state, &actor)? {
        return Ok(());
    }
    let person = with_directory(state, |directory| {
        own_person(directory.projection()?, &actor)
    })?;
    let responsible = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let record = projection
            .record(IdentityId::Agent(agent_id))
            .ok_or(ServerError::AgentNotVisible)?;
        Ok(record.responsible())
    })?;
    if responsible == Some(person) {
        return Ok(());
    }
    Err(ServerError::NotPermitted {
        reason: format!(
            "{person} is not responsible for agent {agent} and holds no grant of {action} on it"
        ),
    })
}

async fn read_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
) -> Result<Json<Read>, ServerError> {
    let asker = asker(&state, &headers, ("GET", &uri, &[]))?;
    admit(&state, &headers, asker, &id, "variables.read")?;
    let read = crate::variables_store::read(variables(&state)?, &Scope::Agent { id })?;
    Ok(Json(read))
}

async fn read_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
) -> Result<Json<Read>, ServerError> {
    let asker = asker(&state, &headers, ("GET", &uri, &[]))?;
    let agent = session_agent(&state, &id)?;
    admit(&state, &headers, asker, &agent, "variables.read")?;
    let read = crate::variables_store::read(variables(&state)?, &Scope::Session { id })?;
    Ok(Json(read))
}

async fn patch_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<Read>, ServerError> {
    let body: PatchBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let asker = asker(&state, &headers, ("POST", &uri, &bytes))?;
    admit(&state, &headers, asker, &id, "variables.set")?;
    patched(&state, Scope::Agent { id }, asker, body)
}

async fn patch_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<Read>, ServerError> {
    let body: PatchBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let asker = asker(&state, &headers, ("POST", &uri, &bytes))?;
    let agent = session_agent(&state, &id)?;
    admit(&state, &headers, asker, &agent, "variables.set")?;
    patched(&state, Scope::Session { id }, asker, body)
}

fn patched(
    state: &AppState,
    scope: Scope,
    author: IdentityId,
    body: PatchBody,
) -> Result<Json<Read>, ServerError> {
    let read = crate::variables_store::patch(
        variables(state)?,
        Patch {
            scope,
            revision: body.revision,
            author: author.to_string(),
            values: body.values,
            expires_at: body.expires_at,
        },
    )?;
    Ok(Json(read))
}
