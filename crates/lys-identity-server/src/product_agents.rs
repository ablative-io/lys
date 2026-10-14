//! A product's person pass reads only that person's agents, without a dashboard session.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::{Json, Router, routing::get};
use lys_identity::{IdentityId, LifecycleState};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::routes::{Shared, with_directory};

/// No caller-selected person or other scope is accepted.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductAgentsQuery {}

/// An agent the pass holder may choose for a seat.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ProductAgent {
    /// The enduring directory identifier.
    id: String,
    /// The directory's display name.
    display_name: String,
    /// Null because the directory records no product handle.
    handle: Option<String>,
    /// The lifecycle state, so a chooser can distinguish inactive agents.
    state: String,
}

/// The person's own agents and the issuer's API address.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ProductAgents {
    /// All agents answering to the verified person, in identifier order.
    agents: Vec<ProductAgent>,
    /// The Lys API at the pass's trusted issuer.
    lys_url: String,
}

pub(crate) fn routes() -> Router<Shared> {
    Router::new().route("/apps/me/agents", get(agents))
}

async fn agents(
    State(state): State<Shared>,
    headers: HeaderMap,
    query: Result<Query<ProductAgentsQuery>, QueryRejection>,
) -> Result<Json<ProductAgents>, ServerError> {
    query.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let person = crate::provider::product_person(&state, &headers)?;
    let agents = with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let holder = projection
            .record(IdentityId::Person(person))
            .ok_or(ServerError::NoPerson)?;
        if holder.state() != LifecycleState::Active {
            return Err(ProviderError::HolderRetired {
                holder: person.to_string(),
                state: holder.state(),
            }
            .into());
        }
        projection
            .agents_of(person)
            .map(|entry| {
                let (id, record) = entry?;
                Ok(ProductAgent {
                    id: id.to_string(),
                    display_name: record.profile().display_name().to_owned(),
                    handle: None,
                    state: record.state().to_string(),
                })
            })
            .collect::<Result<Vec<_>, ServerError>>()
    })?;
    let provider = state
        .provider
        .as_ref()
        .ok_or_else(|| ProviderError::Unavailable {
            reason: "the configuration names no provider".to_owned(),
        })?;
    Ok(Json(ProductAgents {
        agents,
        lys_url: format!("{}/api", provider.issuer().trim_end_matches('/')),
    }))
}
