//! Why Lys's `OpenID` provider refused an authorization, an exchange or a
//! userinfo request; `error_status` answers each through [`ProviderError::status`].

use axum::http::StatusCode;

/// A refusal of Lys's `OpenID` provider.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Lys's `OpenID` provider is not configured, or cannot answer now.
    #[error("ProviderUnavailable: {reason}")]
    Unavailable {
        /// What failed.
        reason: String,
    },
    /// No product is registered with that client id, or its secret is wrong.
    #[error("ClientUnknown: no product is registered with that client id and secret")]
    ClientUnknown,
    /// The redirect address is not one registered for the product.
    #[error("RedirectUnregistered: that redirect address is not registered for this product")]
    RedirectUnregistered,
    /// The authorization asks for a scope Lys does not serve.
    #[error("ScopeUnknown: Lys does not serve the scope {scope}")]
    ScopeUnknown {
        /// The scope asked for.
        scope: String,
    },
    /// The authorization asks for a scope the app's sign-in settings do not
    /// grant it.
    #[error("ScopeNotGranted: {app} is not given the scope {scope}")]
    ScopeNotGranted {
        /// The scope asked for.
        scope: String,
        /// The app asking.
        app: String,
    },
    /// The code is not one Lys answered this product with.
    #[error("CodeUnknown: that code is not one Lys gave this product")]
    CodeUnknown,
    /// The code was already exchanged.
    #[error("CodeUsed: that code was already used")]
    CodeUsed,
    /// The code is past its instant.
    #[error("CodeExpired: that code is past the instant it was good until")]
    CodeExpired,
    /// The PKCE verifier does not match the code's challenge.
    #[error("VerifierWrong: the PKCE verifier does not match the code's challenge")]
    VerifierWrong,
    /// The access token is not one Lys issued, or is past its instant.
    #[error("TokenUnknown: that access token is not one Lys issued, or it has ended")]
    TokenUnknown,
}

impl ProviderError {
    /// The refusal's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "ProviderUnavailable",
            Self::ClientUnknown => "ClientUnknown",
            Self::RedirectUnregistered => "RedirectUnregistered",
            Self::ScopeUnknown { .. } => "ScopeUnknown",
            Self::ScopeNotGranted { .. } => "ScopeNotGranted",
            Self::CodeUnknown => "CodeUnknown",
            Self::CodeUsed => "CodeUsed",
            Self::CodeExpired => "CodeExpired",
            Self::VerifierWrong => "VerifierWrong",
            Self::TokenUnknown => "TokenUnknown",
        }
    }

    /// The status the refusal is answered with.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::ClientUnknown | Self::TokenUnknown => StatusCode::UNAUTHORIZED,
            Self::ScopeNotGranted { .. } => StatusCode::FORBIDDEN,
            Self::RedirectUnregistered
            | Self::ScopeUnknown { .. }
            | Self::CodeUnknown
            | Self::CodeUsed
            | Self::CodeExpired
            | Self::VerifierWrong => StatusCode::BAD_REQUEST,
        }
    }
}
