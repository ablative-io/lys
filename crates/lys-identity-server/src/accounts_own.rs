//! A person's own account: the account read every account screen shows, and
//! the person's own email and password. Split from `accounts.rs` so each file
//! stays under 500 lines (ADR-111).
//!
//! The account read says whether the account has a password, as the issuer's
//! account type says it (null when the issuer names a type Lys does not know,
//! which is then confirmed as though it had one), and the providers it signs
//! in with, by the name the sign-in page offers them under. A change to an account that has a password
//! is confirmed by it: a sign-in carried to the issuer exactly as the sign-in
//! page carries one. An account with no password, one that signs in through a
//! provider, has nothing to confirm with: its signed-in session is the only
//! proof there is, so its email is changed and a first password set with no
//! current one. The session's age is never a condition.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{Extensions, HeaderMap};
use lys_identity::Actor;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{body_of, change, check_email, check_password, email_of, read, refused};
use crate::error::ServerError;
use crate::routes::{AppState, signed_in};
use crate::sign_in::{Attempt, person_address};
use crate::sign_in_providers::{SignInProviders, api};

/// The account `actor` signed in with, when it is one at this service's issuer.
fn own_id(state: &AppState, actor: &Actor) -> Result<String, ServerError> {
    if actor.binding().issuer() == state.oidc.issuer() {
        Ok(actor.binding().subject().to_owned())
    } else {
        Err(refused("this sign-in is not a Lys account"))
    }
}

/// The account as a screen shows it: its email, whether it may sign in,
/// whether it has a password, and the providers it signs in with.
pub(super) async fn shown(api: &SignInProviders, id: &str) -> Result<Json<Value>, ServerError> {
    let user = read(api, id).await?;
    Ok(Json(json!({
        "email": user.get("email").cloned().unwrap_or(Value::Null),
        "enabled": user.get("enabled").cloned().unwrap_or(Value::Bool(true)),
        "password": has_password(&user),
        "providers": providers_of(api, &user).await?,
    })))
}

/// Whether the issuer holds a password for `user`, as its account type says;
/// unknown when the issuer names no type Lys knows. Unknown never refuses a
/// read or an administrator's change, and is never taken as "no password":
/// a change to such an account is confirmed by its password.
fn has_password(user: &Value) -> Option<bool> {
    match user.get("account_type").and_then(Value::as_str)? {
        "password" | "federated_password" => Some(true),
        "new" | "passkey" | "federated" | "federated_passkey" => Some(false),
        _ => None,
    }
}

/// The providers `user` signs in with, by the name the sign-in page offers
/// each under ("google"); a provider set some other way by its name at the
/// issuer, and one the issuer no longer lists by the issuer's id for it.
async fn providers_of(api: &SignInProviders, user: &Value) -> Result<Vec<String>, ServerError> {
    let Some(provider) = user.get("auth_provider_id").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };
    let listed = api.listed().await?;
    let name = listed
        .iter()
        .find(|listed| listed.id == provider)
        .map_or(provider, |listed| listed.name.as_str());
    Ok(vec![
        crate::sign_in_providers::Provider::from_name(name)
            .map_or_else(|| name.to_owned(), |offered| offered.name().to_lowercase()),
    ])
}

/// What confirms a change to account `id`: its password when it has one,
/// refused without it, and when the issuer does not say; nothing when it has none.
async fn confirmed(
    state: &AppState,
    extensions: &Extensions,
    id: &str,
    password: Option<&str>,
) -> Result<(), ServerError> {
    if has_password(&read(api(state)?, id).await?) == Some(false) {
        return Ok(());
    }
    match password {
        Some(password) => confirm(state, extensions, id, password).await,
        None => Err(refused(
            "this account has a password: send your current password to change it",
        )),
    }
}

/// Confirm that `password` is account `id`'s by signing in with it.
async fn confirm(
    state: &AppState,
    extensions: &Extensions,
    id: &str,
    password: &str,
) -> Result<(), ServerError> {
    let email = email_of(api(state)?, id).await?;
    let attempt = Attempt {
        email: &email,
        password,
        address: person_address(extensions)?,
    };
    let checked = state
        .sign_in
        .password(&state.oidc, &attempt)
        .await
        .map_err(|error| match error {
            ServerError::SignInRefused => refused("that is not your current password"),
            other => other,
        })?;
    if checked.binding().subject() == id {
        Ok(())
    } else {
        Err(refused("that password belongs to another account"))
    }
}

pub(super) async fn own_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = own_id(&state, &actor)?;
    shown(api(&state)?, &id).await
}

/// A new email, confirmed by the password when the account has one. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnEmail {
    email: String,
    /// The account's password; left out by an account that has none.
    #[serde(default)]
    password: Option<String>,
}

pub(super) async fn own_email(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    extensions: Extensions,
    body: Result<Json<OwnEmail>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = own_id(&state, &actor)?;
    let body = body_of(body)?;
    let email = check_email(&body.email)?.to_owned();
    confirmed(&state, &extensions, &id, body.password.as_deref()).await?;
    let api = api(&state)?;
    change(api, &id, |update| {
        update["email"] = Value::String(email);
        update["email_verified"] = Value::Bool(true);
    })
    .await?;
    shown(api, &id).await
}

/// A new password, confirmed by the current one when the account has one. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnPassword {
    /// The current password; left out by an account that has none.
    #[serde(default)]
    current: Option<String>,
    password: String,
}

pub(super) async fn own_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    extensions: Extensions,
    body: Result<Json<OwnPassword>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = own_id(&state, &actor)?;
    let body = body_of(body)?;
    let policy = state.password_policy.as_ref();
    check_password(policy, &body.password)?;
    confirmed(&state, &extensions, &id, body.current.as_deref()).await?;
    let api = api(&state)?;
    change(api, &id, |update| {
        update["password"] = Value::String(body.password);
        update["email_verified"] = Value::Bool(true);
    })
    .await?;
    shown(api, &id).await
}

#[cfg(test)]
#[path = "accounts_own_tests.rs"]
mod tests;
