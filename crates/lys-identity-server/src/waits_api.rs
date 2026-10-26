//! Only a signed-in administrator may inspect or cancel outbound work.

use std::sync::Arc;
use axum::{Json, Router};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use serde::Serialize;
use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// Current outbound calls, oldest first.
#[derive(Serialize, utoipa::ToSchema)]
pub struct Waits { /// Calls that have not yet answered or closed.
    pub waits: Vec<crate::waits::Wait>,
    /// The change event the page waits after for its next list.
    pub generation: u64,
}

#[derive(serde::Deserialize)]
struct After { after: Option<u64> }

/// The administrator's cancellation receipt.
#[derive(Serialize, utoipa::ToSchema)]
pub struct Cancelled {
    /// The outbound call cancelled.
    pub cancelled: String,
    /// The administrator whose act ended it.
    pub by: String,
}

fn administrator(state: &AppState, headers: &HeaderMap) -> Result<String, ServerError> {
    let actor = signed_in(state, headers)?;
    if !crate::routes::is_administrator(state, &actor)? {
        return Err(ServerError::NotAdmitted { reason:"only an administrator may inspect or cancel outbound calls" });
    }
    Ok(format!("administrator {}", actor.binding().subject()))
}

async fn list(State(state): State<Arc<AppState>>, headers: HeaderMap, Query(after): Query<After>) -> Result<Json<Waits>, ServerError> {
    administrator(&state, &headers)?;
    let (generation, waits) = crate::waits::after(after.after).await?;
    administrator(&state, &headers)?;
    Ok(Json(Waits { waits, generation }))
}

async fn cancel(State(state): State<Arc<AppState>>, headers: HeaderMap, Path(id): Path<String>, body: Result<Json<crate::runner_api::Empty>, axum::extract::rejection::JsonRejection>) -> Result<Json<Cancelled>, ServerError> {
    let by = administrator(&state, &headers)?;
    body.map_err(|error| ServerError::RequestMalformed { reason:error.body_text() })?;
    crate::waits::cancel(&id, &by)?;
    Ok(Json(Cancelled { cancelled:id, by }))
}

/// The administrator's in-flight work routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/waits", get(list)).route("/waits/{id}/cancel", post(cancel))
}

#[cfg(test)]
#[path = "waits_api_tests.rs"]
mod tests;
