//! Each agent's tool-boundary policy, read and changed through the API.
//!
//! - `GET /agents/{id}/policy`: the latest version, its digest, and when it
//!   applies
//! - `POST /agents/{id}/policy` `{"version", "rules"}`: the next version,
//!   sent on the version read (0 for none)
//!
//! A policy is read and changed by an administrator or the person
//! responsible for the agent, as the directory records it; a policy never
//! names who may change it. Anyone else is refused `not_permitted` and
//! shown nothing. A change that crossed another is refused
//! `PolicyVersionConflict`; one of the wrong shape is refused by name. A
//! kept version applies from the agent's next launch, never to a session
//! already running.

use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_runner::judge::{Policy, Rule};
use serde::{Deserialize, Serialize};

use crate::agent_policy_store::{PolicyStore, digest};
use crate::budgets_api::authorised;
use crate::budgets_state::{Holder, HolderKind};
use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// When a kept version takes effect, as the editor says it.
pub const APPLIES: &str = "applies on the agent's next launch; a session already running keeps the version it was launched with";

/// A change to an agent's policy.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PolicyBody {
    /// The version read before this change; 0 for a policy not yet set.
    version: u64,
    /// The deny rules, in the order the judge reads them.
    rules: Vec<Rule>,
}

/// An agent's policy as it stands.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PolicyView {
    /// The agent.
    pub agent: String,
    /// Its latest version; none while no policy is set.
    pub policy: Option<Policy>,
    /// The digest of that version's canonical encoding.
    pub digest: Option<String>,
    /// When a kept version takes effect.
    pub applies: &'static str,
}

/// The policy routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/policy", get(read).post(set))
}

/// Run `act` on the policies once they are settled.
pub(crate) fn with_policies<T>(
    state: &AppState,
    act: impl FnOnce(&mut PolicyStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .policies
        .as_ref()
        .ok_or_else(|| ServerError::PolicyUnavailable {
            reason: "the configuration names no policies_dir".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

/// The policy a launch of `agent` carries: its latest version with the
/// digest the runner checks. A missing policy refuses the launch.
pub(crate) fn launch_policy(
    state: &AppState,
    agent: &str,
) -> Result<Option<Box<lys_runner::admitted::Admitted>>, ServerError> {
    if state.policies.is_none() {
        return Err(ServerError::AgentHasNoPolicy {
            agent: agent.to_owned(),
        });
    }
    let latest = with_policies(state, |store| Ok(store.held().latest(agent).cloned()))?
        .ok_or_else(|| ServerError::AgentHasNoPolicy {
            agent: agent.to_owned(),
        })?;
    lys_runner::admitted::Admitted::of(latest)
        .map(|policy| Some(Box::new(policy)))
        .map_err(|error| ServerError::PolicyUnavailable {
            reason: error.to_string(),
        })
}

fn admitted(state: &AppState, headers: &HeaderMap, agent: &str) -> Result<(), ServerError> {
    let actor = signed_in(state, headers)?;
    let holder = Holder {
        kind: HolderKind::Agent,
        id: agent.to_owned(),
    };
    authorised(state, &actor, &holder).map(|_person| ())
}

fn view(agent: &str, policy: Option<Policy>) -> Result<Json<PolicyView>, ServerError> {
    let digest = policy.as_ref().map(digest).transpose()?;
    Ok(Json(PolicyView {
        agent: agent.to_owned(),
        policy,
        digest,
        applies: APPLIES,
    }))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
) -> Result<Json<PolicyView>, ServerError> {
    admitted(&state, &headers, &agent)?;
    let policy = with_policies(&state, |store| Ok(store.held().latest(&agent).cloned()))?;
    view(&agent, policy)
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
    given: Result<Json<PolicyBody>, JsonRejection>,
) -> Result<Json<PolicyView>, ServerError> {
    admitted(&state, &headers, &agent)?;
    let Json(given) = given.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let policy = Policy {
        version: given.version + 1,
        agent: agent.clone(),
        rules: given.rules,
    };
    let kept = with_policies(&state, |store| store.set(policy, given.version))?;
    view(&agent, Some(kept))
}
