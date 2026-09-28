//! An in-process stand-in for the issuer's administration API, so the
//! sign-in provider tests need no live Rauthy.
//!
//! It serves the provider lookup, the provider list, creation and
//! replacement, admitting only the API key it was started with, and keeps
//! every provider it was given so a test reads back what the service sent,
//! secret included.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

/// The API key the fake admits, `name$secret` as the issuer writes it.
pub const API_KEY: &str = "lys$contract-test-api-key";

struct Inner {
    api: String,
    providers: Mutex<Vec<Value>>,
    /// Holds the API key file, as the installation writes it, for the
    /// stand-in's life.
    keys: tempfile::TempDir,
}

/// A running stand-in.
#[derive(Clone)]
pub struct FakeRauthy {
    inner: Arc<Inner>,
}

type Shared = Arc<Inner>;

impl FakeRauthy {
    /// Start the stand-in on a local port.
    pub async fn start() -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let api = format!("http://{}/auth/v1", listener.local_addr()?);
        let keys = tempfile::TempDir::new()?;
        let key_file = keys.path().join("sign-in-providers.key");
        std::fs::write(&key_file, format!("{API_KEY}\n"))?;
        std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600))?;
        let inner = Arc::new(Inner {
            api,
            providers: Mutex::new(Vec::new()),
            keys,
        });
        let router = Router::new()
            .route("/auth/v1/providers/lookup", post(lookup))
            .route("/auth/v1/providers", get(list).post(create))
            .route("/auth/v1/providers/{id}", axum::routing::put(replace))
            .with_state(Arc::clone(&inner));
        tokio::spawn(async move { axum::serve(listener, router).await });
        Ok(Self { inner })
    }

    /// The API base the service is configured with.
    pub fn api(&self) -> &str {
        &self.inner.api
    }

    /// The API key file, `name$secret`, mode 0600, as the installation
    /// writes it for the service.
    pub fn api_key_file(&self) -> PathBuf {
        self.inner.keys.path().join("sign-in-providers.key")
    }

    /// Every provider held, as the service sent it, with its id.
    pub fn providers(&self) -> Vec<Value> {
        self.inner
            .providers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The refusal for a request without the API key, none when it carries it.
fn refused(headers: &HeaderMap) -> Option<Response> {
    let expected = format!("API-Key {API_KEY}");
    if headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected)
    {
        None
    } else {
        Some(
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "message": "the API key is missing or wrong" })),
            )
                .into_response(),
        )
    }
}

async fn lookup(State(inner): State<Shared>, headers: HeaderMap, body: Json<Value>) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let Some(issuer) = body.get("issuer").and_then(Value::as_str) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "the lookup names an issuer" })),
        )
            .into_response();
    };
    debug_assert!(!inner.api.is_empty());
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "userinfo_endpoint": format!("{issuer}/userinfo"),
        "jwks_endpoint": format!("{issuer}/jwks"),
        "scope": "openid email profile",
        "use_pkce": true,
        "client_secret_basic": true,
        "client_secret_post": false,
    }))
    .into_response()
}

async fn list(State(inner): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let held = inner
        .providers
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    Json(Value::Array(held)).into_response()
}

async fn create(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(mut body): Json<Value>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let mut held = inner
        .providers
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let id = format!("provider-{}", held.len() + 1);
    if let Value::Object(fields) = &mut body {
        fields.insert("id".to_owned(), Value::String(id));
    }
    held.push(body.clone());
    Json(body).into_response()
}

async fn replace(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut body): Json<Value>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let mut held = inner
        .providers
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some(slot) = held
        .iter_mut()
        .find(|provider| provider.get("id").and_then(Value::as_str) == Some(id.as_str()))
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "message": "no provider has that id" })),
        )
            .into_response();
    };
    if let Value::Object(fields) = &mut body {
        fields.insert("id".to_owned(), Value::String(id));
    }
    *slot = body.clone();
    Json(body).into_response()
}
