//! The service's own answer that it is serving: `GET /health` on the API
//! router, so `/api/health` when the screens are served.
//!
//! The answer is the service's name and its build and nothing else: no
//! configuration, path, key, grant or person, and no session is asked for.
//! It asks no other service anything, so it never waits on `SpiceDB`, the
//! issuer or the database; whether those are ready is `lys identity health`'s
//! to say. The router exists only once the logs are opened and the listener
//! bound, so an answer means this process got that far.

use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::routes::AppState;

/// The name the health answer gives this service.
pub const SERVICE: &str = "lys-identity-server";

/// The health route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<Value> {
    Json(json!({ "service": SERVICE, "version": env!("CARGO_PKG_VERSION") }))
}

#[cfg(test)]
#[path = "health_api_tests.rs"]
mod tests;
