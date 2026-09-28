//! An in-process stand-in for the issuer's administration API, so the
//! sign-in provider tests need no live Rauthy.
//!
//! It serves the provider lookup, the provider list, creation and
//! replacement, and the user list and user creation, admitting only the API
//! key it was started with. It keeps every provider and every user it was
//! given, so a test reads back what the service sent, secret included, and
//! counts every request that reached it, answered or not.
//!
//! It reads an account by id or by email and replaces one whole, as the
//! issuer's own update does. When it is linked to a fake issuer, every
//! change is handed to that issuer, so the account then signs in there with
//! its email and password as changed, or not at all once disabled; the
//! password is never kept here.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::fake_issuer::FakeIssuer;

/// The API key the fake admits, `name$secret` as the issuer writes it.
pub const API_KEY: &str = "lys$contract-test-api-key";

struct Inner {
    api: String,
    providers: Mutex<Vec<Value>>,
    users: Mutex<Vec<Value>>,
    requests: AtomicUsize,
    issuer: Mutex<Option<FakeIssuer>>,
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
            users: Mutex::new(Vec::new()),
            requests: AtomicUsize::new(0),
            issuer: Mutex::new(None),
            keys,
        });
        let router = Router::new()
            .route("/auth/v1/providers/lookup", post(lookup))
            .route("/auth/v1/providers", post(list))
            .route("/auth/v1/providers/create", post(create))
            .route("/auth/v1/providers/{id}", axum::routing::put(replace))
            .route("/auth/v1/users", get(users).post(create_user))
            .route("/auth/v1/users/{id}", get(read_user).put(update_user))
            .route("/auth/v1/users/email/{email}", get(user_by_email))
            .layer(from_fn_with_state(Arc::clone(&inner), counted))
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

    /// How many users the stand-in holds.
    pub fn user_count(&self) -> usize {
        self.inner
            .users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Hand every password set on an account from now on to `issuer`.
    pub fn link(&self, issuer: FakeIssuer) {
        *self
            .inner
            .issuer
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(issuer);
    }

    /// Every account held, as the service last wrote it, never with a password.
    pub fn users(&self) -> Vec<Value> {
        self.inner
            .users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// How many requests reached the stand-in, on any route, answered or refused.
    pub fn request_count(&self) -> usize {
        self.inner.requests.load(Ordering::SeqCst)
    }
}

/// Count every request before it is routed, so one to a route the stand-in
/// does not serve is counted too.
async fn counted(State(inner): State<Shared>, request: Request, next: Next) -> Response {
    inner.requests.fetch_add(1, Ordering::SeqCst);
    next.run(request).await
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

async fn users(State(inner): State<Shared>, headers: HeaderMap) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let held = inner
        .users
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    Json(Value::Array(held)).into_response()
}

async fn create_user(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(mut body): Json<Value>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let mut held = inner.users.lock().unwrap_or_else(PoisonError::into_inner);
    let id = format!("user-{}", held.len() + 1);
    if let Value::Object(fields) = &mut body {
        fields.insert("id".to_owned(), Value::String(id));
    }
    held.push(body.clone());
    Json(body).into_response()
}

fn no_user() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "message": "no user has that id" })),
    )
        .into_response()
}

fn held_user(inner: &Inner, found: impl Fn(&Value) -> bool) -> Option<Value> {
    inner
        .users
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|user| found(user))
        .cloned()
}

async fn read_user(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    held_user(&inner, |user| user["id"] == id.as_str())
        .map_or_else(no_user, |user| Json(user).into_response())
}

async fn user_by_email(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    held_user(&inner, |user| user["email"] == email.as_str())
        .map_or_else(no_user, |user| Json(user).into_response())
}

async fn update_user(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut body): Json<Value>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    let password = match &mut body {
        Value::Object(fields) => {
            fields.insert("id".to_owned(), Value::String(id.clone()));
            fields.remove("password")
        }
        _ => None,
    };
    let email = body["email"].as_str().unwrap_or_default().to_owned();
    let linked = inner
        .issuer
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    // The issuer refuses a password set again while it is one of the last
    // used (Rauthy v0.36.2's password policy, not_recently_used).
    let reused = linked.as_ref().is_some_and(|issuer| {
        issuer.account(&email).is_some_and(|account| {
            account.login.subject == id
                && password.as_ref().and_then(Value::as_str) == Some(account.password.as_str())
        })
    });
    if reused {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "BadRequest",
                "message": "The new password must not be one of the last 3 used passwords",
            })),
        )
            .into_response();
    }
    {
        let mut held = inner.users.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(slot) = held.iter_mut().find(|user| user["id"] == id.as_str()) else {
            return no_user();
        };
        *slot = body.clone();
    }
    let password = match password {
        Some(Value::String(password)) => Some(password),
        _ => None,
    };
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    if let Some(issuer) = linked {
        issuer.follow(&id, &email, password, enabled);
    }
    Json(body).into_response()
}
