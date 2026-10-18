//! Connect an app in one act (the app-manifest pattern): the administrator
//! approves the manifest the browser fetched from the app, and Lys registers,
//! approves and issues the app's credentials with the existing acts, keeping
//! them behind a one-time code the app's server exchanges.
//!
//! Lys still calls no app: the manifest arrives in the administrator's body
//! and the return address is only answered, never fetched.
//!
//! The code is 256 bits, is used once and is bound to its app by a SHA-256
//! lookup. An encrypted operation checkpoint retains
//! its outcome across retries and restarts; consuming a code durably removes
//! the credential values before answering the exchange.

#[cfg(test)]
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{AppApprovalBody, RegisterBody, approve, malformed, redirect, register, with_apps};
use crate::apps_binding::{new_secret, sha256_hex};
use crate::apps_client_credentials::{
    ClientCredentialIssueBody, ClientCredentialRevokeBody, issue, issue_bearer, revoke,
};
use crate::apps_error::AppError;
use crate::apps_state::Standing;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// How long a connect code is exchangeable, in milliseconds.
const CODE_LIFETIME_MS: u64 = 10 * 60 * 1000;

#[path = "apps_connect_checkpoint.rs"]
mod checkpoint;

/// The connect routes, merged into the app routes.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps/connect", post(connect))
        .route("/apps/connect/exchange", post(exchange))
}

/// What an app publishes at `{app}/identity/lys/manifest`.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Manifest {
    id: String,
    name: String,
    redirects: Vec<String>,
    #[serde(default)]
    post_logout_redirects: Vec<String>,
    #[serde(default)]
    profile: bool,
    #[schema(value_type = Object)]
    schema: Value,
}

/// The administrator's approval of a fetched manifest.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConnectBody {
    operation: String,
    manifest: Manifest,
    #[serde(rename = "return")]
    back: String,
    state: String,
}

/// Where the approval page sends the browser.
#[derive(Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Connected {
    redirect: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExchangeBody {
    code: String,
}

/// What the app's server collects, once.
#[derive(Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct Exchanged {
    app: String,
    issuer: String,
    client_id: String,
    client_secret: String,
    workspace_credential: String,
}

struct Pending {
    at: u64,
    answer: Option<Exchanged>,
}

#[cfg(test)]
fn codes() -> &'static Mutex<HashMap<String, Pending>> {
    static CODES: OnceLock<Mutex<HashMap<String, Pending>>> = OnceLock::new();
    CODES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn unavailable(reason: &str) -> ServerError {
    AppError::AppsUnavailable {
        reason: reason.to_owned(),
    }
    .into()
}

/// The `scheme://host:port` of `address`.
fn origin(address: &str) -> Option<String> {
    reqwest::Url::parse(address)
        .ok()
        .map(|url| url.origin().ascii_serialization())
}

/// Refuse a return address that is not a sign-in address by Lys's rule or
/// that lies outside every origin the manifest's return addresses name.
pub(crate) fn return_admitted(back: &str, redirects: &[String]) -> Result<(), ServerError> {
    redirect(back)?;
    let wanted = origin(back);
    if wanted.is_some() && redirects.iter().any(|known| origin(known) == wanted) {
        return Ok(());
    }
    Err(AppError::RedirectInvalid {
        address: back.to_owned(),
        reason: "is outside every origin the app's manifest names",
    }
    .into())
}

fn operation(parent: &str, phase: &str) -> String {
    let digest = sha256_hex(&format!("lys/app-connect/v1/{parent}/{phase}"));
    format!("op-{}", &digest[..32])
}

async fn prepare_app(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    manifest: &Manifest,
    operation_id: &str,
) -> Result<(), ServerError> {
    let id = manifest.id.clone();
    let standing = with_apps(state, |apps, _| {
        Ok(apps.app(&id).map(crate::apps_state::App::standing))
    })?;
    match standing {
        None => {
            drop(
                register(
                    State(Arc::clone(state)),
                    headers.clone(),
                    Ok(Json(RegisterBody {
                        operation: operation(operation_id, "register"),
                        id: id.clone(),
                        name: manifest.name.clone(),
                        redirects: manifest.redirects.clone(),
                        schema: manifest.schema.clone(),
                        service_account: None,
                    })),
                )
                .await?,
            );
        }
        Some(Standing::Pending | Standing::Approved) => {}
        Some(Standing::Declined) => return Err(AppError::AppDecided { app: id }.into()),
        Some(Standing::Retired) => return Err(AppError::AppRetired { app: id }.into()),
    }
    if standing == Some(Standing::Approved) {
        drop(
            crate::apps_sign_in::set(
                State(Arc::clone(state)),
                headers.clone(),
                UrlPath(id.clone()),
                Ok(Json(crate::apps_sign_in::AppSignInBody {
                    operation: operation(operation_id, "sign-in"),
                    redirects: manifest.redirects.clone(),
                    profile: manifest.profile,
                })),
            )
            .await?,
        );
    } else {
        drop(
            approve(
                State(Arc::clone(state)),
                headers.clone(),
                UrlPath(id.clone()),
                Ok(Json(AppApprovalBody {
                    operation: operation(operation_id, "approve"),
                    redirects: manifest.redirects.clone(),
                    profile: manifest.profile,
                })),
            )
            .await?,
        );
    }
    Ok(())
}

async fn replace_credentials(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    record: &mut checkpoint::Record,
    operation_id: &str,
    checkpoint: &checkpoint::Checkpoint,
) -> Result<(), ServerError> {
    let id = &record.app;
    if record.client.is_none() {
        let Json(client) = issue(
            State(Arc::clone(state)),
            headers.clone(),
            UrlPath(id.to_owned()),
            Ok(Json(ClientCredentialIssueBody {
                operation: operation_id.to_owned(),
            })),
        )
        .await?;
        let value = client.credential.ok_or_else(|| unavailable(
            "the client issue is recorded but its one-time value was not checkpointed; the old credentials remain live; start a new connect operation"))?;
        record.client = Some((client.credential_id, Some(value)));
        checkpoint.save(record)?;
    }
    if record.bearer.is_none() {
        let Json(bearer) = issue_bearer(
            State(Arc::clone(state)),
            headers.clone(),
            UrlPath(id.to_owned()),
            Ok(Json(ClientCredentialIssueBody {
                operation: operation(operation_id, "bearer"),
            })),
        )
        .await?;
        let value = bearer.credential.ok_or_else(|| unavailable(
            "the bearer issue is recorded but its one-time value was not checkpointed; start a new connect operation"))?;
        record.bearer = Some((bearer.reference, Some(value)));
        checkpoint.save(record)?;
    }
    Ok(())
}

fn complete(
    checkpoint: &checkpoint::Checkpoint,
    record: &mut checkpoint::Record,
    back: &str,
    state: &str,
) -> Result<Connected, ServerError> {
    let (code, _) = new_secret()?;
    let mut back = reqwest::Url::parse(back)
        .map_err(|error| malformed(format!("the return address is invalid: {error}")))?;
    back.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", state);
    let connected = Connected {
        redirect: back.to_string(),
    };
    record.code = Some(code);
    record.at = now();
    record.connected = Some(connected.clone());
    checkpoint.save(record)?;
    checkpoint.index(record)?;
    Ok(connected)
}

async fn connect(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ConnectBody>, JsonRejection>,
) -> Result<Json<Connected>, ServerError> {
    let Json(mut body) = body.map_err(|refused| malformed(refused.body_text()))?;
    body.operation = OperationId::from_str(&body.operation)?.to_string();
    if body.state.is_empty()
        || body
            .state
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(malformed("a connection carries the app's state, one word"));
    }
    return_admitted(&body.back, &body.manifest.redirects)?;
    with_apps(&state, |apps, directory| {
        super::administrator(&state, apps, &headers, directory)
    })?;
    let checkpoint = checkpoint::Checkpoint::open(&state)?;
    let gate = checkpoint.gate().await?;
    let by = with_apps(&state, |apps, directory| {
        super::administrator(&state, apps, &headers, directory)
    })?;
    let request = sha256_hex(
        &serde_json::to_string(&json!({"body":body,"by":by}))
            .map_err(|error| checkpoint::json_failed("encode its request", &error))?,
    );
    let manifest = body.manifest;
    let id = manifest.id.clone();
    if id == lys_identity::grants::LYS_APP {
        return Err(AppError::AppIsLys {
            reason: "the app lys is Lys itself and is never connected from a manifest",
        }
        .into());
    }
    let mut record = match checkpoint.load(&body.operation)? {
        Some(record) if record.request == request && record.app == id => record,
        Some(_) => {
            return Err(AppError::AppOperationReused {
                operation: body.operation,
            }
            .into());
        }
        None => {
            let prior = with_apps(&state, |apps, _| {
                if apps.held().operation(&body.operation).is_some() {
                    return Err(AppError::AppOperationReused {
                        operation: body.operation.clone(),
                    }
                    .into());
                }
                Ok(apps
                    .app(&id)
                    .map_or_else(Vec::new, crate::apps_state::App::live_client_credentials))
            })?;
            let issuer = state
                .provider
                .as_ref()
                .map_or_else(|| state.oidc.issuer(), |provider| provider.issuer())
                .to_owned();
            let record = checkpoint::Record::new(
                request,
                body.operation.clone(),
                id.clone(),
                issuer,
                prior,
                now(),
            );
            checkpoint.save(&record)?;
            record
        }
    };
    if let Some(connected) = record.connected.clone() {
        checkpoint.index(&record)?;
        return Ok(Json(connected));
    }
    prepare_app(&state, &headers, &manifest, &body.operation).await?;
    replace_credentials(&state, &headers, &mut record, &body.operation, &checkpoint).await?;
    // Both replacement values are durable before any previous credential is revoked.
    for credential in &record.prior {
        drop(
            revoke(
                State(Arc::clone(&state)),
                headers.clone(),
                UrlPath((id.clone(), credential.clone())),
                Ok(Json(ClientCredentialRevokeBody {
                    operation: operation(&body.operation, &format!("revoke/{credential}")),
                    reason: "the app was connected again".to_owned(),
                })),
            )
            .await?,
        );
    }
    let connected = complete(&checkpoint, &mut record, &body.back, &body.state)?;
    drop(gate);
    Ok(Json(connected))
}

#[cfg(test)]
fn keep_code(digest: String, at: u64, answer: Exchanged) -> Result<(), ServerError> {
    let mut codes = codes()
        .lock()
        .map_err(|_poisoned| unavailable("the connect codes lock is poisoned"))?;
    // A spent or expired code is kept one more lifetime, so it is refused by
    // its own name rather than as unknown; after that it is forgotten.
    codes.retain(|_digest, pending| at.saturating_sub(pending.at) <= 2 * CODE_LIFETIME_MS);
    codes.insert(
        digest,
        Pending {
            at,
            answer: Some(answer),
        },
    );
    Ok(())
}

/// Take the answer `code` holds at `at`, or the refusal's name.
#[cfg(test)]
fn take(code: &str, at: u64) -> Result<Result<Exchanged, &'static str>, ServerError> {
    let mut codes = codes()
        .lock()
        .map_err(|_poisoned| unavailable("the connect codes lock is poisoned"))?;
    let Some(pending) = codes.get_mut(&sha256_hex(code)) else {
        return Ok(Err("connect_code_unknown"));
    };
    Ok(take_pending(pending, at))
}

fn take_pending(pending: &mut Pending, at: u64) -> Result<Exchanged, &'static str> {
    if at.saturating_sub(pending.at) > CODE_LIFETIME_MS {
        pending.answer = None;
        return Err("connect_code_expired");
    }
    pending.answer.take().ok_or("connect_code_used")
}

/// The refusal `name` in plain words for the person reading the app's page.
fn refusal_words(name: &str) -> &'static str {
    match name {
        "connect_code_used" => "this connection code was already used; start the connection again",
        "connect_code_expired" => {
            "this connection code is more than ten minutes old; start the connection again"
        }
        _ => "Lys does not know this connection code; start the connection again",
    }
}

async fn exchange(
    State(state): State<Arc<AppState>>,
    body: Result<Json<ExchangeBody>, JsonRejection>,
) -> Result<Response, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let checkpoint = checkpoint::Checkpoint::open(&state)?;
    let gate = checkpoint.gate().await?;
    let answer = checkpoint.take(&body.code, now())?;
    drop(gate);
    Ok(match answer {
        Ok(answer) => Json(answer).into_response(),
        Err(refusal) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"refusal": refusal, "reason": refusal_words(refusal)})),
        )
            .into_response(),
    })
}

#[cfg(test)]
#[path = "apps_connect_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "apps_connect_reconnect_tests.rs"]
mod reconnect_tests;

#[cfg(test)]
#[path = "apps_connect_manifest_tests.rs"]
mod manifest_tests;
