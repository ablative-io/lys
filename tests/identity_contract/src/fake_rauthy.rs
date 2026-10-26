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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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
    /// The highest account number issued: a real issuer never gives a
    /// forgotten account's id to a new one.
    issued: AtomicUsize,
    /// Whether the next account creation is refused, once, as a service
    /// that will not take the request does.
    refuse_create: AtomicBool,
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

fn held<T>(slot: &Mutex<T>) -> std::io::Result<std::sync::MutexGuard<'_, T>> {
    slot.lock()
        .map_err(|error| std::io::Error::other(format!("fixture_lock_poisoned: {error}")))
}

macro_rules! fixture {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "fixture_lock_poisoned", "message": error.to_string() })),
            ).into_response(),
        }
    };
}

impl FakeRauthy {
    /// Start the stand-in on a local port.
    ///
    /// # Errors
    /// Propagates fixture directory, listener or server-start failures.
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
            issued: AtomicUsize::new(0),
            refuse_create: AtomicBool::new(false),
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

    /// A service with the bound account a provider round trip will open.
    /// Existing sign-in tests keep their subject and all browser assertions.
    ///
    /// # Errors
    /// Propagates fixture, service-start and account-store failures.
    pub async fn service_for(
        login: &crate::fake_issuer::Login,
    ) -> Result<(crate::harness::Service, Self), Box<dyn Error>> {
        let api = Self::start().await?;
        held(&api.inner.users)?.push(json!({
            "id": login.subject, "email": login.email, "email_verified": true,
            "enabled": true, "language": "en", "roles": [], "user_values": {},
            "account_type": "federated",
        }));
        let settings = lys_identity_server::sign_in_providers::SignInProvidersSettings {
            api: api.api().to_owned(),
            api_key_file: api.api_key_file(),
        };
        let (service, ()) = crate::harness::Service::start_setting(
            crate::harness::GRANT_MODEL,
            None,
            None,
            Some(settings),
            |config| {
                lys_identity_server::dev_seed::seed_configured(
                    config,
                    [login.subject.as_str(), "provider-fixture-other"],
                )?;
                Ok(())
            },
        )
        .await?;
        Ok((service, api))
    }

    /// The API base the service is configured with.
    #[must_use]
    pub fn api(&self) -> &str {
        &self.inner.api
    }

    /// The API key file, `name$secret`, mode 0600, as the installation
    /// writes it for the service.
    #[must_use]
    pub fn api_key_file(&self) -> PathBuf {
        self.inner.keys.path().join("sign-in-providers.key")
    }

    /// Every provider held, as the service sent it, with its id.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn providers(&self) -> std::io::Result<Vec<Value>> {
        Ok(held(&self.inner.providers)?.clone())
    }

    /// How many users the stand-in holds.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn user_count(&self) -> std::io::Result<usize> {
        Ok(held(&self.inner.users)?.len())
    }

    /// Hand every password set on an account from now on to `issuer`.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn link(&self, issuer: FakeIssuer) -> std::io::Result<()> {
        *held(&self.inner.issuer)? = Some(issuer);
        Ok(())
    }

    /// Every account held, as the service last wrote it, never with a password.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn users(&self) -> std::io::Result<Vec<Value>> {
        Ok(held(&self.inner.users)?.clone())
    }

    /// Drop the account `id`, as an issuer wiped or cleaned behind Lys's
    /// back does: reads of it answer 404 from now on.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn forget(&self, id: &str) -> std::io::Result<()> {
        held(&self.inner.users)?.retain(|user| user["id"] != id);
        Ok(())
    }

    /// Hold `user` as an account the issuer already has, under its own id.
    ///
    /// # Errors
    /// Names a poisoned account store.
    pub fn remember(&self, user: Value) -> std::io::Result<()> {
        held(&self.inner.users)?.push(user);
        Ok(())
    }

    /// Refuse the next POST /users once, answering 503, so a caller's
    /// failed account step can be exercised and retried.
    pub fn refuse_create_once(&self) {
        self.inner.refuse_create.store(true, Ordering::SeqCst);
    }

    /// How many requests reached the stand-in, on any route, answered or refused.
    #[must_use]
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
    let held = fixture!(held(&inner.providers)).clone();
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
    let mut held = fixture!(held(&inner.providers));
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
    let mut held = fixture!(held(&inner.providers));
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
    let held = fixture!(held(&inner.users)).clone();
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
    if inner.refuse_create.swap(false, Ordering::SeqCst) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "message": "the stand-in refuses this creation once" })),
        )
            .into_response();
    }
    let mut held = fixture!(held(&inner.users));
    let number = held.len().max(inner.issued.load(Ordering::SeqCst)) + 1;
    inner.issued.store(number, Ordering::SeqCst);
    let id = format!("user-{number}");
    if let Value::Object(fields) = &mut body {
        fields.insert("id".to_owned(), Value::String(id));
        fields.entry("language").or_insert_with(|| json!("en"));
        fields.entry("roles").or_insert_with(|| json!([]));
        fields.entry("enabled").or_insert_with(|| json!(true));
        fields
            .entry("email_verified")
            .or_insert_with(|| json!(false));
        fields.entry("user_values").or_insert_with(|| json!({}));
        // The issuer's account type (Rauthy's `User::account_type`): no
        // account is made with a password, so a new one is federated or new.
        let federated = fields
            .get("federation_uid")
            .is_some_and(|uid| !uid.is_null());
        fields.insert(
            "account_type".to_owned(),
            json!(if federated { "federated" } else { "new" }),
        );
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

fn held_user(inner: &Inner, found: impl Fn(&Value) -> bool) -> std::io::Result<Option<Value>> {
    Ok(held(&inner.users)?.iter().find(|user| found(user)).cloned())
}

async fn read_user(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Some(refusal) = refused(&headers) {
        return refusal;
    }
    fixture!(held_user(&inner, |user| user["id"] == id.as_str()))
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
    fixture!(held_user(&inner, |user| user["email"] == email.as_str()))
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
    let linked = fixture!(held(&inner.issuer)).clone();
    // The issuer refuses a password set again while it is one of the last
    // used (Rauthy v0.36.2's password policy, not_recently_used).
    let account = match linked.as_ref() {
        Some(issuer) => fixture!(issuer.account(&email)),
        None => None,
    };
    let reused = account.is_some_and(|account| {
        account.login.subject == id
            && password.as_ref().and_then(Value::as_str) == Some(account.password.as_str())
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
        let mut held = fixture!(held(&inner.users));
        let Some(slot) = held.iter_mut().find(|user| user["id"] == id.as_str()) else {
            return no_user();
        };
        // The account type follows the password as Rauthy's does: a password
        // set makes it a password account, federated or not; none keeps it.
        let federated = slot.get("federation_uid").is_some_and(|uid| !uid.is_null());
        body["account_type"] = match (&password, slot.get("account_type")) {
            (Some(Value::String(_)), _) if federated => json!("federated_password"),
            (Some(Value::String(_)), _) => json!("password"),
            (_, Some(kept)) => kept.clone(),
            (_, None) => json!(if federated { "federated" } else { "new" }),
        };
        // The issuer's account update cannot unlink federation or passkeys;
        // those have separate endpoints and are retained by User::update.
        for field in [
            "auth_provider_id",
            "federation_uid",
            "webauthn_user_id",
            "picture_id",
        ] {
            if let Some(value) = slot.get(field) {
                body[field] = value.clone();
            }
        }
        *slot = body.clone();
    }
    let password = match password {
        Some(Value::String(password)) => Some(password),
        _ => None,
    };
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    if let Some(issuer) = linked {
        fixture!(issuer.follow(&id, &email, password, enabled));
    }
    Json(body).into_response()
}
