//! An app's credentials are made and sealed by the secrets broker before its
//! approval is kept, and no route answers either one's value.
use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::apps_api::with_apps;
use crate::apps_binding::acting;
use crate::apps_error::AppError;
use crate::apps_state::{Line, Standing};
use crate::error::ServerError;
use crate::routes::AppState;

/// The broker's references to an approved app's sealed credentials.
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    app: String,
    client_secret_ref: String,
    api_credential_ref: String,
}

/// Prepare before activation. A repeated request reconciles the broker's sealed value.
pub(crate) async fn prepare(
    state: &AppState,
    headers: &HeaderMap,
    app: &str,
) -> Result<(String, Saved), ServerError> {
    let owner = crate::secrets_api::person(state, headers)?;
    let body = json!({"app": app, "upstream": state.identity_upstream}).to_string();
    let mut answer = crate::secrets_api::ask(
        state,
        headers,
        Method::POST,
        "/_lys/apps/prepare",
        Bytes::from(body),
    )
    .await.map_err(|error| match error {
        ServerError::SecretsRefused { status, refusal, reason } => ServerError::SecretsUnavailable {
            reason: format!("app credential custody was not confirmed; broker answered {status}: {refusal}: {reason}"),
        },
        other => other,
    })?;
    let invalid = || ServerError::SecretsUnavailable {
        reason: "the broker did not confirm custody of this app's credentials".to_owned(),
    };
    let digest = answer
        .as_object_mut()
        .and_then(|answer| answer.remove("client_secret_sha256"))
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    let saved: Saved = serde_json::from_value(answer).map_err(|_error| invalid())?;
    let prefix = format!("lys-app-{owner}-{app}");
    if saved.app != app
        || saved.client_secret_ref != format!("{prefix}-client")
        || saved.api_credential_ref != format!("{prefix}-api")
    {
        return Err(invalid());
    }
    Ok((digest, saved))
}

/// Check the approval's authority, operation and standing before broker custody.
pub(crate) fn pending(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    operation: &str,
) -> Result<bool, ServerError> {
    with_apps(state, |apps, projection| {
        acting(state, apps.held(), headers, projection)?.administrator()?;
        if let Some(line) = apps.held().operation(operation) {
            return match line {
                Line::Approved(approved) if approved.app == id => Ok(false),
                _ => Err(AppError::AppOperationReused {
                    operation: operation.to_owned(),
                }
                .into()),
            };
        }
        let app = apps
            .app(id)
            .ok_or_else(|| AppError::AppUnknown { app: id.to_owned() })?;
        if app.standing() != Standing::Pending {
            return Err(AppError::AppDecided { app: id.to_owned() }.into());
        }
        Ok(true)
    })
}
