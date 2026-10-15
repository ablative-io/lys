//! Connect an app in one act (the app-manifest pattern): the administrator
//! approves the manifest the browser fetched from the app, and Lys registers,
//! approves and issues the app's credentials with the existing acts, keeping
//! them behind a one-time code the app's server exchanges.
//!
//! Lys still calls no app: the manifest arrives in the administrator's body
//! and the return address is only answered, never fetched.
//!
//! The code is 256 bits, lives ten minutes, is used once and is kept only as
//! its SHA-256, bound to its app. Pending codes are held in this process's
//! memory: a restart forgets them, and the app starts its connection again.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};

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

/// The connect routes, merged into the app routes.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps/connect", post(connect))
        .route("/apps/connect/exchange", post(exchange))
}

/// What an app publishes at `{app}/identity/lys/manifest`.
#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct Manifest {
    id: String,
    name: String,
    redirects: Vec<String>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "named by the manifest; Lys keeps no logout addresses yet"
    )]
    post_logout_redirects: Vec<String>,
    #[serde(default)]
    profile: bool,
    #[schema(value_type = Object)]
    schema: Value,
}

/// The administrator's approval of a fetched manifest.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConnectBody {
    operation: String,
    manifest: Manifest,
    #[serde(rename = "return")]
    back: String,
    state: String,
}

/// Where the approval page sends the browser.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct Connected {
    redirect: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExchangeBody {
    code: String,
}

/// What the app's server collects, once.
#[derive(Clone, Serialize, utoipa::ToSchema)]
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

fn operation() -> Result<String, ServerError> {
    Ok(OperationId::generate()?.to_string())
}

async fn connect(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ConnectBody>, JsonRejection>,
) -> Result<Json<Connected>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    OperationId::from_str(&body.operation)?;
    let manifest = body.manifest;
    if body.state.is_empty()
        || body
            .state
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(malformed("a connection carries the app's state, one word"));
    }
    return_admitted(&body.back, &manifest.redirects)?;
    // Lys's own provider when configured; without it the app's sign-in
    // client lives at the issuer Lys itself signs in through.
    let issuer = state
        .provider
        .as_ref()
        .map_or_else(|| state.oidc.issuer(), |provider| provider.issuer())
        .to_owned();
    let id = manifest.id.clone();
    if id == lys_identity::grants::LYS_APP {
        return Err(AppError::AppIsLys {
            reason: "the app lys is Lys itself and is never connected from a manifest",
        }
        .into());
    }
    let standing = with_apps(&state, |apps, _| {
        Ok(apps.app(&id).map(crate::apps_state::App::standing))
    })?;
    let live = match standing {
        None => {
            let Json(_registered) = register(
                State(Arc::clone(&state)),
                headers.clone(),
                Ok(Json(RegisterBody {
                    operation: operation()?,
                    id: id.clone(),
                    name: manifest.name.clone(),
                    redirects: manifest.redirects.clone(),
                    schema: manifest.schema.clone(),
                    service_account: None,
                })),
            )
            .await?;
            Vec::new()
        }
        Some(Standing::Pending) => Vec::new(),
        Some(Standing::Approved) => with_apps(&state, |apps, _| {
            Ok(apps
                .app(&id)
                .map(crate::apps_state::App::live_client_credentials)
                .unwrap_or_default())
        })?,
        Some(Standing::Declined) => return Err(AppError::AppDecided { app: id }.into()),
        Some(Standing::Retired) => return Err(AppError::AppRetired { app: id }.into()),
    };
    if standing != Some(Standing::Approved) {
        let Json(_approved) = approve(
            State(Arc::clone(&state)),
            headers.clone(),
            UrlPath(id.clone()),
            Ok(Json(AppApprovalBody {
                operation: operation()?,
                redirects: manifest.redirects.clone(),
                profile: manifest.profile,
            })),
        )
        .await?;
    }
    if standing == Some(Standing::Approved) {
        // Reconnecting: the sign-in settings follow the manifest as it is now.
        let Json(_signed_in) = crate::apps_sign_in::set(
            State(Arc::clone(&state)),
            headers.clone(),
            UrlPath(id.clone()),
            Ok(Json(crate::apps_sign_in::AppSignInBody {
                operation: operation()?,
                redirects: manifest.redirects.clone(),
                profile: manifest.profile,
            })),
        )
        .await?;
    }
    // Reconnecting an approved app: its old client credentials end, and the
    // bearer issued below supersedes the old one.
    for credential in live {
        let Json(_revoked) = revoke(
            State(Arc::clone(&state)),
            headers.clone(),
            UrlPath((id.clone(), credential)),
            Ok(Json(ClientCredentialRevokeBody {
                operation: operation()?,
                reason: "the app was connected again".to_owned(),
            })),
        )
        .await?;
    }
    let Json(client) = issue(
        State(Arc::clone(&state)),
        headers.clone(),
        UrlPath(id.clone()),
        Ok(Json(ClientCredentialIssueBody {
            operation: operation()?,
        })),
    )
    .await?;
    let Json(bearer) = issue_bearer(
        State(Arc::clone(&state)),
        headers,
        UrlPath(id.clone()),
        Ok(Json(ClientCredentialIssueBody {
            operation: operation()?,
        })),
    )
    .await?;
    let (Some(client_secret), Some(workspace_credential)) = (client.credential, bearer.credential)
    else {
        return Err(unavailable(
            "a fresh credential was not answered by its issue",
        ));
    };
    let (code, digest) = new_secret()?;
    let answer = Exchanged {
        app: id.clone(),
        issuer,
        client_id: id,
        client_secret,
        workspace_credential,
    };
    keep_code(digest, now(), answer)?;
    let mut back = reqwest::Url::parse(&body.back).map_err(|_unparsed| malformed("return"))?;
    back.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &body.state);
    Ok(Json(Connected {
        redirect: back.to_string(),
    }))
}

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
    body: Result<Json<ExchangeBody>, JsonRejection>,
) -> Result<Response, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    Ok(match take(&body.code, now())? {
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
