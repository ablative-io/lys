//! Lifecycle admission at HTTP ingress and session creation. Directory locks
//! end before a handler runs; creating a session holds the same lock as a
//! lifecycle transition, so a completed suspension cannot race a new session.

use std::sync::Arc;

use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use lys_identity::projection::Projection;
use lys_identity::{Actor, IdentityId};

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
pub async fn begin(state: &AppState, actor: Actor) -> Result<String, ServerError> {
    let account = with_directory(state, |directory| {
        Ok(account_id(
            directory.projection()?,
            &actor,
            state.oidc.issuer(),
        ))
    })?;
    let (Some(api), Some(account)) = (&state.sign_in_providers, account) else {
        return begin_lifecycle(state, actor);
    };
    let guard = api.lock_account(&account).await?;
    let user = crate::accounts::read(api, &account).await?;
    let enabled = user
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in service's account answer has no boolean enabled field".to_owned(),
        })?;
    if !enabled {
        return Err(ServerError::SignInRefused);
    }
    let answer = with_directory(state, |directory| {
        let projection = directory.projection()?;
        if account_id(projection, &actor, state.oidc.issuer()).as_deref() != Some(&account) {
            return Err(ServerError::SignInRefused);
        }
        bound_caller(projection, &actor, true)?;
        state.sessions.begin(actor)
    });
    drop(guard);
    answer
}

fn account_id(directory: &Projection, actor: &Actor, issuer: &str) -> Option<String> {
    let person = directory.person_for(actor.binding())?;
    directory
        .record(IdentityId::Person(person))?
        .bindings()
        .iter()
        .find(|binding| binding.issuer() == issuer)
        .map(|binding| binding.subject().to_owned())
}

fn begin_lifecycle(state: &AppState, actor: Actor) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        bound_caller(directory.projection()?, &actor, true)?;
        state.sessions.begin(actor)
    })
}

const OWN_ACCOUNT_ROUTES: &[(Method, &str)] = &[
    (Method::GET, "me"),
    (Method::GET, "me/account"),
    (Method::GET, "sessions"),
    (Method::GET, "login"),
    (Method::GET, "callback"),
    (Method::GET, "sign-in/providers"),
    (Method::GET, "sign-in/providers/{id}"),
    (Method::GET, "auth/v1/providers/callback"),
    (Method::POST, "sign-in"),
    (Method::POST, "setup"),
    (Method::POST, "setup/open"),
    (Method::POST, "setup/administrator"),
    (Method::POST, "setup/password"),
    (Method::POST, "sessions/{id}/end"),
];

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
    OWN_ACCOUNT_ROUTES
        .iter()
        .any(|(method, path)| request.method() == method && route == *path)
}

/// Apply state admission to an API handler tree, including its own fallback.
pub(crate) fn guarded(routes: axum::Router, state: Arc<AppState>) -> axum::Router {
    routes.layer(axum::middleware::from_fn_with_state(state, guard))
}

#[cfg(test)]
#[path = "session_admission_tests.rs"]
mod tests;
