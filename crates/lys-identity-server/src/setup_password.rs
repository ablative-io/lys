//! Password recovery preserves every existing sign-in factor.

use super::{Purpose, admit, settings, spend};
use crate::accounts;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::sign_in::{Attempt, begin_session};
use axum::Json;
use axum::extract::{State, rejection::JsonRejection};
use axum::http::{Extensions, HeaderMap};
use axum::response::Response;
use lys_identity::recovery_events::Kind;
use serde::Deserialize;
use std::sync::Arc;

#[cfg(test)]
#[path = "setup_password_tests.rs"]
mod tests;

/// What the setup page sends to set the administrator's new password. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewPassword {
    code: String,
    password: String,
}

pub(super) async fn new_password(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Result<Json<NewPassword>, JsonRejection>,
) -> Result<Response, ServerError> {
    state
        .sign_in
        .setup_attempts
        .admit(state.sign_in.address(&extensions, &headers)?, 5)?;
    let Json(body) = body.map_err(|refused| super::malformed(&refused))?;
    let turn = super::turn(&state).await?;
    admit(&state, &body.code, Purpose::Password)?;
    let login = state
        .admission
        .administrator_login()?
        .ok_or(ServerError::SetupCodeRefused)?;
    let address = state.sign_in.address(&extensions, &headers)?;
    let api = crate::sign_in_providers::api(&state)?;
    let policy = state.password_policy.as_ref();
    accounts::check_password(policy, &body.password)?;
    crate::recovery_security::record(&state, Kind::PasswordResetStarted, &body.code)?;
    set_password_only(api, policy, login.subject(), &body.password).await?;
    crate::recovery_security::record(&state, Kind::PasswordResetCompleted, &body.code)?;
    // A completed reset remains one-time even when another sign-in factor is required.
    spend(settings(&state)?)?;
    let email = accounts::email_of(api, login.subject()).await?;
    let attempt = Attempt {
        email: &email,
        password: &body.password,
        address,
    };
    let actor = state.sign_in.password(&state.oidc, &attempt).await?;
    drop(turn);
    begin_session(&state, &actor).await
}

pub(crate) async fn set_password_only(
    api: &crate::sign_in_providers::SignInProviders,
    policy: Option<&accounts::PasswordPolicy>,
    id: &str,
    password: &str,
) -> Result<(), ServerError> {
    accounts::check_password(policy, password)?;
    accounts::change(api, id, |update| {
        update["password"] = serde_json::Value::String(password.to_owned());
    })
    .await
}
