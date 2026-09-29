//! An agent's refused tool calls, read by its responsible person or an
//! administrator, newest first. The records are the runners' own, read
//! from their feeds; an agent statement that a call was denied is never
//! one. A viewer who may not see the agent is refused and shown nothing.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::budgets_api::{authorised, with_budgets};
use crate::budgets_state::{Holder, HolderKind};
use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// An agent's refusals.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RefusalsView {
    /// The agent.
    pub agent: String,
    /// Its refusals, newest first.
    #[schema(value_type = Vec<Object>)]
    pub refusals: Vec<lys_runner::refusals::RefusalRecord>,
    /// The runners whose feeds have been read, so a runner missing here is
    /// a source whose refusals are not yet covered.
    pub read_from: Vec<String>,
}

/// The refusal routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/refusals", get(read))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
) -> Result<Json<RefusalsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let holder = Holder {
        kind: HolderKind::Agent,
        id: agent.clone(),
    };
    authorised(&state, &actor, &holder)?;
    with_budgets(&state, |store| {
        let refusals = &store.held().refusals;
        Ok(Json(RefusalsView {
            refusals: refusals.of_agent(&agent),
            read_from: refusals.cursors.keys().cloned().collect(),
            agent,
        }))
    })
}
