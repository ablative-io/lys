//! Lifecycle admission at HTTP ingress and session creation. Directory locks
//! end before a handler runs; creating a session holds the same lock as a
//! lifecycle transition, so a completed suspension cannot race a new session.

use std::sync::Arc;

use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
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
pub async fn guard(State(state): State<Arc<AppState>>, request: Request, next: Next) -> Response {
    let refused = match signed_in(&state, request.headers()) {
        Ok(actor) => with_directory(&state, |directory| {
            bound_caller(directory.projection()?, &actor, own_account_route(&request))
        })
        .err(),
        Err(ServerError::NotSignedIn) => None,
        Err(error) => Some(error),
    };
    // A refusal here comes before the body is read: the rest of the body is
    // read and dropped first, so the caller finishes sending and receives
    // the refusal, never a reset under it.
    match refused {
        Some(error) => crate::signed_first::refuse(request, error.into_response()).await,
        None => next.run(request).await,
    }
}

/// Begin a session while its bound identity may authenticate, serialized
/// with lifecycle transitions. Unbound first-run administrator login is kept.
///
/// # Errors
/// Refuses lifecycle admission, unavailable account lock or account read,
/// an account without its enabled flag, a disabled or changed binding,
/// or session clock, random source and durable write failures.
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
    let email = user
        .get("email")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let answer = with_directory(state, |directory| {
        let projection = directory.projection()?;
        if account_id(projection, &actor, state.oidc.issuer()).as_deref() != Some(&account) {
            return Err(ServerError::SignInRefused);
        }
        bound_caller(projection, &actor, true)?;
        theirs(state, projection, &actor, &email)?;
        state.sessions.begin(actor)
    });
    drop(guard);
    answer
}

/// An account opens its person unless Lys holds more than one address for
/// them (DIRECTORY-124 R4): that is a mix-up an administrator settles on
/// People by setting the one that is theirs, and until then every account
/// bound to them signs in as nobody. The administrator's own login is never
/// refused for it: they are the one who settles it.
fn theirs(
    state: &AppState,
    directory: &Projection,
    actor: &Actor,
    email: &str,
) -> Result<(), ServerError> {
    let Some(person) = directory.person_for(actor.binding()) else {
        return Ok(());
    };
    let administrator = state.admission.administrator_login()?;
    if administrator.is_some_and(|login| {
        login.issuer() == actor.binding().issuer() && login.subject() == actor.binding().subject()
    }) {
        return Ok(());
    }
    let foundings = crate::organisations_found::foundings(state)?;
    let own = foundings.addresses_for(&person.to_string());
    if own.len() > 1 {
        return Err(ServerError::SignInNotTheirs {
            account: crate::people_admit_handed::masked_email(email),
            held: own
                .iter()
                .map(|held| crate::people_admit_handed::masked_email(held))
                .collect::<Vec<_>>()
                .join(" and "),
        });
    }
    Ok(())
}

/// The account a sign-in opens: the login it signed in with, when that login
/// is at this issuer, so a person whose earlier account is gone signs in with
/// the one they hold now (DIRECTORY-124); else the person's first login here.
fn account_id(directory: &Projection, actor: &Actor, issuer: &str) -> Option<String> {
    let person = directory.person_for(actor.binding())?;
    if actor.binding().issuer() == issuer {
        return Some(actor.binding().subject().to_owned());
    }
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

/// Compare the issuer account email to Lys's held person address under the
/// same account hold as account writes and login moves, before any session.
pub(crate) async fn begin_provider(
    state: &AppState,
    login: crate::sign_in_callback::ProviderLogin,
) -> Result<String, ServerError> {
    let api = crate::sign_in_providers::api(state)?;
    let binding = login.actor.binding();
    if binding.issuer() != state.oidc.issuer() {
        return Err(ServerError::SignInRefused);
    }
    let guard = api.lock_account(binding.subject()).await?;
    let user = crate::accounts::read(api, binding.subject()).await?;
    if user.get("enabled").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(ServerError::SignInRefused);
    }
    let email = user
        .get("email")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in service's account answer has no email".to_owned(),
        })?;
    crate::accounts::check_email(email.trim())?;
    let provider = api
        .listed()
        .await?
        .into_iter()
        .find(|provider| provider.id == login.provider)
        .map_or(login.provider, |provider| provider.name);
    let answer = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = projection
            .person_for(login.actor.binding())
            .ok_or(ServerError::NoPerson)?
            .to_string();
        bound_caller(projection, &login.actor, true)?;
        let mut foundings = crate::organisations_found::foundings(state)?;
        let held: Vec<String> = foundings
            .addresses_for(&person)
            .into_iter()
            .map(str::to_owned)
            .collect();
        match held.as_slice() {
            [] => foundings.hold_email(&person, email.trim(), true)?,
            [one] if one.eq_ignore_ascii_case(email.trim()) => {}
            held => {
                return Err(ServerError::ProviderEmailMismatch {
                    provider,
                    account: crate::people_admit_handed::masked_email(email.trim()),
                    held: held
                        .iter()
                        .map(|held| crate::people_admit_handed::masked_email(held))
                        .collect::<Vec<_>>()
                        .join(" and "),
                });
            }
        }
        state.sessions.begin(login.actor)
    });
    drop(guard);
    answer
}

#[cfg(test)]
#[path = "session_admission_tests.rs"]
mod tests;
