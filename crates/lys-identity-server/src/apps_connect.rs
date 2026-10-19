//! Connect an app in one act (the app-manifest pattern): the administrator
//! approves the manifest Lys read from the app, and Lys registers, approves
//! and issues the app's client credential with the existing acts, keeping
//! it behind a one-time code the app's server exchanges.
//!
//! Connecting an approved app again ends nothing the running app holds: the
//! approval issues a new client credential beside the live ones and keeps
//! where the app connected from (`connect_origin`); the exchange that hands
//! the new values over issues the app's new workspace credential and then
//! revokes every client credential live at the approval, with the reason
//! "the app was connected again" (DIRECTORY-102 R1). A reconnect the app
//! never finishes leaves the running app signed in. A newer connect before
//! the exchange replaces the waiting one, whose code is then refused
//! `connect_code_replaced`.
//!
//! Lys reads an app only for the administrator, at the manifest address the
//! approve page names (`apps_connect_manifest`); the return address is only
//! answered, never fetched.
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
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{AppApprovalBody, RegisterBody, approve, malformed, redirect, register, with_apps};
use crate::apps_binding::{new_secret, sha256_hex};
use crate::apps_client_credentials::{ClientCredentialIssueBody, issue};
use crate::apps_error::AppError;
use crate::apps_state::{ConnectedFrom, Line, Standing};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// How long a connect code is exchangeable, in milliseconds.
const CODE_LIFETIME_MS: u64 = 10 * 60 * 1000;

#[path = "apps_connect_checkpoint.rs"]
mod checkpoint;

#[path = "apps_connect_exchange.rs"]
mod exchange;

#[path = "apps_connect_manifest.rs"]
mod manifest;
pub(crate) use manifest::ManifestRead;

/// The connect routes, merged into the app routes.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps/connect", post(connect))
        .route("/apps/connect/exchange", post(exchange::exchange))
        .route("/apps/connect/manifest", get(manifest::read))
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

/// The administrator's approval of a manifest Lys read.
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

#[cfg(test)]
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

/// Issue the app's new client credential beside the live ones, its value
/// checkpointed before anything answers. The workspace credential is issued
/// by the exchange, so the running app's stays live until then.
async fn new_client(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    record: &mut checkpoint::Record,
    operation_id: &str,
    checkpoint: &checkpoint::Checkpoint,
) -> Result<(), ServerError> {
    if record.client.is_some() {
        return Ok(());
    }
    let Json(client) = issue(
        State(Arc::clone(state)),
        headers.clone(),
        UrlPath(record.app.clone()),
        Ok(Json(ClientCredentialIssueBody {
            operation: operation_id.to_owned(),
        })),
    )
    .await?;
    let value = client.credential.ok_or_else(|| unavailable(
        "the client issue is recorded but its one-time value was not checkpointed; the old credentials remain live; start a new connect operation"))?;
    record.client = Some((client.credential_id, Some(value)));
    checkpoint.save(record)
}

/// Keep where app `id` connected from: the origin of the return address,
/// which `return_admitted` has held to an origin of the manifest's own.
fn keep_origin(
    state: &AppState,
    headers: &HeaderMap,
    parent: &str,
    id: &str,
    back: &str,
) -> Result<(), ServerError> {
    let origin = origin(back).ok_or_else(|| malformed("the return address has no origin"))?;
    let operation = operation(parent, "origin");
    with_apps(state, |apps, directory| {
        let by = super::administrator(state, apps, headers, directory)?;
        if apps.held().operation(&operation).is_some() {
            return Ok(());
        }
        apps.keep(Line::ConnectedFrom(ConnectedFrom {
            operation,
            app: id.to_owned(),
            origin,
            by,
            at: now(),
        }))
        .map(|_kept| ())
    })
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
    checkpoint.replace_waiting(record)?;
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
            // The exchange acts as this administrator: it issues the
            // workspace credential and revokes what the new values replace.
            let person = crate::secrets_api::person(&state, &headers)?;
            let record = checkpoint::Record::new(
                request,
                body.operation.clone(),
                id.clone(),
                issuer,
                prior,
                now(),
            )
            .acting_as(by, person);
            checkpoint.save(&record)?;
            record
        }
    };
    if let Some(connected) = record.connected.clone() {
        checkpoint.index(&record)?;
        checkpoint.replace_waiting(&record)?;
        return Ok(Json(connected));
    }
    prepare_app(&state, &headers, &manifest, &body.operation).await?;
    new_client(&state, &headers, &mut record, &body.operation, &checkpoint).await?;
    keep_origin(&state, &headers, &body.operation, &id, &body.back)?;
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
    if at.saturating_sub(pending.at) > CODE_LIFETIME_MS {
        pending.answer = None;
        return Ok(Err("connect_code_expired"));
    }
    Ok(pending.answer.take().ok_or("connect_code_used"))
}

#[cfg(test)]
#[path = "apps_connect_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "apps_connect_reconnect_tests.rs"]
mod reconnect_tests;
