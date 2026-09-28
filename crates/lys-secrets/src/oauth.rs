//! An OAuth service grant kept in the store: the access token, when it
//! expires, the refresh token, and the provenance of the consent (the
//! token endpoint, the client, the provider subject the account holder
//! selected, and the scopes). A refresh keeps the client and the subject;
//! it never rebinds the grant to another account. Neither token ever
//! prints.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::SecretsError;
use crate::secret::Secret;

/// How long before expiry an access token is refreshed.
pub const REFRESH_MARGIN_MS: i64 = 30_000;

/// Where a grant came from, as the consent recorded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The provider's token endpoint.
    pub token_endpoint: String,
    /// The provider's revocation endpoint, when it has one.
    pub revocation_endpoint: Option<String>,
    /// The client the grant was consented to.
    pub client_id: String,
    /// The provider subject selected during service consent.
    pub provider_subject: String,
    /// The consented scopes.
    pub scopes: Vec<String>,
}

/// An OAuth service grant.
pub struct OAuthGrant {
    provenance: Provenance,
    client_secret: Option<Secret>,
    access_token: Secret,
    access_expires_ms: i64,
    refresh_token: Secret,
}

impl fmt::Debug for OAuthGrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthGrant")
            .field("provenance", &self.provenance)
            .field("access_expires_ms", &self.access_expires_ms)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize)]
struct Stored {
    provenance: Provenance,
    client_secret: Option<String>,
    access_token: String,
    access_expires_ms: i64,
    refresh_token: String,
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
}

fn text(secret: &Secret, what: &'static str) -> Result<String, SecretsError> {
    String::from_utf8(secret.expose().to_vec())
        .ok()
        .ok_or_else(|| SecretsError::Encoding {
            context: what,
            reason: "not UTF-8".to_owned(),
        })
}

impl OAuthGrant {
    /// A grant as consent produced it.
    pub fn new(
        provenance: Provenance,
        client_secret: Option<Secret>,
        access_token: Secret,
        access_expires_ms: i64,
        refresh_token: Secret,
    ) -> Self {
        Self {
            provenance,
            client_secret,
            access_token,
            access_expires_ms,
            refresh_token,
        }
    }

    /// The grant as the store seals it.
    ///
    /// # Errors
    ///
    /// `Encoding` when a token is not UTF-8.
    pub fn to_sealed(&self) -> Result<Secret, SecretsError> {
        let stored = Stored {
            provenance: self.provenance.clone(),
            client_secret: self
                .client_secret
                .as_ref()
                .map(|secret| text(secret, "client secret"))
                .transpose()?,
            access_token: text(&self.access_token, "access token")?,
            access_expires_ms: self.access_expires_ms,
            refresh_token: text(&self.refresh_token, "refresh token")?,
        };
        let bytes = serde_json::to_vec(&stored).map_err(|error| SecretsError::Encoding {
            context: "OAuth grant",
            reason: error.to_string(),
        })?;
        Ok(Secret::new(bytes))
    }

    /// Reads a grant the store sealed.
    ///
    /// # Errors
    ///
    /// `Encoding` when the bytes are not a grant.
    pub fn from_sealed(sealed: &Secret) -> Result<Self, SecretsError> {
        let stored: Stored =
            serde_json::from_slice(sealed.expose()).map_err(|error| SecretsError::Encoding {
                context: "OAuth grant",
                reason: error.to_string(),
            })?;
        Ok(Self {
            provenance: stored.provenance,
            client_secret: stored
                .client_secret
                .map(|secret| Secret::new(secret.into_bytes())),
            access_token: Secret::new(stored.access_token.into_bytes()),
            access_expires_ms: stored.access_expires_ms,
            refresh_token: Secret::new(stored.refresh_token.into_bytes()),
        })
    }

    /// Where the grant came from.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// The access token, for the one header the proxy writes it into.
    pub fn access_token(&self) -> &Secret {
        &self.access_token
    }

    /// Every token the grant holds, for redacting an upstream's answer.
    pub fn tokens(&self) -> Vec<&Secret> {
        let mut tokens = vec![&self.access_token, &self.refresh_token];
        tokens.extend(self.client_secret.as_ref());
        tokens
    }

    /// Whether the access token is expired, or expires within the margin.
    pub fn needs_refresh(&self, now_ms: i64) -> bool {
        now_ms.saturating_add(REFRESH_MARGIN_MS) >= self.access_expires_ms
    }

    /// The form a refresh posts to the token endpoint.
    pub fn refresh_form(&self) -> Vec<(&'static str, Secret)> {
        let mut form = vec![
            ("grant_type", Secret::from_slice(b"refresh_token")),
            (
                "refresh_token",
                Secret::from_slice(self.refresh_token.expose()),
            ),
            (
                "client_id",
                Secret::from_slice(self.provenance.client_id.as_bytes()),
            ),
        ];
        if let Some(secret) = &self.client_secret {
            form.push(("client_secret", Secret::from_slice(secret.expose())));
        }
        form
    }

    /// The form a revocation posts: the refresh token, which ends the grant.
    pub fn revocation_form(&self) -> Vec<(&'static str, Secret)> {
        let mut form = vec![
            ("token", Secret::from_slice(self.refresh_token.expose())),
            ("token_type_hint", Secret::from_slice(b"refresh_token")),
            (
                "client_id",
                Secret::from_slice(self.provenance.client_id.as_bytes()),
            ),
        ];
        if let Some(secret) = &self.client_secret {
            form.push(("client_secret", Secret::from_slice(secret.expose())));
        }
        form
    }

    /// Takes the token endpoint's answer to a refresh at `now_ms`. The
    /// client and the subject are kept; a new refresh token replaces the
    /// old one when the provider rotates it. An answer that states no
    /// lifetime is taken as expiring at once, so the next call refreshes.
    ///
    /// # Errors
    ///
    /// `Encoding` when the answer is not a token answer.
    pub fn apply_refresh(&mut self, answer: &[u8], now_ms: i64) -> Result<(), SecretsError> {
        let answer: TokenAnswer = serde_json::from_slice(answer).ok().ok_or_else(|| {
            SecretsError::Encoding {
                context: "token endpoint answer",
                reason: "not a token answer".to_owned(),
            }
        })?;
        self.access_token = Secret::new(answer.access_token.into_bytes());
        let lifetime_ms = answer.expires_in.unwrap_or(0).saturating_mul(1_000);
        self.access_expires_ms = now_ms.saturating_add(lifetime_ms);
        if let Some(refresh) = answer.refresh_token {
            self.refresh_token = Secret::new(refresh.into_bytes());
        }
        Ok(())
    }
}
