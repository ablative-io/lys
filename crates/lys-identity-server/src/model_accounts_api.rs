//! Model accounts: the long-lived tokens a person registers for the AIs Lys
//! starts (Tom, 3 October 2026). The token goes to the secrets broker and
//! is sealed there; this service never keeps, logs or answers it, and no
//! screen shows it again. Who may draw from an account is the grant on the
//! account's secret; who drew from it, and how many calls, the broker
//! counts and answers here.
//!
//! - `GET /model-accounts`: the accounts the signed-in person may see,
//!   each with who drew from it and how many calls
//! - `POST /model-accounts`: register one, as the administrator, for a
//!   program the harness catalogue says can be run from a model account
//! - `POST /model-accounts/retire`: retire one, as the person who
//!   registered it; every draw on it ends and its name is never used again

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Extension, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::error::ServerError;
use crate::harness_catalogue::Catalogue;
use crate::routes::{AppState, signed_in};
use crate::secrets_api::ask;

/// The model account routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/model-accounts", get(list).post(register))
        .route("/model-accounts/retire", post(retire))
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let answer = ask(
        &state,
        &headers,
        Method::GET,
        "/_lys/model/accounts",
        Bytes::new(),
    )
    .await?;
    Ok(Json(answer))
}

/// A registration as the browser sends it. It prints nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Register {
    operation: String,
    name: String,
    harness: String,
    token: String,
}

/// Registers a model account as the administrator: the program's route
/// comes from the harness catalogue, never from the browser, and the token
/// is handed to the broker in the one signed body that seals it.
async fn register(
    State(state): State<Arc<AppState>>,
    Extension(catalogue): Extension<Arc<Catalogue>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    // The parser's own words can quote what it could not read, so a body
    // that does not read is refused in fixed words.
    let asked: Register =
        serde_json::from_slice(&body).map_err(|_unread| ServerError::RequestMalformed {
            reason: "the body is not {operation, name, harness, token}, each a string".to_owned(),
        })?;
    drop(body);
    let token = Zeroizing::new(asked.token);
    let using = catalogue.model_account(&asked.harness).ok_or_else(|| {
        ServerError::ModelAccount {
            status: StatusCode::CONFLICT,
            refusal: crate::model_draw::UNSUPPORTED.to_owned(),
            reason: format!(
                "{} cannot be run from a model account: the harness catalogue names no way to give it an account's token",
                asked.harness
            ),
        }
    })?;
    let sealed = Zeroizing::new(
        serde_json::to_vec(&json!({
            "operation": asked.operation,
            "name": asked.name,
            "harness": asked.harness,
            "upstream": using.upstream,
            "header": using.header,
            "prefix": using.prefix,
            "token": token.as_str(),
        }))
        .map_err(|error| ServerError::SecretsUnavailable {
            reason: format!("the registration could not be written: {error}"),
        })?,
    );
    let answer = ask(
        &state,
        &headers,
        Method::POST,
        "/_lys/model/register",
        Bytes::copy_from_slice(&sealed),
    )
    .await?;
    Ok(Json(answer))
}

/// Retires a model account; the body is forwarded unchanged and the broker
/// checks the caller registered it.
async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    let answer = ask(&state, &headers, Method::POST, "/_lys/model/retire", body).await?;
    Ok(Json(answer))
}
