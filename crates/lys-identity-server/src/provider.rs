//! Lys as the `OpenID` provider every product is registered with.
//!
//! The provider answers at Lys's own origin, which is its issuer name:
//! discovery at `/.well-known/openid-configuration`, and `/oauth/authorize`,
//! `/oauth/token`, `/oauth/userinfo` and `/oauth/jwks`. A person signs in on
//! Lys's own sign-in page; the authorization then answers the product's
//! registered redirect address with a code, and the product exchanges it,
//! with its secret and its PKCE verifier, for an ID token Lys signs with
//! `EdDSA` (Ed25519) under the key the install keeps in its state folder. The
//! token's subject is the person's directory id, never a login at the
//! issuer behind Lys, so the configuration a product receives names only
//! Lys's address.
//!
//! A product is a client registered with Lys: until products register
//! themselves (DIRECTORY-048) the clients are the ones the configuration
//! lists, each with its secret's SHA-256 digest and its exact redirect
//! addresses. Refusals are named, and none sends the browser anywhere: a
//! redirect address not registered (`RedirectUnregistered`), a code used
//! twice (`CodeUsed`), a wrong PKCE verifier (`VerifierWrong`), a code past
//! its instant (`CodeExpired`). An instant is data compared on use, never a
//! wait.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::extract::rejection::FormRejection;
use axum::extract::{Form, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use lys_core::Ed25519Identity;
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::{AppState, cookie_header, hex, with_directory};
use crate::session::now;

/// How long a code lives when the configuration says nothing, in seconds:
/// the ten minutes RFC 6749 (section 4.1.2) recommends as a code's longest
/// life. An ID token and an access token have no lifetime of their own: each
/// ends when the Lys sign-in it states ends.
pub const CODE_SECONDS: u64 = 600;

/// A product registered as a client of Lys.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductClient {
    /// The product's client id.
    pub client_id: String,
    /// Lowercase hex SHA-256 of the product's client secret.
    pub secret_sha256: String,
    /// The exact addresses Lys may send the product's codes to.
    pub redirect_uris: Vec<String>,
}

/// What the provider is started with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderSettings {
    /// The file holding the Ed25519 seed ID tokens are signed with.
    pub key_file: PathBuf,
    /// The products registered as clients.
    #[serde(default)]
    pub clients: Vec<ProductClient>,
    /// How long a code lives, in seconds.
    #[serde(default = "code_seconds")]
    pub code_seconds: u64,
}

fn code_seconds() -> u64 {
    CODE_SECONDS
}

/// A code Lys answered a product with.
struct Grant {
    client_id: String,
    redirect_uri: String,
    challenge: String,
    nonce: Option<String>,
    subject: String,
    authenticated_at: u64,
    expires_at: u64,
    /// When the Lys sign-in the code states ends; its tokens end with it.
    sign_in_ends_at: u64,
    used: bool,
}

/// An access token Lys issued, for the user information route.
struct Access {
    subject: String,
    expires_at: u64,
}

/// Lys's `OpenID` provider.
pub struct OpenIdProvider {
    issuer: String,
    key: Ed25519Identity,
    kid: String,
    clients: Vec<ProductClient>,
    code_seconds: u64,
    codes: Mutex<HashMap<String, Grant>>,
    tokens: Mutex<HashMap<String, Access>>,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::ProviderUnavailable {
        reason: reason.into(),
    }
}

fn held<T>(slot: &Mutex<T>) -> MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `N` bytes from the secure random source, base64url.
fn random<const N: usize>() -> Result<String, ServerError> {
    let mut bytes = [0u8; N];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| unavailable(format!("the secure random source failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// Whether `a` and `b` are the same text, in time that does not depend on
/// where they first differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |differ, (x, y)| differ | (x ^ y))
            == 0
}

/// `text` percent-encoded for a query value.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push_str(&hex(&[byte]).to_ascii_uppercase());
        }
    }
    out
}

impl OpenIdProvider {
    /// The provider `settings` names, answering as `issuer`, Lys's origin.
    pub fn open(settings: &ProviderSettings, issuer: String) -> Result<Self, ServerError> {
        let key = Ed25519Identity::load(&settings.key_file).map_err(|error| {
            ServerError::ConfigInvalid {
                reason: format!(
                    "the provider's signing key {} cannot be read: {error}",
                    settings.key_file.display()
                ),
            }
        })?;
        let digest = Sha256::digest(key.public_key_bytes());
        let kid = hex(digest.get(..8).unwrap_or_default());
        Ok(Self {
            issuer,
            key,
            kid,
            clients: settings.clients.clone(),
            code_seconds: settings.code_seconds,
            codes: Mutex::new(HashMap::new()),
            tokens: Mutex::new(HashMap::new()),
        })
    }

    /// Lys's issuer name.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    fn client(&self, client_id: &str) -> Result<&ProductClient, ServerError> {
        self.clients
            .iter()
            .find(|client| client.client_id == client_id)
            .ok_or(ServerError::ClientUnknown)
    }

    /// The client whose id and secret a token request presents.
    fn authenticated(&self, client_id: &str, secret: &str) -> Result<&ProductClient, ServerError> {
        let client = self.client(client_id)?;
        let digest = hex(&Sha256::digest(secret.as_bytes()));
        if same(&digest, &client.secret_sha256) {
            Ok(client)
        } else {
            Err(ServerError::ClientUnknown)
        }
    }

    /// Sign `claims` as a compact JWS with the provider's key.
    fn signed(&self, claims: &Value) -> String {
        let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": self.kid });
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header.to_string()),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        let signature = URL_SAFE_NO_PAD.encode(self.key.sign(input.as_bytes()));
        format!("{input}.{signature}")
    }

    fn discovery(&self) -> Value {
        let issuer = &self.issuer;
        json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}/oauth/authorize"),
            "token_endpoint": format!("{issuer}/oauth/token"),
            "userinfo_endpoint": format!("{issuer}/oauth/userinfo"),
            "jwks_uri": format!("{issuer}/oauth/jwks"),
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["EdDSA"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"],
            "scopes_supported": ["openid"],
            "claims_supported": ["iss", "sub", "aud", "iat", "exp", "auth_time", "nonce"],
        })
    }

    fn keys(&self) -> Value {
        json!({ "keys": [{
            "kty": "OKP",
            "crv": "Ed25519",
            "use": "sig",
            "alg": "EdDSA",
            "kid": self.kid,
            "x": URL_SAFE_NO_PAD.encode(self.key.public_key_bytes()),
        }]})
    }
}

/// The provider's routes, at Lys's own origin beside the screens.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
        .route("/oauth/userinfo", get(userinfo))
        .route("/oauth/jwks", get(jwks))
        .with_state(state)
}

fn provider(state: &AppState) -> Result<&OpenIdProvider, ServerError> {
    state
        .provider
        .as_ref()
        .ok_or_else(|| unavailable("the configuration names no provider"))
}

async fn discovery(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ServerError> {
    Ok(Json(provider(&state)?.discovery()))
}

async fn jwks(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ServerError> {
    Ok(Json(provider(&state)?.keys()))
}

/// What a product sends a person to Lys with.
#[derive(Deserialize)]
struct Asked {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

fn malformed(reason: &str) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.to_owned(),
    }
}

async fn authorize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    asked: Result<Query<Asked>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ServerError> {
    let provider = provider(&state)?;
    let Query(asked) = asked.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let client = provider.client(&asked.client_id)?;
    if !client.redirect_uris.contains(&asked.redirect_uri) {
        return Err(ServerError::RedirectUnregistered);
    }
    if asked.response_type != "code" {
        return Err(malformed("a product asks for a code"));
    }
    let challenge = match (asked.code_challenge, asked.code_challenge_method.as_deref()) {
        (Some(challenge), Some("S256")) if !challenge.is_empty() => challenge,
        _ => return Err(malformed("a product signs in with a PKCE S256 challenge")),
    };
    let session = match state.sessions.session(cookie_header(&headers)) {
        Ok(session) => session,
        Err(ServerError::NotSignedIn) => {
            let back = format!("/oauth/authorize?{}", query.unwrap_or_default());
            let screen = crate::sign_in::SIGN_IN_SCREEN;
            let location = format!("{screen}?continue={}", encoded(&back));
            return Ok((StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response());
        }
        Err(error) => return Err(error),
    };
    let person = with_directory(&state, |directory| {
        Ok(directory.projection()?.person_for(session.actor.binding()))
    })?
    .ok_or(ServerError::NoPerson)?;
    let code = random::<32>()?;
    let at = now();
    {
        let mut codes = held(&provider.codes);
        codes.retain(|_, grant| grant.expires_at > at);
        codes.insert(
            code.clone(),
            Grant {
                client_id: client.client_id.clone(),
                redirect_uri: asked.redirect_uri.clone(),
                challenge,
                nonce: asked.nonce,
                subject: person.to_string(),
                authenticated_at: session.actor.provenance().authenticated_at(),
                expires_at: at.saturating_add(provider.code_seconds),
                sign_in_ends_at: session.ends_at,
                used: false,
            },
        );
    }
    let mut back = asked.redirect_uri;
    back.push(if back.contains('?') { '&' } else { '?' });
    back.push_str("code=");
    back.push_str(&encoded(&code));
    if let Some(given) = asked.state {
        back.push_str("&state=");
        back.push_str(&encoded(&given));
    }
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, back)]).into_response())
}

/// A product's token request.
#[derive(Deserialize)]
struct Exchange {
    grant_type: String,
    code: String,
    redirect_uri: String,
    code_verifier: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

/// The client id and secret a token request presents, in its Basic
/// authorization or in its form.
fn presented(headers: &HeaderMap, form: &Exchange) -> Option<(String, String)> {
    let basic = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .and_then(|encoded| STANDARD.decode(encoded.trim()).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|pair| {
            pair.split_once(':')
                .map(|(id, secret)| (id.to_owned(), secret.to_owned()))
        });
    basic.or_else(|| form.client_id.clone().zip(form.client_secret.clone()))
}

/// An OAuth error answer carrying the refusal by name.
fn oauth_refusal(error: &ServerError) -> Response {
    let code = match error {
        ServerError::ClientUnknown => "invalid_client",
        ServerError::CodeUsed
        | ServerError::CodeExpired
        | ServerError::CodeUnknown
        | ServerError::VerifierWrong
        | ServerError::RedirectUnregistered => "invalid_grant",
        ServerError::RequestMalformed { .. } => "invalid_request",
        _ => "server_error",
    };
    let body = json!({
        "error": code,
        "error_description": error.to_string(),
        "refusal": error.name(),
        "reason": error.to_string(),
    });
    (
        error.status(),
        [(header::CACHE_CONTROL, "no-store")],
        Json(body),
    )
        .into_response()
}

async fn token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    form: Result<Form<Exchange>, FormRejection>,
) -> Response {
    match exchange(&state, &headers, form) {
        Ok(answer) => (
            StatusCode::OK,
            [(header::CACHE_CONTROL, "no-store")],
            Json(answer),
        )
            .into_response(),
        Err(error) => oauth_refusal(&error),
    }
}

fn exchange(
    state: &AppState,
    headers: &HeaderMap,
    form: Result<Form<Exchange>, FormRejection>,
) -> Result<Value, ServerError> {
    let provider = provider(state)?;
    let Form(form) = form.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    if form.grant_type != "authorization_code" {
        return Err(malformed("a product exchanges an authorization code"));
    }
    let (client_id, secret) = presented(headers, &form).ok_or(ServerError::ClientUnknown)?;
    let client = provider.authenticated(&client_id, &secret)?;
    let at = now();
    let (subject, nonce, authenticated_at, expires_at) = {
        let mut codes = held(&provider.codes);
        let grant = codes.get_mut(&form.code).ok_or(ServerError::CodeUnknown)?;
        if grant.client_id != client.client_id {
            return Err(ServerError::CodeUnknown);
        }
        if grant.used {
            return Err(ServerError::CodeUsed);
        }
        if at >= grant.expires_at || at >= grant.sign_in_ends_at {
            return Err(ServerError::CodeExpired);
        }
        if grant.redirect_uri != form.redirect_uri {
            return Err(ServerError::RedirectUnregistered);
        }
        grant.used = true;
        let verified = URL_SAFE_NO_PAD.encode(Sha256::digest(form.code_verifier.as_bytes()));
        if !same(&verified, &grant.challenge) {
            return Err(ServerError::VerifierWrong);
        }
        (
            grant.subject.clone(),
            grant.nonce.clone(),
            grant.authenticated_at,
            grant.sign_in_ends_at,
        )
    };
    let mut claims = json!({
        "iss": provider.issuer,
        "sub": subject,
        "aud": client.client_id,
        "iat": at,
        "exp": expires_at,
        "auth_time": authenticated_at,
    });
    if let Some(nonce) = nonce {
        claims["nonce"] = Value::String(nonce);
    }
    let access = random::<32>()?;
    {
        let mut tokens = held(&provider.tokens);
        tokens.retain(|_, token| token.expires_at > at);
        tokens.insert(
            access.clone(),
            Access {
                subject,
                expires_at,
            },
        );
    }
    Ok(json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": expires_at.saturating_sub(at),
        "id_token": provider.signed(&claims),
    }))
}

async fn userinfo(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let provider = provider(&state)?;
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ServerError::TokenUnknown)?;
    let tokens = held(&provider.tokens);
    let access = tokens
        .get(bearer.trim())
        .filter(|access| access.expires_at > now())
        .ok_or(ServerError::TokenUnknown)?;
    Ok(Json(json!({ "sub": access.subject })))
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
