//! The signed agent's own current grant scope.

use std::sync::Arc;

use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, header};
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::grants::GrantError;
use lys_identity::grants::admission::effective;
use lys_identity::{AgentId, IdentityId};
use serde::Serialize;

use crate::error::ServerError;
use crate::grant_contract::{RefusedView, ResourceView, WindowView};
use crate::grants::{Judged, with_grants};
use crate::routes::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AgentGrants {
    agent: String,
    grants: Vec<OwnGrant>,
    revision: u64,
}

#[derive(Serialize, utoipa::ToSchema)]
struct OwnGrant {
    grant: String,
    resource: ResourceView,
    actions: Vec<String>,
    window: WindowView,
    responsible: String,
    admission: ChainAdmission,
    holding: Holding,
}

#[derive(Serialize, utoipa::ToSchema)]
struct ChainAdmission {
    admitted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<RefusedView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = AgentGrantHolding)]
enum Holding {
    #[serde(rename = "judged by the route")]
    JudgedByTheRoute,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agent/grants", get(read))
}

async fn read(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Result<Json<AgentGrants>, ServerError> {
    if !headers.contains_key(crate::agent_signature::HEADER) {
        return Err(ServerError::AgentSignatureRefused {
            reason: "an agent signature is required",
        });
    }
    if headers.get_all(header::COOKIE).iter().any(|value| {
        value.to_str().is_ok_and(|cookies| {
            cookies.split(';').any(|cookie| {
                cookie
                    .trim()
                    .split_once('=')
                    .is_some_and(|(name, _)| name == crate::session::COOKIE)
            })
        })
    }) {
        return Err(ServerError::AgentSignatureRefused {
            reason: "an agent signature cannot carry a session cookie",
        });
    }
    // The signature covers an empty body, so a read that carries one is
    // refused rather than answered over bytes nobody signed.
    if headers.contains_key(header::TRANSFER_ENCODING)
        || headers
            .get(header::CONTENT_LENGTH)
            .is_some_and(|length| length.as_bytes() != b"0")
    {
        return Err(ServerError::AgentSignatureRefused {
            reason: "a signed grant read carries no body",
        });
    }
    let path = uri
        .path_and_query()
        .map_or(uri.path(), |path| path.as_str());
    with_grants(&state, |judged| {
        let agent = crate::agent_signature::signed_agent(
            &state,
            judged.directory,
            &headers,
            ("GET", path, &[]),
        )?
        .ok_or(ServerError::AgentSignatureRefused {
            reason: "an agent signature is required",
        })?;
        Ok(Json(owned(&judged, agent, crate::session::now())))
    })
}

fn owned(judged: &Judged<'_>, agent: AgentId, at: u64) -> AgentGrants {
    let book = judged.grants.book();
    let grants = book
        .held_by(IdentityId::Agent(agent))
        .filter_map(|record| {
            let grant = record.grant();
            let admission = match effective(book, judged.directory, grant.id(), at) {
                Ok(_) => ChainAdmission {
                    admitted: true,
                    refused: None,
                },
                Err(
                    GrantError::Revoked { .. }
                    | GrantError::Expired { .. }
                    | GrantError::NotStarted { .. },
                ) => return None,
                Err(error) => {
                    let refused =
                        crate::grant_sight::as_seen_by(judged, IdentityId::Agent(agent), error);
                    ChainAdmission {
                        admitted: false,
                        refused: Some(RefusedView {
                            refusal: refused.name(),
                            grant: None,
                            reason: refused.to_string(),
                        }),
                    }
                }
            };
            let parts = grant.parts();
            Some(OwnGrant {
                grant: grant.id().to_string(),
                resource: ResourceView {
                    kind: parts.resource.kind().to_owned(),
                    id: parts.resource.id().to_owned(),
                },
                actions: parts.actions.iter().map(ToString::to_string).collect(),
                window: WindowView {
                    starts_at: parts.window.starts_at(),
                    ends_at: parts.window.ends_at(),
                },
                responsible: parts.responsible.to_string(),
                admission,
                holding: Holding::JudgedByTheRoute,
            })
        })
        .collect();
    AgentGrants {
        agent: agent.to_string(),
        grants,
        revision: judged.grants.revision(),
    }
}

#[cfg(test)]
#[path = "agent_grants_tests.rs"]
mod tests;
