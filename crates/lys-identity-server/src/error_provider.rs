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
    /// The refresh token is not one Lys gave this product, or it has ended
    /// with the sign-in it stands on (ACCESS-002 R2).
    #[error("RefreshUnknown: that refresh token is not one Lys gave this product, or it has ended")]
    RefreshUnknown,
    /// The Lys sign-in a refresh token stands on has ended, so no new pass is
    /// issued from it (ACCESS-002 R2).
    #[error("SessionEnded: the Lys sign-in this refresh token stands on has ended")]
    SessionEnded,
    /// The holder a pass would be issued to is retired or suspended, and is
    /// given no pass (ACCESS-002 R2).
    #[error("HolderRetired: {holder} is {state} and is given no pass")]
    HolderRetired {
        /// The holder refused.
        holder: String,
        /// Its recorded lifecycle state.
        state: lys_identity::LifecycleState,
    },
    /// A bearer pass that lys-pass's verifier refuses for this route: its
    /// signature, key, issuer, audience or instant (ACCESS-002).
    #[error("PassRefused: the pass is refused: {refusal}")]
    PassRefused {
        /// lys-pass's name for the refusal.
        refusal: String,
    },
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
            Self::RefreshUnknown => "RefreshUnknown",
            Self::SessionEnded => "SessionEnded",
            Self::HolderRetired { .. } => "HolderRetired",
            Self::PassRefused { .. } => "PassRefused",
        }
    }

    /// The status the refusal is answered with.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::ClientUnknown | Self::TokenUnknown | Self::PassRefused { .. } => {
                StatusCode::UNAUTHORIZED
            }
            Self::ScopeNotGranted { .. } => StatusCode::FORBIDDEN,
            Self::RedirectUnregistered
            | Self::ScopeUnknown { .. }
            | Self::CodeUnknown
            | Self::CodeUsed
            | Self::CodeExpired
            | Self::VerifierWrong
            | Self::RefreshUnknown
            | Self::SessionEnded
            | Self::HolderRetired { .. } => StatusCode::BAD_REQUEST,
        }
    }
}
