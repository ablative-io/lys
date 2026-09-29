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
use crate::error::ServerError;
use crate::routes::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveBody {
    client_secret: String,
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Saved {
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
