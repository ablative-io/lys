//! Lifecycle admission at HTTP ingress and session creation. Directory locks
//! end before a handler runs; creating a session holds the same lock as a
//! lifecycle transition, so a completed suspension cannot race a new session.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use lys_identity::Actor;
use lys_identity::projection::Projection;

use crate::caller_admission::active_caller;
use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};

/// Enforce lifecycle for a bound identity. An unbound login keeps the route's
/// existing admission rules, including the administrator's first-run setup.
fn bound_caller(directory: &Projection, actor: &Actor) -> Result<(), ServerError> {
    match active_caller(directory, actor) {
        Ok(_) | Err(ServerError::NoPerson) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Refuse an inactive authenticated caller before any route can bypass a
/// person check through an administrator shortcut or swallow a refusal.
pub async fn guard(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, ServerError> {
    match signed_in(&state, request.headers()) {
        Ok(actor) => with_directory(&state, |directory| {
            bound_caller(directory.projection()?, &actor)
        })?,
        Err(ServerError::NotSignedIn) => {}
        Err(error) => return Err(error),
    }
    Ok(next.run(request).await)
}

/// Begin a session only while its bound identity may still act, serialized
/// with lifecycle transitions. Unbound first-run administrator login is kept.
pub fn begin(state: &AppState, actor: Actor) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        bound_caller(directory.projection()?, &actor)?;
        state.sessions.begin(actor)
    })
}
