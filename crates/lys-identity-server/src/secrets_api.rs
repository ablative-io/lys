//! The secrets screens' routes. The signed-in person's session is the only
//! word for who asks: the service signs each request to the secrets broker
//! on that person's behalf with its own key, and the broker answers exactly
//! as it answers the person. Nothing of a secret's value, a handle's bearer
//! token, a key or a credential's route reaches the browser: the broker
//! sends none of them, and the upstream a secret is routed to is taken off
//! before the answer leaves.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{RawQuery, State};
use axum::http::{HeaderMap, Method};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_core::Ed25519Identity;
use serde::Deserialize;
use serde_json::Value;

use crate::error::ServerError;
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};

/// Where the secrets broker is and what this service signs as.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretsSettings {
    /// The broker's base URL, on the loopback.
    pub broker: String,
    /// The name the broker trusts this service by.
    pub service: String,
    /// The file holding this service's Ed25519 key, mode 0600.
    pub service_key_file: PathBuf,
}

/// The broker this service asks, and the key it asks with.
pub struct SecretsBroker {
    base: String,
    service: String,
    key: Ed25519Identity,
    client: reqwest::Client,
}

impl SecretsBroker {
    /// The broker `settings` names, with the service's key read from its
    /// file.
    ///
    /// # Errors
    ///
    /// `ConfigInvalid` when the key file cannot be read.
    pub fn open(settings: &SecretsSettings) -> Result<Self, ServerError> {
        let key = Ed25519Identity::load(&settings.service_key_file).map_err(|error| {
            ServerError::ConfigInvalid {
                reason: format!(
                    "the secrets service key {} cannot be read: {error}",
                    settings.service_key_file.display()
                ),
            }
        })?;
        Ok(Self {
            base: settings.broker.trim_end_matches('/').to_owned(),
            service: settings.service.clone(),
            key,
            client: reqwest::Client::new(),
        })
    }
}

/// The secrets routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/secrets", get(secrets))
        .route("/secrets/grants", get(grants))
        .route("/secrets/audit", get(audit))
        .route("/secrets/revocation", get(revocation))
        .route("/secrets/scope", post(scope))
        .route("/secrets/recipients", post(recipients))
}

/// The person the session speaks for, as the broker names people.
fn person(state: &AppState, headers: &HeaderMap) -> Result<String, ServerError> {
    let actor = signed_in(state, headers)?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        Ok(own_person(projection, &actor)?.to_string())
    })
}

/// Asks the broker `path` on behalf of the session's person, with `body`.
async fn ask(
    state: &AppState,
    headers: &HeaderMap,
    method: Method,
    path: &str,
    body: Bytes,
) -> Result<Value, ServerError> {
    let broker = state
        .secrets
        .as_ref()
        .ok_or_else(|| ServerError::SecretsUnavailable {
            reason: "no secrets broker is configured for this service".to_owned(),
        })?;
    let person = person(state, headers)?;
    let [service, on_behalf_of, operation, signed_at, signature] = crate::secrets_sign::headers(
        &crate::secrets_sign::Asked {
            service: &broker.service,
            person: &person,
            method: method.as_str(),
            path,
            body: &body,
            signed_at_ms: now_ms()?,
        },
        &broker.key,
    )?;
    let answer = broker
        .client
        .request(method, format!("{}{path}", broker.base))
        .header("lys-service", service)
        .header("lys-on-behalf-of", on_behalf_of)
        .header("lys-operation", operation)
        .header("lys-signed-at", signed_at)
        .header("lys-service-signature", signature)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|error| ServerError::SecretsUnavailable {
            reason: format!("the secrets broker could not be reached: {error}"),
        })?;
    let status = answer.status();
    let text = answer
        .text()
        .await
        .map_err(|error| ServerError::SecretsUnavailable {
            reason: format!("the secrets broker's answer could not be read: {error}"),
        })?;
    if !status.is_success() {
        let text = text.trim();
        let (refusal, reason) = text.split_once(": ").unwrap_or(("SecretsRefused", text));
        return Err(ServerError::SecretsRefused {
            status,
            refusal: refusal.to_owned(),
            reason: reason.to_owned(),
        });
    }
    serde_json::from_str(&text).map_err(|error| ServerError::SecretsUnavailable {
        reason: format!("the secrets broker's answer is not JSON: {error}"),
    })
}

async fn secrets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let mut answer = ask(&state, &headers, Method::GET, "/_lys/secrets", Bytes::new()).await?;
    if let Some(entries) = answer.get_mut("secrets").and_then(Value::as_array_mut) {
        for entry in entries {
            if let Some(fields) = entry.as_object_mut() {
                fields.remove("upstream");
                fields.remove("header");
            }
        }
    }
    Ok(Json(answer))
}

async fn grants(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let answer = ask(&state, &headers, Method::GET, "/_lys/grants", Bytes::new()).await?;
    Ok(Json(answer))
}

async fn audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let answer = ask(&state, &headers, Method::GET, "/_lys/audit", Bytes::new()).await?;
    Ok(Json(answer))
}

async fn revocation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Result<Json<Value>, ServerError> {
    let query = query.ok_or_else(|| ServerError::RequestMalformed {
        reason: "name the handle as ?handle=<id>".to_owned(),
    })?;
    let path = format!("/_lys/revocation?{query}");
    let answer = ask(&state, &headers, Method::GET, &path, Bytes::new()).await?;
    Ok(Json(answer))
}

async fn scope(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    let answer = ask(&state, &headers, Method::POST, "/_lys/scope", body).await?;
    Ok(Json(answer))
}

async fn recipients(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    let answer = ask(&state, &headers, Method::POST, "/_lys/recipients", body).await?;
    Ok(Json(answer))
}

/// The time now, in milliseconds since the epoch, as the broker reads a
/// signing time.
fn now_ms() -> Result<i64, ServerError> {
    let unreadable = |reason: String| ServerError::SecretsUnavailable { reason };
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| unreadable(format!("the clock reads before the epoch: {error}")))?;
    i64::try_from(since.as_millis())
        .map_err(|_overflow| unreadable("the clock is past what a signing time holds".to_owned()))
}
