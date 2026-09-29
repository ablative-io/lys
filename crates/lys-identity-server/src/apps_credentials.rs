//! An administrator saves an issued app credential straight into Lys secrets.
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::apps_api::with_apps;
use crate::apps_binding::{acting, app_acting, sha256_hex};
use crate::apps_error::AppError;
use crate::apps_state::{Line, Standing};
use crate::error::ServerError;
use crate::routes::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveBody {
    client_secret: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    app: String,
    client_secret_ref: String,
    api_credential_ref: String,
}

/// Verify both the signed-in administrator and the current app credential
/// before handing it to the broker. No credential appears in the answer.
pub(crate) async fn save(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app): Path<String>,
    body: Result<Json<SaveBody>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Saved>, ServerError> {
    let Json(body) = body.map_err(|_error| ServerError::RequestMalformed {
        reason: "expected the issued client_secret and no other fields".to_owned(),
    })?;
    with_apps(&state, |apps| {
        acting(&state, apps.held(), &headers)?.administrator()?;
        app_acting(apps.held(), &app, &sha256_hex(&body.client_secret))?;
        Ok(())
    })?;
    let owner = crate::secrets_api::person(&state, &headers)?;
    let bytes = serde_json::to_vec(&json!({
        "app": app, "client_secret": body.client_secret,
        "upstream": state.identity_upstream,
    }))
    .map_err(|_error| ServerError::RequestMalformed {
        reason: "credential save cannot be encoded".to_owned(),
    })?;
    let answer = crate::secrets_api::ask(
        &state,
        &headers,
        Method::POST,
        "/_lys/apps/save",
        Bytes::from(bytes),
    )
    .await?;
    let invalid = || ServerError::SecretsUnavailable {
        reason: "the broker did not confirm this app credential save".to_owned(),
    };
    let saved: Saved = serde_json::from_value(answer).map_err(|_error| invalid())?;
    let prefix = format!("lys-app-{owner}-{app}");
    if saved.app != app
        || saved.client_secret_ref != format!("{prefix}-client")
        || saved.api_credential_ref != format!("{prefix}-api")
    {
        return Err(invalid());
    }
    Ok(Json(saved))
}

/// Save before activation. A repeated request reconciles the broker's sealed value.
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
    with_apps(state, |apps| {
        acting(state, apps.held(), headers)?.administrator()?;
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
