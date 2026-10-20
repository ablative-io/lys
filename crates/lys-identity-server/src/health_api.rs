//! The service's own answer that it is serving: `GET /health` on the API
//! router, so `/api/health` when the screens are served.
//!
//! The answer is the service's name and its build, and, while an upgrade is
//! still reversible and kept the stores before organisations, where that
//! copy is (DIRECTORY-109 R2), so a put-back has the path from the fence
//! itself: no other configuration, key, grant or person, and no session is
//! asked for.
//! It asks no other service anything, so it never waits on `SpiceDB`, the
//! issuer or the database; whether those are ready is `lys identity health`'s
//! to say. The router exists only once the logs are opened and the listener
//! bound, so an answer means this process got that far.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::error::ServerError;
use crate::routes::AppState;

/// The name the health answer gives this service.
pub const SERVICE: &str = "lys-identity-server";

/// The health route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health))
}

/// The health answer: this service's name and build, and nothing else.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Health {
    /// The service answering.
    pub service: &'static str,
    /// Its build version.
    pub version: &'static str,
    /// Where the upgrade under way kept the directory and grant logs before
    /// the first boot under organisations rewrote them (DIRECTORY-109 R2):
    /// the put-back to the build before organisations. Absent when no
    /// upgrade is under way or it kept no copy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_organisation: Option<String>,
}

async fn health(State(state): State<Arc<AppState>>) -> Result<Json<Health>, ServerError> {
    let previous_organisation =
        crate::operator::organisation_copy(&state).map_err(|error| ServerError::ConfigInvalid {
            reason: format!("the upgrade record cannot be read: {error}"),
        })?;
    Ok(Json(Health {
        service: SERVICE,
        version: env!("CARGO_PKG_VERSION"),
        previous_organisation,
    }))
}

#[cfg(test)]
#[path = "health_api_tests.rs"]
mod tests;
