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
//! An own email change requires single-use codes mailed to the new and held
//! addresses, as well as the current password when held. Password confirmation is a
//! sign-in carried to the issuer exactly as the sign-in page carries one.
//! The administrator changes any person's email, sets them a new password,
//! and enables or disables their sign-in. A person's account is the login
//! the directory binds them to at this service's issuer, never an email.
//!
//! The password policy is Lys's own, [`PasswordPolicy`], as the service's
//! configuration states it: the install writes the same policy to the
//! issuer, which enforces it on every password set, and the issuer is never
//! asked for its own. The screens show it before a password is sent, it is
//! checked here first so a person is told in Lys's words, and the issuer's
//! refusal is answered in Lys's words too. A configuration that names no
//! policy leaves the issuer's check alone, and the screens show none.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::FromRequest;
use axum::extract::rejection::{BytesRejection, JsonRejection};
use axum::extract::{OriginalUri, Path, State};
use axum::http::{HeaderMap, Request};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::PersonId;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};
use crate::sign_in_providers::{SignInProviders, api};

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::AccountRefused {
        reason: reason.into(),
    }
}

#[path = "accounts_agent.rs"]
mod agent;
#[path = "accounts_own.rs"]
mod own;
#[path = "accounts_policy.rs"]
mod policy;
use own::shown;
pub(crate) use own::{ConfirmEmail, OwnEmail, OwnPassword, PendingEmail};
pub use policy::{PasswordPolicy, check_email, check_password};

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

/// A person's name as a screen sent it: the name people see and, when the
/// screen asked for them apart, their first and last names.
#[derive(Clone, Copy)]
pub struct AccountName<'a> {
    /// The name people see in the directory.
    pub display: &'a str,
    /// The first name, when the screen asked for it.
    pub given: Option<&'a str>,
    /// The last name, when the screen asked for it.
    pub family: Option<&'a str>,
}

impl AccountName<'_> {
    /// The given and family names the issuer keeps: the first and last
    /// names when both were sent and are not blank, else the whole name
    /// people see as the given name and no family name. A name the issuer
    /// would refuse is left out, as [`issuer_name`] says.
    fn issuer(&self) -> (Option<String>, Option<String>) {
        let filled = |name: &&str| !name.trim().is_empty();
        match (self.given.filter(filled), self.family.filter(filled)) {
            (Some(given), Some(family)) => (issuer_name(given), issuer_name(family)),
            _ => (issuer_name(self.display), None),
        }
    }
}

pub(crate) fn user_id(user: &Value) -> Result<String, ServerError> {
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
///
/// # Errors
/// Propagates sign-in service transport and non-404 refusals.
pub async fn find_by_email(
    api: &SignInProviders,
    email: &str,
) -> Result<Option<Value>, ServerError> {
    match api.account_by_email(email).await {
        Ok(user) => Ok(Some(user)),
        Err(ServerError::SignInProvidersRefused { status: 404, .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The account `id`.
///
/// # Errors
/// Propagates sign-in service transport failures and its named account refusal.
pub async fn read(api: &SignInProviders, id: &str) -> Result<Value, ServerError> {
    api.call(reqwest::Method::GET, &format!("/users/{id}"), None)
        .await
}

/// Make an account for `email`, answering its id and `true`, or answer the
/// id of the account already held under `email` and `false`, so a setup that
/// is retried after the account was made finds it again.
///
/// # Errors
/// Refuses sign-in service lookup or account creation failures, or an
/// account answer without a readable user id.
pub async fn make(
    api: &SignInProviders,
    email: &str,
    name: AccountName<'_>,
) -> Result<(String, bool), ServerError> {
    if let Some(held) = find_by_email(api, email).await? {
        return Ok((user_id(&held)?, false));
    }
    let (given_name, family_name) = name.issuer();
    let request = json!({
        "email": email,
        "given_name": given_name,
        "family_name": family_name,
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
///
/// # Errors
/// Refuses an unavailable account lock, an unreadable account update,
/// or sign-in service read and update failures.
pub async fn change(
    api: &SignInProviders,
    id: &str,
    change: impl FnOnce(&mut Value) + Send,
) -> Result<(), ServerError> {
    let guard = api.lock_account(id).await?;
    let user = read(api, id).await?;
    let mut update = update_of(&user)?;
    change(&mut update);
    let answer = api
        .call(reqwest::Method::PUT, &format!("/users/{id}"), Some(&update))
        .await
        .map_err(account_refusal)
        .map(|_| ());
    drop(guard);
    answer
}

/// Set account `id`'s password, enabled and its email taken as verified,
/// after `policy` has taken it.
///
/// # Errors
/// Refuses a password the configured policy does not take or sign-in
/// service account update failures.
pub async fn set_password(
    api: &SignInProviders,
    policy: Option<&PasswordPolicy>,
    id: &str,
    password: &str,
) -> Result<(), ServerError> {
    check_password(policy, password)?;
    change(api, id, |update| {
        update["password"] = Value::String(password.to_owned());
        update["enabled"] = Value::Bool(true);
        update["email_verified"] = Value::Bool(true);
    })
    .await
}

/// The email account `id` signs in with.
///
/// # Errors
/// Refuses sign-in service account read failures or an account answer
/// naming no email.
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
        .route("/me/account", get(own::own_account))
        .route("/me/account/email", post(own::own_email))
        .route("/me/account/email/confirm", post(own::confirm_email))
        .route("/me/account/password", post(own::own_password))
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

/// The account of person `id`: their live account at this service's issuer,
/// never a binding whose account the issuer no longer has (DIRECTORY-124).
async fn account_id(state: &AppState, api: &SignInProviders, id: &str) -> Result<String, ServerError> {
    crate::sign_in_reconcile::live_account(state, api, PersonId::from_str(id)?)
        .await?
        .ok_or_else(|| refused("this person has no Lys account to change"))
}

async fn account_of(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(person): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    agent::exercised(&state, &actor, &person, "read")?;
    crate::routes::administrator(&state, &actor)?;
    let api = api(&state)?;
    // A person with no live account is read as missing, so People can set
    // the address Lys makes their account under (DIRECTORY-124 R3).
    let Some(id) = crate::sign_in_reconcile::live_account(&state, api, PersonId::from_str(&person)?).await? else {
        return Ok(Json(json!({ "email": null, "enabled": false, "missing": true })));
    };
    shown(api, &id).await
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
    agent::exercised(&state, &actor, &person, "person.email.set")?;
    crate::routes::administrator(&state, &actor)?;
    let email = check_email(&body_of(body)?.email)?.to_owned();
    let api = api(&state)?;
    // A person with no live account has the address held, and the repair
    // finds or makes their account under it (DIRECTORY-124 R3).
    let held = PersonId::from_str(&person)?;
    let Some(id) = crate::sign_in_reconcile::live_account(&state, api, held).await? else {
        crate::sign_in_reconcile::hold_address(&state, api, held, &email).await?;
        crate::sign_in_reconcile::after_change(&state, &actor).await;
        return match crate::sign_in_reconcile::live_account(&state, api, held).await? {
            Some(made) => shown(api, &made).await,
            None => Ok(Json(json!({ "email": null, "enabled": false, "missing": true }))),
        };
    };
    let guard = api.lock_account(&id).await?;
    email_changes::write_email(&state, api, &id, &person, &email).await?;
    drop(guard);
    let answer = shown(api, &id).await?;
    crate::sign_in_reconcile::after_change(&state, &actor).await;
    Ok(answer)
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
    crate::routes::administrator(&state, &actor)?;
    let api = api(&state)?;
    let id = account_id(&state, api, &person).await?;
    let policy = state.password_policy.as_ref();
    set_password(api, policy, &id, &body_of(body)?.password).await?;
    shown(api, &id).await
}

/// Whether a person may sign in.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Enabled {
    enabled: bool,
}

#[path = "accounts_giving.rs"]
mod giving;

async fn set_enabled(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(person): Path<String>,
    principal: Option<axum::Extension<crate::agent_signature::TokenPrincipal>>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<Value>, ServerError> {
    let bytes = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let actor = crate::routes::people_giving::actor(
        &state,
        &headers,
        ("POST", uri.path(), &bytes),
        principal.as_ref().map(|value| &value.0),
    )?;
    if actor.provenance().agent().is_none() {
        crate::routes::administrator(&state, &actor)?;
    }
    let mut request = Request::new(Body::from(bytes));
    *request.headers_mut() = headers;
    let Json(body) = Json::<Enabled>::from_request(request, &())
        .await
        .map_err(|error| ServerError::RequestMalformed {
            reason: error.body_text(),
        })?;
    if actor.provenance().agent().is_some() {
        return giving::enabled(&state, &actor, PersonId::from_str(&person)?, body.enabled).await;
    }
    let api = api(&state)?;
    let id = account_id(&state, api, &person).await?;
    if actor.binding().subject() == id {
        return Err(refused(
            "the administrator does not disable their own sign-in",
        ));
    }
    let enabled = body.enabled;
    let guard = api.lock_account(&id).await?;
    let mut update = update_of(&read(api, &id).await?)?;
    update["enabled"] = Value::Bool(enabled);
    if !enabled {
        let person = PersonId::from_str(&person)?;
        with_directory(&state, |directory| {
            let projection = directory.projection()?;
            state
                .sessions
                .revoke_matching(|actor| projection.person_for(actor.binding()) == Some(person))
        })?;
        if let Some(provider) = &state.provider {
            provider.revoke_person(&person.to_string())?;
        }
    }
    api.call(reqwest::Method::PUT, &format!("/users/{id}"), Some(&update))
        .await
        .map_err(account_refusal)?;
    drop(guard);
    shown(api, &id).await
}

#[path = "accounts_email.rs"]
mod email_changes;

#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "accounts_changes_tests.rs"]
pub(crate) mod changes_tests;

#[cfg(test)]
#[path = "accounts_giving_tests.rs"]
mod giving_tests;
