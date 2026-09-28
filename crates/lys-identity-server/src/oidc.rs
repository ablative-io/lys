//! Sign-in against the configured issuer, and validation of the returned
//! token to an issuer, a subject and an authentication time.
//!
//! Every token check goes through openidconnect: discovery, the code exchange
//! with PKCE S256, the nonce, and the ID token's signature and claims. The
//! state of a sign-in in flight is kept here and used once.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreJwsSigningAlgorithm, CoreProviderMetadata,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
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

/// The service's OIDC relying party.
pub struct Oidc {
    metadata: CoreProviderMetadata,
    client_id: ClientId,
    secret: ClientSecret,
    redirect: RedirectUrl,
    http: reqwest::Client,
    in_flight: Mutex<HashMap<String, (PkceCodeVerifier, Nonce)>>,
}

impl Oidc {
    /// Discover the configured issuer.
    pub async fn discover(config: &Config) -> Result<Self, ServerError> {
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| failed(&error))?;
        let issuer = IssuerUrl::new(config.issuer.clone()).map_err(|error| failed(&error))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &http)
            .await
            .map_err(|error| failed(&error))?;
        Ok(Self {
            metadata,
            client_id: ClientId::new(config.client_id.clone()),
            secret: config.client_secret()?,
            redirect: RedirectUrl::new(config.redirect_url.clone())
                .map_err(|error| failed(&error))?,
            http,
            in_flight: Mutex::new(HashMap::new()),
        })
    }

    /// The configured issuer; no client credential is included.
    pub(crate) fn issuer(&self) -> &str {
        self.metadata.issuer().as_str()
    }

    /// Begin a sign-in, answering the issuer URL to send the browser to.
    pub fn begin(&self) -> Result<String, ServerError> {
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
        if in_flight.len() >= IN_FLIGHT_MAX {
            return Err(failed(&"too many sign-ins are in flight"));
        }
        in_flight.insert(state.secret().clone(), (verifier, nonce));
        Ok(url.to_string())
    }

    /// Finish a sign-in from the issuer's answer, validating its ID token.
    pub async fn finish(&self, code: String, state: &str) -> Result<Actor, ServerError> {
        let (verifier, nonce) = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(state)
            .ok_or(ServerError::SignInStateUnknown)?;
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
            .map_err(|error| failed(&error))?;
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
