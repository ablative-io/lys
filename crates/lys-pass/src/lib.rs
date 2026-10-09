//! Shared pass verification and permission contracts for products.

pub mod conformance;
pub mod deliberate;
pub mod drafts;
pub mod membership;
pub mod membership_conformance;
pub mod refusal;
pub mod rights;
pub mod verify;

pub use membership::{GrantLog, MembershipDecision, MembershipRequest, Subject, Verdict};
pub use refusal::Refusal;
pub use rights::{Claims, Decision, Holder, Mode, Right, Target};
pub use verify::{KeySet, VerifiedPass};

use reqwest::Client as Http;
use serde::{Deserialize, Serialize};
use url::Url;

/// A named refusal at a token, key or live-contract boundary.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The signed audience differs from the product.
    #[error("wrong_audience")]
    WrongAudience,
    /// The signed issuer differs from the trusted issuer.
    #[error("wrong_issuer")]
    WrongIssuer,
    /// The pass has reached its expiry.
    #[error("pass_expired")]
    Expired,
    /// The pass starts in the future.
    #[error("pass_not_yet_valid")]
    NotYetValid,
    /// No published verification key has this id.
    #[error("unpublished_key: {0}")]
    UnpublishedKey(String),
    /// Strict signature verification failed.
    #[error("signature_refused")]
    Signature(#[source] ed25519_dalek::SignatureError),
    /// A compact token segment was not canonical base64url.
    #[error("pass_encoding_refused")]
    Encoding(#[from] base64::DecodeError),
    /// A signed claim or published key could not be read.
    #[error("pass_contract_refused")]
    Json(#[from] serde_json::Error),
    /// A pass contains another application's rights.
    #[error("rights_outside_audience")]
    RightsOutsideAudience,
    /// The issuer omitted rights for this application.
    #[error("rights_truncated: {0}")]
    Truncated(String),
    /// A boundary value violates the published contract.
    #[error("contract_refused: {0}")]
    Invalid(&'static str),
    /// A live request failed before a contract answer was received.
    #[error("Lys could not be asked")]
    Transport(#[source] Box<reqwest::Error>),
    /// A live response is outside the permission contract.
    #[error("Lys could not be asked: {0}")]
    CannotAsk(&'static str),
    /// A supplied endpoint is malformed.
    #[error("endpoint_refused")]
    Url(#[from] url::ParseError),
    /// A named issuer refusal of the exchange or refresh.
    #[error("token_refused: {name}: {reason}")]
    TokenRefused {
        /// The issuer's stable refusal name.
        name: String,
        /// The issuer's refusal words.
        reason: String,
    },
}

impl Error {
    /// The stable machine name, without keys, credentials or response bodies.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::WrongAudience => "wrong_audience",
            Self::WrongIssuer => "wrong_issuer",
            Self::Expired => "pass_expired",
            Self::NotYetValid => "pass_not_yet_valid",
            Self::UnpublishedKey(_) => "unpublished_key",
            Self::Signature(_) => "signature_refused",
            Self::Encoding(_) => "pass_encoding_refused",
            Self::Json(_) | Self::Invalid(_) => "contract_refused",
            Self::RightsOutsideAudience => "rights_outside_audience",
            Self::Truncated(_) => "rights_truncated",
            Self::Transport(_) | Self::CannotAsk(_) => "lys_could_not_be_asked",
            Self::Url(_) => "endpoint_refused",
            Self::TokenRefused { name, .. } => name,
        }
    }
}

/// A transport supplied by the product, holding no token or credential.
#[derive(Clone, Debug)]
pub struct Client {
    http: Http,
    issuer: Url,
}

impl Client {
    /// Use the product's configured transport and trusted Lys issuer.
    pub fn new(http: Http, mut issuer: Url) -> Result<Self, Error> {
        if !matches!(issuer.scheme(), "http" | "https")
            || issuer.host_str().is_none()
            || !issuer.username().is_empty()
            || issuer.password().is_some()
            || issuer.query().is_some()
            || issuer.fragment().is_some()
        {
            return Err(Error::Invalid("issuer is not an HTTP origin"));
        }
        if !issuer.path().ends_with('/') {
            issuer.set_path(&format!("{}/", issuer.path()));
        }
        Ok(Self { http, issuer })
    }

    fn endpoint(&self, relative: &str) -> Result<Url, Error> {
        Ok(self.issuer.join(relative)?)
    }

    /// Fetch the issuer's current keyset explicitly; offline checks never fetch.
    pub async fn fetch_keys(&self) -> Result<KeySet, Error> {
        let response = self
            .http
            .get(self.endpoint("oauth/jwks")?)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        let text = response
            .error_for_status()
            .map_err(|error| Error::Transport(Box::new(error)))?
            .text()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        KeySet::from_json(&text)
    }

    /// Exchange an authorization code using credentials owned by the caller.
    pub async fn fetch_token(&self, request: &TokenRequest<'_>) -> Result<TokenResponse, Error> {
        if request.client_id.is_empty()
            || request.client_secret.is_empty()
            || request.code.is_empty()
            || request.redirect_uri.is_empty()
            || request.code_verifier.is_empty()
        {
            return Err(Error::Invalid("token request is incomplete"));
        }
        self.token(&[
            ("grant_type", "authorization_code"),
            ("client_id", request.client_id),
            ("client_secret", request.client_secret),
            ("code", request.code),
            ("redirect_uri", request.redirect_uri),
            ("code_verifier", request.code_verifier),
        ])
        .await
    }

    /// Ask Lys to refresh from live grants; no timer or cached authority is used.
    pub async fn refresh_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<TokenResponse, Error> {
        if client_id.is_empty() || client_secret.is_empty() || refresh_token.is_empty() {
            return Err(Error::Invalid("refresh request is incomplete"));
        }
        self.token(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
        ])
        .await
    }

    async fn token(&self, form: &[(&str, &str)]) -> Result<TokenResponse, Error> {
        let response = self
            .http
            .post(self.endpoint("oauth/token")?)
            .form(form)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if !response.status().is_success() {
            let refused: TokenRefusal = response
                .json()
                .await
                .map_err(|error| Error::Transport(Box::new(error)))?;
            if refused.error.is_empty()
                || refused.refusal.is_empty()
                || refused.reason != refused.error_description
            {
                return Err(Error::CannotAsk("token refusal has no name"));
            }
            return Err(Error::TokenRefused {
                name: refused.refusal,
                reason: refused.error_description,
            });
        }
        let answer: TokenResponse = response
            .json()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if answer.token_type != "Bearer" || answer.access_token.is_empty() || answer.expires_in == 0
        {
            return Err(Error::CannotAsk("invalid token response"));
        }
        Ok(answer)
    }
}

/// Borrowed authorization-code exchange inputs, never retained by the client.
pub struct TokenRequest<'a> {
    /// The registered audience.
    pub client_id: &'a str,
    /// The registered client credential.
    pub client_secret: &'a str,
    /// The one-use authorization code.
    pub code: &'a str,
    /// The registered return address.
    pub redirect_uri: &'a str,
    /// The authorization's PKCE verifier.
    pub code_verifier: &'a str,
}

/// A token response owned by the product that requested it.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TokenResponse {
    /// The signed access pass.
    pub access_token: String,
    /// The authorization scheme.
    pub token_type: String,
    /// The remaining lifetime in seconds.
    pub expires_in: u64,
    /// The identity assertion, when the exchange includes it.
    pub id_token: Option<String>,
    /// The credential used for a live refresh, when issued.
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenRefusal {
    error: String,
    error_description: String,
    refusal: String,
    reason: String,
}
