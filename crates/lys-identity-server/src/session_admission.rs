//! Lifecycle admission at HTTP ingress and session creation. Directory locks
//! end before a handler runs; creating a session holds the same lock as a
//! lifecycle transition, so a completed suspension cannot race a new session.

use std::sync::Arc;

use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use lys_identity::Actor;
use lys_identity::projection::Projection;

use crate::caller_admission::{active_caller, authenticating_caller};
use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};

/// Enforce lifecycle for a bound identity. An unbound login keeps the route's
/// existing admission rules, including the administrator's first-run setup.
fn bound_caller(
    directory: &Projection,
    actor: &Actor,
    own_account: bool,
) -> Result<(), ServerError> {
    let answer = if own_account {
        authenticating_caller(directory, actor)
    } else {
        active_caller(directory, actor)
    };
    match answer {
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
            bound_caller(directory.projection()?, &actor, own_account_route(&request))
        })?,
        Err(ServerError::NotSignedIn) => {}
        Err(error) => return Err(error),
    }
    Ok(next.run(request).await)
}

/// Begin a session while its bound identity may authenticate, serialized
/// with lifecycle transitions. Unbound first-run administrator login is kept.
pub fn begin(state: &AppState, actor: Actor) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        bound_caller(directory.projection()?, &actor, true)?;
        state.sessions.begin(actor)
    })
}

/// Explicit exceptions to C39: authentication (lifecycle contract section 2),
/// setup's existing code/bootstrap rules, own-account reads, and ending one's
/// own session. No transition or account mutation is an exception. Match the
/// router's template and method, never an untrusted URL suffix.
fn own_account_route(request: &Request) -> bool {
    let Some(route) = request.extensions().get::<MatchedPath>() else {
        return false;
    };
    let route = route.as_str();
    let route = route.strip_prefix("/api/").unwrap_or(route);
    let route = route.strip_prefix('/').unwrap_or(route);
    match *request.method() {
        Method::GET => matches!(
            route,
            "me" | "me/account"
                | "sessions"
                | "login"
                | "callback"
                | "sign-in/providers"
                | "sign-in/providers/{id}"
                | "auth/v1/providers/callback"
        ),
        Method::POST => matches!(
            route,
            "sign-in"
                | "setup"
                | "setup/open"
                | "setup/administrator"
                | "setup/password"
                | "sessions/{id}/end"
        ),
        _ => false,
    }
}

/// Apply state admission to an API handler tree, including its own fallback.
pub(crate) fn guarded(routes: axum::Router, state: Arc<AppState>) -> axum::Router {
    routes.layer(axum::middleware::from_fn_with_state(state, guard))
}
