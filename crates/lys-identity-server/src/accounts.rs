//! People's accounts at the issuer, made and changed by this service through
//! the issuer's administration API and never by a person at the issuer.
//!
//! An account Lys makes has a password only. The password is sent to the
//! issuer and nowhere else: it is never kept here, logged, or part of an
//! error. A change reads the account first and writes it back whole with
//! the one field changed, because the issuer's update replaces every field
//! it is given. An incomplete account answer is refused before a write,
//! so an unknown saved value is never replaced by a guessed default.
//!
//! A person changes their own email, confirmed by their password, and their
//! own password, confirmed by the current one; the confirmation is a
//! sign-in carried to the issuer exactly as the sign-in page carries one.
//! The administrator changes any person's email, sets them a new password,
//! and enables or disables their sign-in. A person's account is the login
//! the directory binds them to at this service's issuer, never an email.
//!
//! The password policy is the issuer's own shipped policy, which no install
//! changes: at least 14 and at most 128 characters, with a lower-case
//! letter, an upper-case letter and a digit. It is checked here first so a
//! person is told in Lys's words before anything is sent; the issuer checks
//! it again and its refusal is answered in Lys's words too.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{Extensions, HeaderMap};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{Actor, IdentityId, PersonId};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};
use crate::sign_in::{Attempt, person_address};
use crate::sign_in_providers::{SignInProviders, api};

/// The fewest characters a password has, as the issuer's policy says.
pub const PASSWORD_MIN: usize = 14;
/// The most characters a password has.
pub const PASSWORD_MAX: usize = 128;

/// The password policy as the setup and account screens show it before a
/// password is sent.
pub fn password_policy() -> Value {
    json!({
        "length_min": PASSWORD_MIN,
        "length_max": PASSWORD_MAX,
        "lower_case": 1,
        "upper_case": 1,
        "digits": 1,
        "words": format!(
            "At least {PASSWORD_MIN} characters, with a lower-case letter, an upper-case letter and a digit."
        ),
    })
}

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::AccountRefused {
        reason: reason.into(),
    }
}

/// Refuse a password the policy does not take, in Lys's words.
pub fn check_password(password: &str) -> Result<(), ServerError> {
    let length = password.chars().count();
    if !(PASSWORD_MIN..=PASSWORD_MAX).contains(&length) {
        return Err(refused(format!(
            "a password has {PASSWORD_MIN} to {PASSWORD_MAX} characters"
        )));
    }
    let has = |test: fn(&char) -> bool| password.chars().any(|c| test(&c));
    let mixed =
        has(char::is_ascii_lowercase) && has(char::is_ascii_uppercase) && has(char::is_ascii_digit);
    if !mixed {
        return Err(refused(
            "a password has a lower-case letter, an upper-case letter and a digit",
        ));
    }
    Ok(())
}

/// Refuse what is not an email address: one `@` with text on each side, a
/// dot in the domain, and no space, control character or character an
/// address carries in its path or query. The 254-octet ceiling is RFC 5321's
/// (4.5.3.1.3: a path of 256 octets, less its angle brackets), which the
/// issuer's own email validation also holds to.
pub fn check_email(email: &str) -> Result<&str, ServerError> {
    let email = email.trim();
    let shaped = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains('@')
    });
    let clean = !email
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '/' | '?' | '#' | '%'));
    if shaped && clean && email.len() <= 254 {
        Ok(email)
    } else {
        Err(refused("that is not an email address"))
    }
}

/// The name the issuer keeps for a person, when `display_name` is one the
/// issuer takes: letters, digits, spaces, hyphens and apostrophes, at most 32,
/// the issuer's own user-name rule (Rauthy v0.36.2, `RE_USER_NAME`).
fn issuer_name(display_name: &str) -> Option<String> {
    let name = display_name.trim();
    let taken = !name.is_empty()
        && name.chars().count() <= 32
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\''));
    taken.then(|| name.to_owned())
}

fn user_id(user: &Value) -> Result<String, ServerError> {
    user.get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in service's account answer names no id".to_owned(),
        })
}

/// Answer an issuer refusal of an account change in Lys's words.
fn account_refusal(error: ServerError) -> ServerError {
    match error {
        ServerError::SignInProvidersRefused { status: 409, .. } => {
            refused("an account with that email already exists")
        }
        ServerError::SignInProvidersRefused { reason, .. } => refused(reason),
        other => other,
    }
}

/// The account held under `email`, if one is.
pub async fn find_by_email(
    api: &SignInProviders,
    email: &str,
) -> Result<Option<Value>, ServerError> {
    match api
        .call(reqwest::Method::GET, &format!("/users/email/{email}"), None)
        .await
    {
        Ok(user) => Ok(Some(user)),
        Err(ServerError::SignInProvidersRefused { status: 404, .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The account `id`.
pub async fn read(api: &SignInProviders, id: &str) -> Result<Value, ServerError> {
    api.call(reqwest::Method::GET, &format!("/users/{id}"), None)
        .await
}

/// Make an account for `email`, answering its id and `true`, or answer the
/// id of the account already held under `email` and `false`, so a setup that
/// is retried after the account was made finds it again.
pub async fn make(
    api: &SignInProviders,
    email: &str,
    display_name: &str,
) -> Result<(String, bool), ServerError> {
    if let Some(held) = find_by_email(api, email).await? {
        return Ok((user_id(&held)?, false));
    }
    let request = json!({
        "email": email,
        "given_name": issuer_name(display_name),
        "family_name": null,
        "language": "en",
        "groups": null,
        "roles": [],
        "user_expires": null,
    });
    let made = api
        .call(reqwest::Method::POST, "/users", Some(&request))
        .await
        .map_err(account_refusal)?;
    Ok((user_id(&made)?, true))
}

/// The account `user` as the issuer's update takes it, with nothing changed
/// and no password.
fn update_of(user: &Value) -> Result<Value, ServerError> {
    let fields = user
        .as_object()
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in service's account answer is not an object".to_owned(),
        })?;
    for name in [
        "email",
        "language",
        "roles",
        "enabled",
        "email_verified",
        "user_values",
    ] {
        if !fields.contains_key(name) {
            return Err(ServerError::SignInProvidersUnavailable {
                reason: format!("the sign-in service's account answer names no {name}"),
            });
        }
    }
    let mut update = serde_json::Map::new();
    for name in [
        "email",
        "given_name",
        "family_name",
        "language",
        "roles",
        "groups",
        "enabled",
        "email_verified",
        "user_expires",
        "user_values",
    ] {
        if let Some(value) = fields.get(name) {
            update.insert(name.to_owned(), value.clone());
        }
    }
    update.insert("password".to_owned(), Value::Null);
    Ok(Value::Object(update))
}

/// Read account `id`, let `change` change its update, and write it back.
pub async fn change(
    api: &SignInProviders,
    id: &str,
    change: impl FnOnce(&mut Value) + Send,
) -> Result<(), ServerError> {
    let user = read(api, id).await?;
    let mut update = update_of(&user)?;
    change(&mut update);
    api.call(reqwest::Method::PUT, &format!("/users/{id}"), Some(&update))
        .await
        .map_err(account_refusal)?;
    Ok(())
}

/// Set account `id`'s password, enabled and its email taken as verified,
/// after the policy has taken it.
pub async fn set_password(
    api: &SignInProviders,
    id: &str,
    password: &str,
) -> Result<(), ServerError> {
    check_password(password)?;
    change(api, id, |update| {
        update["password"] = Value::String(password.to_owned());
        update["enabled"] = Value::Bool(true);
        update["email_verified"] = Value::Bool(true);
    })
    .await
}

/// The email account `id` signs in with.
pub async fn email_of(api: &SignInProviders, id: &str) -> Result<String, ServerError> {
    read(api, id)
        .await?
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in service's account answer names no email".to_owned(),
        })
}

/// The account routes: a person's own, and the administrator's for anyone.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me/account", get(own_account))
        .route("/me/account/email", post(own_email))
        .route("/me/account/password", post(own_password))
        .route("/directory/people/{id}/account", get(account_of))
        .route("/directory/people/{id}/account/email", post(set_email))
        .route(
            "/directory/people/{id}/account/password",
            post(reset_password),
        )
        .route("/directory/people/{id}/account/enabled", post(set_enabled))
}

fn body_of<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    body.map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })
}

/// The account `actor` signed in with, when it is one at this service's issuer.
fn own_id(state: &AppState, actor: &Actor) -> Result<String, ServerError> {
    if actor.binding().issuer() == state.oidc.issuer() {
        Ok(actor.binding().subject().to_owned())
    } else {
        Err(refused("this sign-in is not a Lys account"))
    }
}

/// The account of person `id`: the login the directory binds them to at
/// this service's issuer.
fn account_id(state: &AppState, id: &str) -> Result<String, ServerError> {
    let person = PersonId::from_str(id)?;
    with_directory(state, |directory| {
        let record = directory
            .record(IdentityId::Person(person))?
            .ok_or_else(|| lys_identity::IdentityError::IdentityUnknown {
                identity: person.to_string(),
            })?;
        record
            .bindings()
            .iter()
            .find(|binding| binding.issuer() == state.oidc.issuer())
            .map(|binding| binding.subject().to_owned())
            .ok_or_else(|| refused("this person has no Lys account to change"))
    })
}

/// The account as a screen shows it.
async fn shown(api: &SignInProviders, id: &str) -> Result<Json<Value>, ServerError> {
    let user = read(api, id).await?;
    Ok(Json(json!({
        "email": user.get("email").cloned().unwrap_or(Value::Null),
        "enabled": user.get("enabled").cloned().unwrap_or(Value::Bool(true)),
    })))
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

async fn own_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = own_id(&state, &actor)?;
    shown(api(&state)?, &id).await
}

/// A new email, confirmed by the password. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnEmail {
    email: String,
    password: String,
}

async fn own_email(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    extensions: Extensions,
    body: Result<Json<OwnEmail>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = own_id(&state, &actor)?;
    let body = body_of(body)?;
    let email = check_email(&body.email)?.to_owned();
    confirm(&state, &extensions, &id, &body.password).await?;
    let api = api(&state)?;
    change(api, &id, |update| {
        update["email"] = Value::String(email);
        update["email_verified"] = Value::Bool(true);
    })
    .await?;
    shown(api, &id).await
}

/// A new password, confirmed by the current one. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnPassword {
    current: String,
    password: String,
}

async fn own_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    extensions: Extensions,
    body: Result<Json<OwnPassword>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = own_id(&state, &actor)?;
    let body = body_of(body)?;
    check_password(&body.password)?;
    confirm(&state, &extensions, &id, &body.current).await?;
    let api = api(&state)?;
    set_password(api, &id, &body.password).await?;
    shown(api, &id).await
}

async fn account_of(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let id = account_id(&state, &person)?;
    shown(api(&state)?, &id).await
}

/// A person's new email, set by the administrator.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewEmail {
    email: String,
}

async fn set_email(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person): Path<String>,
    body: Result<Json<NewEmail>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let id = account_id(&state, &person)?;
    let email = check_email(&body_of(body)?.email)?.to_owned();
    let api = api(&state)?;
    change(api, &id, |update| {
        update["email"] = Value::String(email);
        update["email_verified"] = Value::Bool(true);
    })
    .await?;
    shown(api, &id).await
}

/// A person's new password, set by the administrator. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reset {
    password: String,
}

async fn reset_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person): Path<String>,
    body: Result<Json<Reset>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let id = account_id(&state, &person)?;
    let api = api(&state)?;
    set_password(api, &id, &body_of(body)?.password).await?;
    shown(api, &id).await
}

/// Whether a person may sign in.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Enabled {
    enabled: bool,
}

async fn set_enabled(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person): Path<String>,
    body: Result<Json<Enabled>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let id = account_id(&state, &person)?;
    if actor.binding().subject() == id {
        return Err(refused(
            "the administrator does not disable their own sign-in",
        ));
    }
    let enabled = body_of(body)?.enabled;
    let api = api(&state)?;
    change(api, &id, |update| update["enabled"] = Value::Bool(enabled)).await?;
    shown(api, &id).await
}

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
