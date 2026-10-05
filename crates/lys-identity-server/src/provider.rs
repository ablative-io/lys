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
//! A product is an app approved on the Apps screen, and nothing else: the
//! provider's clients are the approved apps in the apps' record, read at each
//! request (`apps_binding::sign_in_client`), so an approval or a retirement
//! is in force at the next request with no file edited and nothing
//! restarted. Refusals are named, and none sends the browser anywhere: an
//! app not approved or retired, an id no app holds, an address the
//! registration does not list (the apps' own refusals), a code exchanged for
//! another address (`RedirectUnregistered`), a code used twice (`CodeUsed`),
//! a wrong PKCE verifier (`VerifierWrong`), a code past its instant
//! (`CodeExpired`). An instant is data compared on use, never a wait.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_core::Ed25519Identity;
use lys_identity::PersonId;
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::hex;
use crate::session::now;

#[path = "provider_tokens.rs"]
mod token_store;
use token_store::Tokens;

mod endpoints;
pub use endpoints::routes;

/// How long a code lives when the configuration says nothing, in seconds:
/// the ten minutes RFC 6749 (section 4.1.2) recommends as a code's longest
/// life. Access tokens are checked against the issuing session on use.
pub const CODE_SECONDS: u64 = 600;

/// Offline identity assertions must be renewed through a live sign-in.
const ID_TOKEN_SECONDS: u64 = 300;

/// What the provider is started with. Its clients are the approved apps,
/// read from the apps' record at each request, never from here.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderSettings {
    /// The file holding the Ed25519 seed ID tokens are signed with.
    pub key_file: PathBuf,
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
    /// The person, for the name read at the exchange when `profile` is set.
    person: PersonId,
    /// Whether the product asked for the `profile` scope and the app's
    /// sign-in settings granted it at this authorization. The name itself is
    /// kept nowhere: it is read from the directory at each issue.
    profile: bool,
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
    /// The app the token was issued to.
    app: String,
    /// Whether the token was issued for the `profile` scope. The user
    /// information route reads the name again at each request, and only
    /// while the app's sign-in settings still grant it.
    profile: bool,
    expires_at: u64,
}

/// Lys's `OpenID` provider.
pub struct OpenIdProvider {
    issuer: String,
    key: Ed25519Identity,
    kid: String,
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
            "scopes_supported": ["openid", "profile"],
            "claims_supported": ["iss", "sub", "aud", "iat", "exp", "auth_time", "nonce", "name"],
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
