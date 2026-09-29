//! First-run setup on Lys's own setup page, and the administrator's setup act.
//!
//! An install that names no administrator makes none. It writes a one-time
//! setup code's SHA-256 digest, with the code's purpose, to the code file
//! (owner-only), and hands the code itself to the browser only in the
//! fragment of the setup page's address, which never reaches a server log.
//! The code file is JSON, `{"purpose": "first-run" | "password", "sha256":
//! "<lowercase hex>"}`; `lys identity install` and `lys identity setup-code`
//! write it and this module alone reads and removes it.
//!
//! With a first-run code, while no administrator exists, the setup page
//! takes the person's name, email and password. The service makes the
//! account at the issuer through its API key, signs the person in with what
//! they typed exactly as the sign-in page does, records the signed-in login
//! as the administrator, runs the setup act for it, removes the code, and
//! answers with the session. With a password code, while an administrator
//! exists, the page takes a new password for the administrator's account,
//! which is how a forgotten password is recovered on the machine.
//!
//! Invariants: the code is never kept, printed or logged, only its digest is
//! read; a code is used once, because a completed setup removes its file; a
//! first-run code is refused `SetupClosed` once an administrator exists; the
//! administrator, once recorded, is never replaced; and no field is filled
//! from anything but what the person typed or the install was told.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{Extensions, HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{LoginBinding, OperationId, Profile};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::accounts;
use crate::config::Config;
use crate::directory_views::{PersonRegistered, receipt_view};
use crate::error::ServerError;
use crate::routes::{AppState, hex, signed_in, with_directory};
use crate::session::now;
use crate::sign_in::{Attempt, begin_session};

/// Where first-run setup reads its code and records its administrator.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupSettings {
    /// The owner-only file holding the pending setup code's purpose and
    /// digest, removed once the code is used.
    pub code_file: PathBuf,
    /// The file the administrator first-run setup made is recorded in.
    pub administrator_file: PathBuf,
    /// The administrator's email an unattended install was given: the only
    /// email the setup page then takes.
    #[serde(default)]
    pub email: Option<String>,
}

/// What a setup code lets the setup page do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    /// Make the first administrator.
    FirstRun,
    /// Set a new password for the administrator.
    Password,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCode {
    purpose: Purpose,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedAdministrator {
    issuer: String,
    subject: String,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::SetupUnavailable {
        reason: reason.into(),
    }
}

fn malformed(refused: &JsonRejection) -> ServerError {
    ServerError::RequestMalformed {
        reason: refused.body_text(),
    }
}

/// The digest a setup code is kept by: lowercase hex SHA-256 of its text.
pub fn digest(code: &str) -> String {
    hex(&Sha256::digest(code.as_bytes()))
}

/// Whether `a` and `b` are the same text, compared in time that does not
/// depend on where they first differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |differ, (x, y)| differ | (x ^ y))
            == 0
}

/// The administrator the service starts with: the configured one, or else
/// the one first-run setup recorded, or none yet.
pub fn administrator(config: &Config) -> Result<Option<LoginBinding>, ServerError> {
    if let Some(configured) = config.configured_administrator()? {
        return Ok(Some(configured));
    }
    match &config.setup {
        Some(settings) => read_administrator(&settings.administrator_file),
        None => Ok(None),
    }
}

fn read_administrator(path: &Path) -> Result<Option<LoginBinding>, ServerError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(unavailable(format!(
                "{} could not be read: {error}",
                path.display()
            )));
        }
    };
    let recorded: RecordedAdministrator = serde_json::from_str(&text).map_err(|error| {
        unavailable(format!(
            "{} is not a recorded administrator: {error}",
            path.display()
        ))
    })?;
    LoginBinding::new(&recorded.issuer, &recorded.subject)
        .map(Some)
        .map_err(|error| unavailable(format!("{}: {error}", path.display())))
}

fn record_administrator(path: &Path, login: &LoginBinding) -> Result<(), ServerError> {
    let recorded = RecordedAdministrator {
        issuer: login.issuer().to_owned(),
        subject: login.subject().to_owned(),
    };
    let text = serde_json::to_vec_pretty(&recorded)
        .map_err(|error| unavailable(format!("the administrator could not be written: {error}")))?;
    let fail = |error: std::io::Error| {
        unavailable(format!("{} could not be written: {error}", path.display()))
    };
    let mut file = std::fs::File::create(path).map_err(fail)?;
    file.write_all(&text).map_err(fail)?;
    file.sync_all().map_err(fail)
}

fn settings(state: &AppState) -> Result<&SetupSettings, ServerError> {
    state
        .setup
        .as_ref()
        .ok_or_else(|| unavailable("the configuration names no setup"))
}

fn pending(settings: &SetupSettings) -> Result<Option<PendingCode>, ServerError> {
    let path = &settings.code_file;
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(unavailable(format!(
                "the setup code file could not be read: {}",
                error.kind()
            )));
        }
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| unavailable(format!("the setup code file is not one: {error}")))
}

/// Admit `code` for `purpose`, or refuse: a first-run code once an
/// administrator exists is `SetupClosed`, and a code that is not the
/// pending one, is for the other purpose, or was used is `SetupCodeRefused`.
fn admit(state: &AppState, code: &str, purpose: Purpose) -> Result<(), ServerError> {
    let settings = settings(state)?;
    let held = pending(settings)?;
    let administrator = state.admission.administrator_login().is_some();
    if purpose == Purpose::FirstRun && administrator {
        return Err(ServerError::SetupClosed);
    }
    let Some(held) = held else {
        return Err(ServerError::SetupCodeRefused);
    };
    let fits = match held.purpose {
        Purpose::FirstRun => !administrator,
        Purpose::Password => administrator,
    };
    if held.purpose != purpose || !fits || !same(&digest(code), &held.sha256) {
        return Err(ServerError::SetupCodeRefused);
    }
    Ok(())
}

/// Remove the used code, so it is never taken again.
fn spend(settings: &SetupSettings) -> Result<(), ServerError> {
    match std::fs::remove_file(&settings.code_file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(unavailable(format!(
            "the used setup code could not be removed: {}",
            error.kind()
        ))),
    }
}

/// The purpose of the pending code, when `code` is it.
fn purpose_of(state: &AppState, code: &str) -> Result<Purpose, ServerError> {
    let held = pending(settings(state)?)?;
    let purpose = match held {
        Some(held) => held.purpose,
        None if state.admission.administrator_login().is_some() => {
            return Err(ServerError::SetupClosed);
        }
        None => return Err(ServerError::SetupCodeRefused),
    };
    admit(state, code, purpose)?;
    Ok(purpose)
}

/// The setup routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/setup", post(finish))
        .route("/setup/open", post(open))
        .route("/setup/administrator", post(make_administrator))
        .route("/setup/password", post(new_password))
}

/// The code the setup page was opened with.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Opened {
    code: String,
}

/// What the setup page shows for `code`: its purpose, the email it is for
/// when that is fixed, and the password policy.
async fn open(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Result<Json<Opened>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    state
        .sign_in
        .setup_attempts
        .admit(state.sign_in.address(&extensions, &headers)?, 5)?;
    let Json(body) = body.map_err(|refused| malformed(&refused))?;
    let purpose = purpose_of(&state, &body.code)?;
    let email = match purpose {
        Purpose::FirstRun => settings(&state)?.email.clone(),
        Purpose::Password => match state.admission.administrator_login() {
            Some(login) => {
                let api = crate::sign_in_providers::api(&state)?;
                Some(accounts::email_of(api, login.subject()).await?)
            }
            None => None,
        },
    };
    Ok(Json(json!({
        "purpose": purpose,
        "email": email,
        "policy": accounts::password_policy(),
    })))
}

/// What the setup page sends to make the first administrator. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewAdministrator {
    code: String,
    operation: String,
    display_name: String,
    email: String,
    password: String,
}

async fn make_administrator(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Result<Json<NewAdministrator>, JsonRejection>,
) -> Result<Response, ServerError> {
    state
        .sign_in
        .setup_attempts
        .admit(state.sign_in.address(&extensions, &headers)?, 5)?;
    let Json(body) = body.map_err(|refused| malformed(&refused))?;
    let turn = state.setup_lock.lock().await;
    admit(&state, &body.code, Purpose::FirstRun)?;
    let settings = settings(&state)?;
    let email = accounts::check_email(&body.email)?;
    if let Some(fixed) = &settings.email
        && !fixed.eq_ignore_ascii_case(email)
    {
        return Err(ServerError::AccountRefused {
            reason: format!("this install is set up for {fixed}; use that email"),
        });
    }
    let operation = OperationId::from_str(&body.operation)?;
    let profile = Profile::new(&body.display_name)?;
    accounts::check_password(&body.password)?;
    let address = state.sign_in.address(&extensions, &headers)?;
    let api = crate::sign_in_providers::api(&state)?;
    let (id, made) = accounts::make(api, email, &body.display_name).await?;
    let attempt = Attempt {
        email,
        password: &body.password,
        address,
    };
    // A setup retried after its account was made and its password set signs
    // in with that password as it stands: the issuer refuses a password
    // set again as one recently used.
    let held = if made {
        None
    } else {
        match state.sign_in.password(&state.oidc, &attempt).await {
            Ok(actor) => Some(actor),
            Err(ServerError::SignInRefused) => None,
            Err(error) => return Err(error),
        }
    };
    let actor = if let Some(actor) = held {
        actor
    } else {
        accounts::set_password(api, &id, &body.password).await?;
        state.sign_in.password(&state.oidc, &attempt).await?
    };
    if actor.binding().subject() != id {
        return Err(unavailable(
            "the account that signed in is not the account setup made",
        ));
    }
    record_administrator(&settings.administrator_file, actor.binding())?;
    state.admission.set_administrator(actor.binding().clone())?;
    let (person, receipt) = with_directory(&state, |directory| {
        Ok(directory.setup_person(actor.clone(), operation, profile, now())?)
    })?;
    spend(settings)?;
    drop(turn);
    let cookie = crate::session_admission::begin(&state, actor)?;
    let answer = PersonRegistered {
        person: person.to_string(),
        receipt: receipt_view(&receipt),
    };
    Ok(([(header::SET_COOKIE, cookie)], Json(answer)).into_response())
}

/// What the setup page sends to set the administrator's new password. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewPassword {
    code: String,
    password: String,
}

async fn new_password(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Result<Json<NewPassword>, JsonRejection>,
) -> Result<Response, ServerError> {
    state
        .sign_in
        .setup_attempts
        .admit(state.sign_in.address(&extensions, &headers)?, 5)?;
    let Json(body) = body.map_err(|refused| malformed(&refused))?;
    let turn = state.setup_lock.lock().await;
    admit(&state, &body.code, Purpose::Password)?;
    let login = state
        .admission
        .administrator_login()
        .ok_or(ServerError::SetupCodeRefused)?;
    let address = state.sign_in.address(&extensions, &headers)?;
    let api = crate::sign_in_providers::api(&state)?;
    accounts::set_password(api, login.subject(), &body.password).await?;
    let email = accounts::email_of(api, login.subject()).await?;
    let attempt = Attempt {
        email: &email,
        password: &body.password,
        address,
    };
    let actor = state.sign_in.password(&state.oidc, &attempt).await?;
    spend(settings(&state)?)?;
    drop(turn);
    begin_session(&state, &actor)
}

/// Browser-owned operation id and the name to display; identity claims are forbidden.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetupRequest {
    /// Retained across retries of this setup act.
    pub operation: String,
    /// The name shown in the directory, including single names.
    pub display_name: String,
}

/// Complete setup for a configured administrator already signed in, in one
/// signed log write, with no implied resource grants.
///
/// The caller is admitted before the body is read, so a body the route does
/// not take is refused `RequestMalformed` only to the administrator.
pub async fn finish(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SetupRequest>, JsonRejection>,
) -> Result<Json<PersonRegistered>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| malformed(&refused))?;
    let operation = OperationId::from_str(&body.operation)?;
    let profile = Profile::new(&body.display_name)?;
    with_directory(&state, |directory| {
        let (person, receipt) = directory.setup_person(actor, operation, profile, now())?;
        Ok(Json(PersonRegistered {
            person: person.to_string(),
            receipt: receipt_view(&receipt),
        }))
    })
}
