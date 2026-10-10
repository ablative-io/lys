//! An agent asks Lys for a pass to an approved app (AGENTS-006 R1, R2), so
//! it reaches the app's store with its own grants, as a person and a
//! machine do with theirs.
//!
//! - `POST /agents/{id}/pass` `{"audience"}`: the agent `id` asks for a pass
//!   whose audience is the approved app `audience`. The answer is
//!   `{"pass", "expires_at", "audience"}` (`provider::agent_issue`).
//!
//! The agent proves the request is its own in one of two ways, and with no
//! other credential beside it:
//!
//! - a run Lys started presents its run pass (`lys-agent-pass`), judged as
//!   every route judges one (`agent_pass::holder`: the pass, and the seat
//!   that signed it); a pass that does not admit is refused
//!   `AgentPassRefused` before this route is reached, and a pass of another
//!   agent `agent_pass_unproven`;
//! - any other agent presents a grant credential (`lys-grant-token`) that
//!   the person responsible for one of its grants issued through
//!   `POST /grants/{id}/tokens`. The credential must be known, unrevoked and
//!   unexpired (`agent_pass_unproven` otherwise), and its grant must be held
//!   by this agent and live now (`agent_not_holder` otherwise).
//!
//! A retired or suspended agent is refused `agent_retired`, and an audience
//! that is not an approved app `audience_not_approved`. Each is judged at
//! the ask, so a credential revoked or an agent retired is refused at the
//! next one.
//!
//! Each pass issued is one line in the service's log naming the agent, the
//! audience and the proof used: the run pass, or the grant credential by
//! the grant it was issued for. Never the credential, the run pass or the
//! pass itself.
//!
//! The credential is looked up and its lock let go before the grants are
//! held; the grants are let go before the provider reads the apps.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::grants::GrantId;
use lys_identity::grants::admission::effective;
use lys_identity::projection::Projection;
use lys_identity::{AgentId, IdentityId, LifecycleState};
use serde::Deserialize;

use crate::error::ServerError;
use crate::error_app_pass::AppPassError;
use crate::grant_tokens::TokenError;
use crate::grants::Judged;
use crate::provider::{AgentAppPass, agent_app_pass};
use crate::routes::AppState;

/// An agent's request for a pass.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppPassAsked {
    /// The app the pass is for: its audience, an app approved on the Apps
    /// screen.
    audience: String,
}

/// How the agent proved the request its own.
#[derive(Debug, Clone, Copy)]
enum Proof {
    /// The run pass of a run Lys started.
    RunPass,
    /// A grant credential, issued for this grant.
    GrantCredential(GrantId),
}

/// The agents' pass route.
pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/pass", post(pass))
}

/// Whether `path` is an agent's ask for its pass, which this route judges
/// alone (`route_actions::admit` lets a run pass through to it).
pub(crate) fn asks_for_pass(path: &str) -> bool {
    path.strip_prefix("/agents/")
        .and_then(|rest| rest.strip_suffix("/pass"))
        .is_some_and(|id| !id.is_empty() && !id.contains('/'))
}

/// An agent's pass to an app, asked with its own proof.
async fn pass(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AgentAppPass>, ServerError> {
    let agent = AgentId::from_str(&id)?;
    let proof = proved(&state, &headers, agent)?;
    let asked: AppPassAsked =
        serde_json::from_slice(&body).map_err(|error| ServerError::RequestMalformed {
            reason: format!("the pass request does not read: {error}"),
        })?;
    let issued = agent_app_pass(&state, agent, &asked.audience)?;
    eprintln!("{}", issued_line(agent, &issued.audience, proof));
    Ok(Json(issued))
}

/// What a request with no proof is told to bring.
const PROOFS: &str = "bring the run pass (lys-agent-pass) or a grant credential (lys-grant-token)";

fn unproven(reason: impl Into<String>) -> ServerError {
    AppPassError::Unproven {
        reason: reason.into(),
    }
    .into()
}

fn not_holder(reason: String) -> ServerError {
    AppPassError::NotHolder { reason }.into()
}

fn unavailable(reason: String) -> ServerError {
    ServerError::AuthorityUnavailable { reason }
}

fn credential_refused(error: &TokenError) -> ServerError {
    unproven(format!("the grant credential does not admit: {error}"))
}

/// The proof `headers` carry that the request is `agent`'s, judged now.
fn proved(state: &AppState, headers: &HeaderMap, agent: AgentId) -> Result<Proof, ServerError> {
    for other in [
        header::AUTHORIZATION.as_str(),
        header::COOKIE.as_str(),
        crate::agent_signature::HEADER,
        crate::operator::HEADER,
    ] {
        if headers.contains_key(other) {
            let reason = format!("a second credential, {other}, came with the proof");
            return Err(unproven(reason));
        }
    }
    if headers.contains_key(crate::agent_pass::HEADER) {
        return run_pass(state, headers, agent);
    }
    if !headers.contains_key(crate::grant_tokens::HEADER) {
        return Err(unproven(PROOFS));
    }
    let token = match crate::grant_tokens::header(headers) {
        Ok(token) => token,
        Err(error) => return Err(credential_refused(&error)),
    };
    let at = crate::session::now();
    let grant = credential_grant(state, token, at)?;
    crate::grants::with_grants(state, |judged| held(&judged, grant, agent, at))?;
    Ok(Proof::GrantCredential(grant))
}

/// The run pass `headers` carry, when it is `agent`'s own and the agent
/// stands.
fn run_pass(state: &AppState, headers: &HeaderMap, agent: AgentId) -> Result<Proof, ServerError> {
    let Some(holder) = crate::agent_pass::holder(state, headers)? else {
        return Err(unproven("the run pass was not read"));
    };
    if holder != agent {
        let reason = format!("the run pass is agent {holder}'s, not {agent}'s");
        return Err(unproven(reason));
    }
    crate::grants::with_grants(state, |judged| standing(judged.directory, agent))?;
    Ok(Proof::RunPass)
}

/// The grant the grant credential `token` was issued for, when it admits at
/// `at`: known, unrevoked and unexpired.
fn credential_grant(state: &AppState, token: &str, at: u64) -> Result<GrantId, ServerError> {
    let tokens = match state.grant_tokens.lock() {
        Ok(tokens) => tokens,
        Err(error) => return Err(unavailable(error.to_string())),
    };
    let entry = match tokens.lookup(token, at) {
        Ok(entry) => entry,
        Err(TokenError::Unavailable(reason)) => return Err(unavailable(reason)),
        Err(error) => return Err(credential_refused(&error)),
    };
    match GrantId::from_str(&entry.grant) {
        Ok(grant) => Ok(grant),
        Err(error) => Err(unavailable(format!("grant {}: {error}", entry.grant))),
    }
}

/// Refuses unless `grant` is held by `agent`, the agent stands, and the
/// grant is live at `at`.
fn held(judged: &Judged<'_>, grant: GrantId, agent: AgentId, at: u64) -> Result<(), ServerError> {
    let book = judged.grants.book();
    let Some(record) = book.grant(grant) else {
        return Err(not_holder(format!("grant {grant} is unknown")));
    };
    let holder = record.holder();
    if holder != IdentityId::Agent(agent) {
        let reason = format!("grant {grant} is held by {holder}, not by agent {agent}");
        return Err(not_holder(reason));
    }
    standing(judged.directory, agent)?;
    if let Err(error) = effective(book, judged.directory, grant, at) {
        return Err(not_holder(format!("grant {grant} is not live: {error}")));
    }
    Ok(())
}

/// Refuses `agent` `agent_retired` when the directory records it retired or
/// suspended now.
fn standing(directory: &Projection, agent: AgentId) -> Result<(), ServerError> {
    let Some(record) = directory.record(IdentityId::Agent(agent)) else {
        return Err(unproven(format!("agent {agent} is not in the directory")));
    };
    let state = record.state();
    if matches!(state, LifecycleState::Retired | LifecycleState::Suspended) {
        let agent = agent.to_string();
        return Err(AppPassError::Retired { agent, state }.into());
    }
    Ok(())
}

/// The service's log line for a pass issued to `agent` for `audience` on
/// `proof`. It names the grant a credential was issued for, never the
/// credential.
fn issued_line(agent: AgentId, audience: &str, proof: Proof) -> String {
    let proof = match proof {
        Proof::RunPass => "run_pass".to_owned(),
        Proof::GrantCredential(grant) => format!("grant_credential for grant {grant}"),
    };
    format!(
        "lys-identity-server agent pass issued: agent {agent} audience {audience} proof {proof}"
    )
}

#[cfg(test)]
mod tests {
    use lys_identity::AgentId;
    use lys_identity::grants::GrantId;

    use super::{Proof, asks_for_pass, issued_line};

    #[test]
    fn only_an_agents_pass_path_is_its_ask() {
        assert!(asks_for_pass("/agents/agent-1/pass"));
        assert!(!asks_for_pass("/agents//pass"));
        assert!(!asks_for_pass("/agents/agent-1/start"));
        assert!(!asks_for_pass("/agents/agent-1/calls/pass"));
        assert!(!asks_for_pass("/runner/dial/op-1/pass"));
    }

    #[test]
    fn the_issue_line_names_the_agent_the_audience_and_the_proof() {
        let agent = AgentId::from_bytes([1; 16]);
        let grant = GrantId::from_bytes([2; 16]);
        let opening = format!("lys-identity-server agent pass issued: agent {agent}");
        assert_eq!(
            issued_line(agent, "notes", Proof::RunPass),
            format!("{opening} audience notes proof run_pass")
        );
        let credential = format!("grant_credential for grant {grant}");
        assert_eq!(
            issued_line(agent, "notes", Proof::GrantCredential(grant)),
            format!("{opening} audience notes proof {credential}")
        );
    }
}
