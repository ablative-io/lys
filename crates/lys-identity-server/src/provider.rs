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
use std::sync::{Arc, Mutex, MutexGuard};

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
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::{AppState, cookie_header, hex, with_directory};
use crate::session::now;

#[path = "provider_tokens.rs"]
mod token_store;
use token_store::Tokens;

mod endpoints;
pub use endpoints::routes;
use endpoints::*;

/// How long a code lives when the configuration says nothing, in seconds:
/// the ten minutes RFC 6749 (section 4.1.2) recommends as a code's longest
/// life. Access tokens are checked against the issuing session on use.
pub const CODE_SECONDS: u64 = 600;

/// Offline identity assertions must be renewed through a live sign-in.
const ID_TOKEN_SECONDS: u64 = 300;

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
    session_id: String,
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
    issued_access: Option<String>,
}

/// An access token Lys issued, for the user information route.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Access {
    session_id: String,
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
    tokens: Mutex<Tokens>,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::ProviderUnavailable {
        reason: reason.into(),
    }
}

fn held<T>(slot: &Mutex<T>) -> Result<MutexGuard<'_, T>, ServerError> {
    slot.lock()
        .map_err(|error| unavailable(format!("the provider state lock is poisoned: {error}")))
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
    /// Revoke the person's codes and durably remove their access tokens.
    pub(crate) fn revoke_person(&self, subject: &str) -> Result<(), ServerError> {
        let mut codes = held(&self.codes)?;
        codes.retain(|_, grant| grant.subject != subject);
        held(&self.tokens)?.revoke_subject(subject)
    }

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
            tokens: Mutex::new(Tokens::open(
                settings.key_file.with_extension("tokens.json"),
                now(),
            )?),
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

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
