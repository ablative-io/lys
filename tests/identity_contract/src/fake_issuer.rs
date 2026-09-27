//! An in-process OIDC issuer that signs ID tokens for a chosen issuer and
//! subject, so the admission tests need no live Rauthy.
//!
//! It serves discovery, its one Ed25519 key as a JWK set, an authorize
//! endpoint that answers at once for the login the test chose, and a token
//! endpoint that checks the PKCE verifier and the client credentials before it
//! signs. The ID token is built here by hand, apart from openidconnect, so the
//! service's validation is checked against a second writer of the format.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

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
    key: Ed25519Identity,
    next: Mutex<Option<Login>>,
    codes: Mutex<HashMap<String, Issued>>,
}

/// A running issuer.
#[derive(Clone)]
pub struct FakeIssuer {
    inner: Arc<Inner>,
}

impl FakeIssuer {
    /// Start an issuer on a local port, signing with the seed in `key_file`.
    pub async fn start(key_file: &Path) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        let inner = Arc::new(Inner {
            issuer,
            key: Ed25519Identity::load(key_file)?,
            next: Mutex::new(None),
            codes: Mutex::new(HashMap::new()),
        });
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .route("/authorize", get(authorize))
            .route("/token", post(token))
            .with_state(Arc::clone(&inner));
        tokio::spawn(async move { axum::serve(listener, router).await });
        Ok(Self { inner })
    }

    /// The issuer URL, exactly as it signs it.
    pub fn issuer(&self) -> &str {
        &self.inner.issuer
    }

    /// Sign the next sign-in as `login`.
    pub fn sign_in_as(&self, login: Login) {
        *self
            .inner
            .next
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(login);
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
    let Some(login) = inner
        .next
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
    else {
        return refused("the test chose no login");
    };
    let code = URL_SAFE_NO_PAD.encode(Sha256::digest(request.state.as_bytes()));
    inner
        .codes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(
            code.clone(),
            Issued {
                login,
                nonce: request.nonce,
                challenge: request.code_challenge,
            },
        );
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
    let Some(issued) = inner
        .codes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&request.code)
    else {
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
