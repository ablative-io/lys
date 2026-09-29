//! Sign-in against the configured issuer, and validation of the returned
//! token to an issuer, a subject and an authentication time.
//!
//! Every token check goes through openidconnect: discovery, the code exchange
//! with PKCE S256, the nonce, and the ID token's signature and claims. The
//! state of a sign-in in flight is kept here and used once.

use crate::sign_in_flights::Flights;
use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreJsonWebKeySet, CoreJwsSigningAlgorithm,
    CoreProviderMetadata,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, TokenResponse, reqwest,
};

use crate::config::Config;
use crate::error::ServerError;

/// The most sign-ins held in flight at once.
pub const IN_FLIGHT_MAX: usize = 1024;

fn failed(reason: &dyn std::fmt::Display) -> ServerError {
    ServerError::SignInFailed {
        reason: reason.to_string(),
    }
}

/// The origin of `url`: its scheme, host and port, as an address prefix.
fn origin_of(url: &str) -> Result<String, ServerError> {
    let parsed = reqwest::Url::parse(url).map_err(|error| failed(&error))?;
    Ok(parsed.origin().ascii_serialization())
}

/// The service's OIDC relying party.
pub struct Oidc {
    metadata: CoreProviderMetadata,
    client_id: ClientId,
    secret: ClientSecret,
    redirect: RedirectUrl,
    http: reqwest::Client,
    in_flight: Mutex<Flights<(PkceCodeVerifier, Nonce)>>,
}

impl Oidc {
    /// Discover the configured issuer over the address the service reaches
    /// it at.
    ///
    /// The issuer's public address is Lys's own origin, which a browser
    /// uses and the service does not: the issuer's discovery document is
    /// read from `sign_in_api`, its name must be the configured issuer
    /// exactly, and every endpoint it names on its public origin is reached
    /// on the `sign_in_api` origin instead. The document is otherwise taken
    /// as the issuer wrote it, and every token is still checked against the
    /// issuer's name and keys.
    pub async fn discover(config: &Config) -> Result<Self, ServerError> {
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| failed(&error))?;
        let api = config.sign_in_api();
        let text = http
            .get(format!("{api}/.well-known/openid-configuration"))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| failed(&error))?
            .text()
            .await
            .map_err(|error| failed(&error))?;
        let mut document: serde_json::Value =
            serde_json::from_str(&text).map_err(|error| failed(&error))?;
        let named = document.get("issuer").and_then(serde_json::Value::as_str);
        if named != Some(config.issuer.as_str()) {
            return Err(failed(&format!(
                "the issuer names itself {}, not the configured {}",
                named.unwrap_or("nothing"),
                config.issuer
            )));
        }
        let public = origin_of(&config.issuer)?;
        let reached = origin_of(&api)?;
        if let Some(fields) = document.as_object_mut() {
            for (key, value) in fields.iter_mut() {
                if key == "issuer" {
                    continue;
                }
                if let Some(rest) = value.as_str().and_then(|url| url.strip_prefix(&public)) {
                    *value = serde_json::Value::String(format!("{reached}{rest}"));
                }
            }
        }
        let metadata: CoreProviderMetadata =
            serde_json::from_value(document).map_err(|error| failed(&error))?;
        let keys = CoreJsonWebKeySet::fetch_async(metadata.jwks_uri(), &http)
            .await
            .map_err(|error| failed(&error))?;
        let metadata = metadata.set_jwks(keys);
        Ok(Self {
            metadata,
            client_id: ClientId::new(config.client_id.clone()),
            secret: config.client_secret()?,
            redirect: RedirectUrl::new(config.redirect_url.clone())
                .map_err(|error| failed(&error))?,
            http,
            in_flight: Mutex::new(Flights::default()),
        })
    }

    /// The configured issuer; no client credential is included.
    pub(crate) fn issuer(&self) -> &str {
        self.metadata.issuer().as_str()
    }

    /// Begin a sign-in, answering the issuer URL to send the browser to.
    pub fn begin(&self, address: IpAddr) -> Result<String, ServerError> {
        let client = CoreClient::from_provider_metadata(
            self.metadata.clone(),
            self.client_id.clone(),
            Some(self.secret.clone()),
        )
        .set_redirect_uri(self.redirect.clone());
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(challenge)
            .url();
        let mut in_flight = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        in_flight.insert(
            state.secret().clone(),
            (verifier, nonce),
            address,
            Instant::now(),
        )?;
        Ok(url.to_string())
    }

    /// Forget the sign-in in flight under `state`, which will not be finished.
    pub fn abandon(&self, state: &str) {
        self.in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(state);
    }

    /// Finish a sign-in from the issuer's answer, validating its ID token.
    pub async fn finish(&self, code: String, state: &str) -> Result<Actor, ServerError> {
        let (verifier, nonce) = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take(state, Instant::now())?;
        let client = CoreClient::from_provider_metadata(
            self.metadata.clone(),
            self.client_id.clone(),
            Some(self.secret.clone()),
        )
        .set_redirect_uri(self.redirect.clone());
        let answer = client
            .exchange_code(AuthorizationCode::new(code))
            .map_err(|error| failed(&error))?
            .set_pkce_verifier(verifier)
            .request_async(&self.http)
            .await
            .map_err(|error| match error {
                // The issuer's own refusal is said; a transport failure is
                // said without the issuer's address, which a browser reads.
                openidconnect::RequestTokenError::ServerResponse(answer) => {
                    failed(&format!("the sign-in service refused the code: {answer}"))
                }
                _ => failed(&"the sign-in service did not answer the code exchange"),
            })?;
        let token = answer
            .id_token()
            .ok_or_else(|| failed(&"the issuer answered no ID token"))?;
        let checker = client.id_token_verifier().set_allowed_algs([
            CoreJwsSigningAlgorithm::EdDsa,
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        ]);
        let claims = token
            .claims(&checker, &nonce)
            .map_err(|error| failed(&error))?;
        let authenticated = claims.auth_time().unwrap_or_else(|| claims.issue_time());
        let at = u64::try_from(authenticated.timestamp())
            .map_err(|_error| failed(&"the authentication time is before the epoch"))?;
        let binding = LoginBinding::new(claims.issuer().as_str(), claims.subject().as_str())?;
        Ok(Actor::new(binding, Provenance::new(AuthMethod::Oidc, at)))
    }
}
