//! An in-process OIDC issuer that signs ID tokens for a chosen issuer and
//! subject, so the admission tests need no live Rauthy.
//!
//! It serves discovery, its one Ed25519 key as a JWK set, an authorize
//! endpoint that answers at once for the login the test chose, and a token
//! endpoint that checks the PKCE verifier and the client credentials before it
//! signs. The ID token is built here by hand, apart from openidconnect, so the
//! service's validation is checked against a second writer of the format.
//!
//! It also serves the issuer's own server-side sign-in, the three steps Lys's
//! sign-in page is carried through: `GET /oidc/authorize`, which opens a
//! session under a cookie and names its request token in a `<template>`;
//! `POST /pow`, a proof-of-work challenge of difficulty 10; and
//! `POST /oidc/authorize`, which takes the credentials with the session, the
//! token and the answered challenge and answers 202 with the address it would
//! send a browser to. Accounts are held by email with a password; an email
//! it holds no account for signs in as the login the test chose. Failed
//! sign-ins are counted by the address in `X-Forwarded-For`, and an address
//! with [`FAILURES_BARRED`] of them is answered 429, so a test can see that
//! one person's failures bar only that person.
//!
//! A stand-in sign-in provider answers on a port of its own: its authorize
//! address refuses a client id beginning `rejected` with a provider's own
//! words, and otherwise, asked with a state, signs the person in as the
//! login the test chose and sends them back to the redirect address. The
//! issuer's own provider steps, `POST /providers/login` and
//! `POST /providers/callback`, send a person to that provider and back to
//! the public callback address the test names, which is Lys's.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use lys_core::Ed25519Identity;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The client the service is registered as.
pub const CLIENT_ID: &str = "lys-directory";
/// The client secret the service holds in its secret file.
pub const CLIENT_SECRET: &str = "contract-test-client-secret";
const KEY_ID: &str = "contract-issuer-key";
/// The cookie the issuer's sign-in session is held under.
pub const SESSION_COOKIE: &str = "IssuerSession";
/// How many failed sign-ins from one address bar that address.
pub const FAILURES_BARRED: u32 = 3;
/// The path an issuer started behind a public origin serves under and
/// names itself at, as the installed issuer does.
pub const PUBLIC_PATH: &str = "/auth/v1";
/// The difficulty of the issuer's proof-of-work challenge, in zero bits.
const DIFFICULTY: u32 = 10;

/// An account the issuer holds, by email.
#[derive(Debug, Clone)]
pub struct Account {
    /// The login it signs in as.
    pub login: Login,
    /// Its password.
    pub password: String,
    /// Whether it asks for a second factor, such as a passkey.
    pub second_factor: bool,
}

/// The login the issuer signs the next token for.
#[derive(Debug, Clone)]
pub struct Login {
    /// The subject claim.
    pub subject: String,
    /// The email claim, which the service must never admit on.
    pub email: String,
}

struct Issued {
    login: Login,
    nonce: String,
    challenge: String,
}

struct Inner {
    issuer: String,
    api: String,
    loopback: String,
    key: Ed25519Identity,
    standing: Option<Login>,
    next: Mutex<Option<Login>>,
    codes: Mutex<HashMap<String, Issued>>,
    accounts: Mutex<HashMap<String, Account>>,
    disabled: Mutex<HashMap<String, Account>>,
    sessions: Mutex<HashMap<String, String>>,
    challenges: Mutex<HashSet<String>>,
    failures: Mutex<HashMap<String, u32>>,
    forwarded: Mutex<Vec<Option<String>>>,
    serial: AtomicU64,
    provider_base: String,
    public_callback: Mutex<String>,
    upstream: Mutex<HashMap<String, PendingUpstream>>,
    upstream_codes: Mutex<HashMap<String, Login>>,
    provider_redirects: Mutex<Vec<String>>,
}

/// A provider sign-in the issuer began and has not finished.
struct PendingUpstream {
    redirect_uri: String,
    state: String,
    nonce: String,
    code_challenge: String,
    pkce_challenge: String,
    xsrf: String,
    session: String,
}

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

impl Inner {
    /// Record the forwarded address a sign-in step carried, answering it.
    fn record(&self, headers: &HeaderMap) -> std::io::Result<String> {
        let forwarded = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        held(&self.forwarded)?.push(forwarded.clone());
        Ok(forwarded.unwrap_or_default())
    }

    fn next_serial(&self) -> u64 {
        self.serial.fetch_add(1, Ordering::SeqCst)
    }

    /// Issue a code for `login`, answering it.
    fn issue(
        &self,
        login: Login,
        state: &str,
        nonce: String,
        challenge: String,
    ) -> std::io::Result<String> {
        let code = URL_SAFE_NO_PAD.encode(Sha256::digest(state.as_bytes()));
        held(&self.codes)?.insert(
            code.clone(),
            Issued {
                login,
                nonce,
                challenge,
            },
        );
        Ok(code)
    }

    /// The login the test chose for the next sign-in, or the standing one.
    fn chosen(&self) -> std::io::Result<Option<Login>> {
        Ok(held(&self.next)?.take().or_else(|| self.standing.clone()))
    }
}

/// A running issuer.
#[derive(Clone)]
pub struct FakeIssuer {
    inner: Arc<Inner>,
    serving: Arc<Mutex<crate::harness_serve::Serving>>,
}

impl FakeIssuer {
    /// Start an issuer on a local port, signing with the seed in `key_file`.
    pub async fn start(key_file: &Path) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        Self::serve(listener, (issuer, ""), key_file, None).await
    }

    /// Start an issuer on a local port that names itself on `public`, the
    /// origin a browser uses, at [`PUBLIC_PATH`], as the installed issuer
    /// does behind Lys: its name and every endpoint its discovery document
    /// gives are on `public`, and it answers only on its own loopback port,
    /// under the same path.
    pub async fn start_behind(public: &str, key_file: &Path) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("{public}{PUBLIC_PATH}");
        Self::serve(listener, (issuer, PUBLIC_PATH), key_file, None).await
    }

    /// Start an issuer listening on `bind` that names itself `issuer`, for a
    /// development service a browser signs in to. Every sign-in the caller
    /// has not chosen with [`FakeIssuer::sign_in_as`] is signed as `standing`.
    pub async fn start_at(
        bind: SocketAddr,
        issuer: String,
        key_file: &Path,
        standing: Login,
    ) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        Self::serve(listener, (issuer, ""), key_file, Some(standing)).await
    }

    /// Serve on `listener` under the name and the path `(issuer, path)`.
    async fn serve(
        listener: tokio::net::TcpListener,
        (issuer, path): (String, &str),
        key_file: &Path,
        standing: Option<Login>,
    ) -> Result<Self, Box<dyn Error>> {
        let provider = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let provider_base = format!("http://{}", provider.local_addr()?);
        let loopback = format!("http://{}", listener.local_addr()?);
        let inner = Arc::new(Inner {
            issuer,
            api: format!("{loopback}{path}"),
            loopback,
            key: Ed25519Identity::load(key_file)?,
            standing,
            next: Mutex::new(None),
            codes: Mutex::new(HashMap::new()),
            accounts: Mutex::new(HashMap::new()),
            disabled: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            challenges: Mutex::new(HashSet::new()),
            failures: Mutex::new(HashMap::new()),
            forwarded: Mutex::new(Vec::new()),
            serial: AtomicU64::new(1),
            provider_base,
            public_callback: Mutex::new(String::new()),
            upstream: Mutex::new(HashMap::new()),
            upstream_codes: Mutex::new(HashMap::new()),
            provider_redirects: Mutex::new(Vec::new()),
        });
        let provider_router = Router::new()
            .route("/authorize", get(provider_authorize))
            .route("/login/oauth/authorize", get(provider_authorize))
            .route("/{tenant}/v2.0/authorize", get(provider_authorize))
            .with_state(Arc::clone(&inner));
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .route("/authorize", get(authorize))
            .route("/token", post(token))
            .route("/oidc/authorize", get(login_page).post(credentials))
            .route("/pow", post(pow))
            .route("/providers/login", post(provider_login))
            .route("/providers/callback", post(provider_callback))
            .with_state(Arc::clone(&inner));
        let router = if path.is_empty() {
            router
        } else {
            Router::new().nest(path, router)
        };
        let serving = crate::harness_serve::Serving::start(vec![
            (provider, provider_router),
            (listener, router),
        ])
        .await?;
        Ok(Self {
            inner,
            serving: Arc::new(Mutex::new(serving)),
        })
    }

    pub(crate) fn shutdown(&self) -> std::io::Result<()> {
        held(&self.serving)?.stop()
    }

    /// The issuer URL, exactly as it signs it.
    pub fn issuer(&self) -> &str {
        &self.inner.issuer
    }

    /// Where the issuer answers: its loopback origin and the path it
    /// serves under, the address a service reaches it at.
    pub fn api(&self) -> &str {
        &self.inner.api
    }

    /// The issuer's loopback origin, which no browser is ever given.
    pub fn loopback(&self) -> &str {
        &self.inner.loopback
    }

    /// Sign the next sign-in as `login`.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn sign_in_as(&self, login: Login) -> std::io::Result<()> {
        *held(&self.inner.next)? = Some(login);
        Ok(())
    }

    /// Hold `account` under its login's email, replacing any held there.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn hold(&self, account: Account) -> std::io::Result<()> {
        held(&self.inner.accounts)?.insert(account.login.email.clone(), account);
        Ok(())
    }

    /// Follow a change the administration API made to account `subject`:
    /// its email, a new password when one was set, and whether it may sign
    /// in. An account never given a password is not held.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn follow(
        &self,
        subject: &str,
        email: &str,
        password: Option<String>,
        enabled: bool,
    ) -> std::io::Result<()> {
        let mut accounts = held(&self.inner.accounts)?;
        let mut disabled = held(&self.inner.disabled)?;
        let before = accounts
            .iter()
            .find(|(_, account)| account.login.subject == subject)
            .map(|(key, account)| (key.clone(), account.clone()));
        let before = match before {
            Some((key, account)) => {
                accounts.remove(&key);
                Some(account)
            }
            None => disabled.remove(subject),
        };
        let earlier = before.as_ref().map(|account| account.password.clone());
        let Some(password) = password.or(earlier) else {
            return Ok(());
        };
        let account = Account {
            login: Login {
                subject: subject.to_owned(),
                email: email.to_owned(),
            },
            password,
            second_factor: before.is_some_and(|account| account.second_factor),
        };
        if enabled {
            accounts.insert(email.to_owned(), account);
        } else {
            disabled.insert(subject.to_owned(), account);
        }
        Ok(())
    }

    /// The account held under `email`, if one is.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn account(&self, email: &str) -> std::io::Result<Option<Account>> {
        Ok(held(&self.inner.accounts)?.get(email).cloned())
    }

    /// The stand-in provider's origin.
    pub fn provider_base(&self) -> &str {
        &self.inner.provider_base
    }

    /// Send people back from the provider to `callback`, the issuer's
    /// public provider callback address.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn set_public_callback(&self, callback: &str) -> std::io::Result<()> {
        callback.clone_into(&mut *held(&self.inner.public_callback)?);
        Ok(())
    }

    /// Every redirect address the stand-in provider was asked with, in order.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn provider_redirects(&self) -> std::io::Result<Vec<String>> {
        Ok(held(&self.inner.provider_redirects)?.clone())
    }

    /// Every `X-Forwarded-For` value a sign-in step carried, in order, none
    /// where a step carried none.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn forwarded(&self) -> std::io::Result<Vec<Option<String>>> {
        Ok(held(&self.inner.forwarded)?.clone())
    }
}

type Shared = Arc<Inner>;

fn refused(reason: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
}

async fn discovery(State(inner): State<Shared>) -> Json<Value> {
    let base = &inner.issuer;
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "jwks_uri": format!("{base}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
        "code_challenge_methods_supported": ["S256"],
    }))
}

async fn jwks(State(inner): State<Shared>) -> Json<Value> {
    Json(json!({ "keys": [{
        "kty": "OKP",
        "crv": "Ed25519",
        "use": "sig",
        "alg": "EdDSA",
        "kid": KEY_ID,
        "x": URL_SAFE_NO_PAD.encode(inner.key.public_key_bytes()),
    }]}))
}

#[derive(Deserialize)]
struct AuthorizeRequest {
    redirect_uri: String,
    state: String,
    nonce: String,
    code_challenge: String,
    code_challenge_method: String,
}

async fn authorize(
    State(inner): State<Shared>,
    Query(request): Query<AuthorizeRequest>,
) -> Response {
    if request.code_challenge_method != "S256" {
        return refused("only S256 is accepted");
    }
    let Some(login) = fixture!(inner.chosen()) else {
        return refused("the test chose no login");
    };
    let code = fixture!(inner.issue(login, &request.state, request.nonce, request.code_challenge));
    let location = format!(
        "{}?code={code}&state={}",
        request.redirect_uri, request.state
    );
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}

#[derive(Deserialize)]
struct TokenRequest {
    code: String,
    code_verifier: String,
}

fn client_admitted(headers: &HeaderMap) -> bool {
    let expected = format!(
        "Basic {}",
        STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"))
    );
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected)
}

async fn token(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Form(request): Form<TokenRequest>,
) -> Response {
    if !client_admitted(&headers) {
        return refused("the client credentials are wrong");
    }
    let Some(issued) = fixture!(held(&inner.codes)).remove(&request.code) else {
        return refused("the code is unknown or used");
    };
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(request.code_verifier.as_bytes()));
    if challenge != issued.challenge {
        return refused("the PKCE verifier does not match");
    }
    let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return refused("the clock is before the epoch");
    };
    let now = now.as_secs();
    let claims = json!({
        "iss": inner.issuer,
        "sub": issued.login.subject,
        "email": issued.login.email,
        "email_verified": true,
        "aud": [CLIENT_ID],
        "iat": now,
        "exp": now + 300,
        "auth_time": now,
        "nonce": issued.nonce,
    });
    let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": KEY_ID });
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    );
    let signature = URL_SAFE_NO_PAD.encode(inner.key.sign(signing_input.as_bytes()));
    Json(json!({
        "access_token": URL_SAFE_NO_PAD.encode(Sha256::digest(signing_input.as_bytes())),
        "token_type": "bearer",
        "expires_in": 300,
        "id_token": format!("{signing_input}.{signature}"),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct PageRequest {
    client_id: String,
    code_challenge_method: String,
}

/// The start of the issuer's own sign-in: a session under a cookie, and its
/// request token in the page's template.
async fn login_page(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Query(request): Query<PageRequest>,
) -> Response {
    fixture!(inner.record(&headers));
    if request.client_id != CLIENT_ID || request.code_challenge_method != "S256" {
        return refused("the authorization request is not one this issuer takes");
    }
    let serial = inner.next_serial();
    let session = format!("session{serial}");
    let token = format!("token{serial}");
    fixture!(held(&inner.sessions)).insert(session.clone(), token.clone());
    let page = format!(
        "<html><body><div hidden><template id=\"tpl_csrf_token\">{token}</template></div></body></html>"
    );
    (
        StatusCode::OK,
        [
            (
                header::SET_COOKIE,
                format!("{SESSION_COOKIE}={session}; Path=/; HttpOnly"),
            ),
            (header::CONTENT_TYPE, "text/html".to_owned()),
        ],
        page,
    )
        .into_response()
}

/// A proof-of-work challenge, remembered until it is answered once.
async fn pow(State(inner): State<Shared>, headers: HeaderMap) -> Response {
    fixture!(inner.record(&headers));
    let serial = inner.next_serial();
    let challenge = format!("1:{DIFFICULTY}:4102444800:{serial:0>16}:{serial:0>43}:");
    fixture!(held(&inner.challenges)).insert(challenge.clone());
    challenge.into_response()
}

/// Whether `answer` answers a challenge this issuer gave and has not taken,
/// taking it.
fn answered(inner: &Inner, answer: &str) -> std::io::Result<bool> {
    let Some((challenge, counter)) = answer.rsplit_once(':') else {
        return Ok(false);
    };
    let hash = Sha256::digest(answer.as_bytes());
    let zeros = hash[0] == 0 && hash[1] >> (16 - DIFFICULTY) == 0;
    Ok(!counter.is_empty() && zeros && held(&inner.challenges)?.remove(&format!("{challenge}:")))
}

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: Option<String>,
    pow: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

fn session_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_owned())
}

fn status(code: StatusCode, message: &str) -> Response {
    (
        code,
        Json(json!({ "error": "Unauthorized", "message": message })),
    )
        .into_response()
}

/// The credentials, with the session, its token and the answered challenge.
async fn credentials(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(request): Json<Credentials>,
) -> Response {
    // The issuer refuses a sign-in that names no User-Agent (Rauthy v0.36.2,
    // "Empty User-Agent not allowed").
    let agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok());
    if agent.is_none_or(str::is_empty) {
        return status(StatusCode::BAD_REQUEST, "Empty User-Agent not allowed");
    }
    let address = fixture!(inner.record(&headers));
    let token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());
    let sessions = fixture!(held(&inner.sessions));
    let session = session_of(&headers).and_then(|id| sessions.get(&id).cloned());
    drop(sessions);
    if session.is_none() || session.as_deref() != token {
        return status(StatusCode::UNAUTHORIZED, "Unauthorized Session");
    }
    if !fixture!(answered(&inner, &request.pow)) {
        return status(StatusCode::BAD_REQUEST, "Invalid PoW");
    }
    if fixture!(held(&inner.failures))
        .get(&address)
        .copied()
        .unwrap_or(0)
        >= FAILURES_BARRED
    {
        return status(StatusCode::TOO_MANY_REQUESTS, "Too many failed logins");
    }
    let password = request.password.unwrap_or_default();
    let account = fixture!(held(&inner.accounts)).get(&request.email).cloned();
    let login = match account {
        Some(account) if account.password == password => {
            if account.second_factor {
                return (StatusCode::OK, Json(json!({ "code": "webauthn" }))).into_response();
            }
            Some(account.login)
        }
        Some(_) => None,
        None if password.is_empty() => None,
        None => fixture!(inner.chosen()),
    };
    let Some(login) = login else {
        *fixture!(held(&inner.failures)).entry(address).or_insert(0) += 1;
        return status(StatusCode::UNAUTHORIZED, "Invalid user credentials");
    };
    if request.client_id != CLIENT_ID || request.code_challenge_method.as_deref() != Some("S256") {
        return status(
            StatusCode::BAD_REQUEST,
            "the client or its challenge is not taken",
        );
    }
    let state = request.state.unwrap_or_default();
    let code = fixture!(inner.issue(
        login,
        &state,
        request.nonce.unwrap_or_default(),
        request.code_challenge.unwrap_or_default(),
    ));
    let location = format!("{}?code={code}&state={state}", request.redirect_uri);
    (StatusCode::ACCEPTED, [(header::LOCATION, location)]).into_response()
}

#[derive(Deserialize)]
struct ProviderAsk {
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
}

/// The stand-in provider's sign-in address.
async fn provider_authorize(
    State(inner): State<Shared>,
    Query(ask): Query<ProviderAsk>,
) -> Response {
    fixture!(held(&inner.provider_redirects)).push(ask.redirect_uri.clone());
    if ask.client_id.starts_with("rejected") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "invalid_client",
                "error_description": "The OAuth client was not found.",
            })),
        )
            .into_response();
    }
    let Some(state) = ask.state else {
        return (StatusCode::OK, "<html><title>Sign in</title></html>").into_response();
    };
    let Some(login) = fixture!(inner.chosen()) else {
        return refused("the test chose no login at the provider");
    };
    let code = format!("upstream{}", inner.next_serial());
    fixture!(held(&inner.upstream_codes)).insert(code.clone(), login);
    let location = format!("{}?code={code}&state={state}", ask.redirect_uri);
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}

#[derive(Deserialize)]
struct ProviderLogin {
    pow: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    provider_id: String,
    pkce_challenge: String,
}

/// The session a request carries, when its request token is the session's.
fn session_with_token(inner: &Inner, headers: &HeaderMap) -> std::io::Result<Option<String>> {
    let Some(token) = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };
    let Some(session) = session_of(headers) else {
        return Ok(None);
    };
    Ok(
        (held(&inner.sessions)?.get(&session).map(String::as_str) == Some(token))
            .then_some(session),
    )
}

/// The issuer's start of a provider sign-in.
async fn provider_login(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(request): Json<ProviderLogin>,
) -> Response {
    fixture!(inner.record(&headers));
    let Some(session) = fixture!(session_with_token(&inner, &headers)) else {
        return status(StatusCode::UNAUTHORIZED, "Unauthorized Session");
    };
    if !fixture!(answered(&inner, &request.pow)) {
        return status(StatusCode::BAD_REQUEST, "Invalid PoW");
    }
    if request.provider_id.is_empty() || request.provider_id.starts_with("unknown") {
        return status(StatusCode::NOT_FOUND, "no provider has that id");
    }
    if request.client_id != CLIENT_ID || request.code_challenge_method.as_deref() != Some("S256") {
        return status(
            StatusCode::BAD_REQUEST,
            "the client or its challenge is not taken",
        );
    }
    let serial = inner.next_serial();
    let callback_id = format!("callback{serial}");
    let xsrf = format!("xsrf{serial}");
    fixture!(held(&inner.upstream)).insert(
        callback_id.clone(),
        PendingUpstream {
            redirect_uri: request.redirect_uri,
            state: request.state.unwrap_or_default(),
            nonce: request.nonce.unwrap_or_default(),
            code_challenge: request.code_challenge.unwrap_or_default(),
            pkce_challenge: request.pkce_challenge,
            xsrf: xsrf.clone(),
            session,
        },
    );
    let callback = fixture!(held(&inner.public_callback)).clone();
    let Ok(mut location) = reqwest::Url::parse(&format!("{}/authorize", inner.provider_base))
    else {
        return status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the provider's address is not one",
        );
    };
    location
        .query_pairs_mut()
        .append_pair("client_id", "stand-in-client")
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &callback)
        .append_pair("state", &callback_id);
    (
        StatusCode::ACCEPTED,
        [
            (header::LOCATION, location.to_string()),
            (
                header::SET_COOKIE,
                format!("IssuerUpstream={callback_id}; Path=/"),
            ),
        ],
        xsrf,
    )
        .into_response()
}

#[derive(Deserialize)]
struct ProviderBack {
    state: String,
    code: String,
    xsrf_token: String,
    pkce_verifier: String,
}

/// The issuer's finish of a provider sign-in.
async fn provider_callback(
    State(inner): State<Shared>,
    headers: HeaderMap,
    Json(back): Json<ProviderBack>,
) -> Response {
    fixture!(inner.record(&headers));
    let Some(session) = fixture!(session_with_token(&inner, &headers)) else {
        return status(StatusCode::UNAUTHORIZED, "Unauthorized Session");
    };
    let Some(pending) = fixture!(held(&inner.upstream)).remove(&back.state) else {
        return status(
            StatusCode::BAD_REQUEST,
            "no provider sign-in has that state",
        );
    };
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains(&format!("IssuerUpstream={}", back.state)));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(back.pkce_verifier.as_bytes()));
    let matches = cookie
        && session == pending.session
        && back.xsrf_token == pending.xsrf
        && challenge == pending.pkce_challenge;
    if !matches {
        return status(
            StatusCode::BAD_REQUEST,
            "the provider sign-in does not match its start",
        );
    }
    let Some(login) = fixture!(held(&inner.upstream_codes)).remove(&back.code) else {
        return status(
            StatusCode::BAD_REQUEST,
            "the provider's code is unknown or used",
        );
    };
    let code = fixture!(inner.issue(login, &pending.state, pending.nonce, pending.code_challenge));
    let location = format!(
        "{}?code={code}&state={}",
        pending.redirect_uri, pending.state
    );
    (StatusCode::ACCEPTED, [(header::LOCATION, location)]).into_response()
}

#[cfg(test)]
mod poison_tests {
    use super::held;

    #[test]
    fn poisoned_fixture_state_fails_instead_of_returning_a_partial_value() {
        let slot = std::sync::Mutex::new(1);
        let poison = std::panic::catch_unwind(|| {
            let mut state = held(&slot).expect("fixture is initially healthy");
            *state = 2;
            panic!("injected partial fixture update");
        });
        assert!(poison.is_err());
        match held(&slot) {
            Err(error) => assert!(error.to_string().contains("fixture_lock_poisoned")),
            Ok(value) => panic!("poisoned fixture value was returned: {value}"),
        }
    }
}
