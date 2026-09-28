//! The mapping of every refusal to an HTTP answer, each by name.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityError;
use lys_identity::grants::GrantError;

/// Everything the service refuses.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// A directory refusal.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// A grant refusal, from the grants' one authority owner.
    #[error(transparent)]
    Grant(#[from] GrantError),
    /// The caller has no live session.
    #[error("NotSignedIn: sign in through the configured issuer first")]
    NotSignedIn,
    /// The caller is signed in but not admitted to this act.
    #[error("NotAdmitted: {reason}")]
    NotAdmitted {
        /// Why the caller is not admitted.
        reason: &'static str,
    },
    /// The caller is signed in through a login bound to no person, so no personal view is theirs.
    #[error("NoPerson: the signed-in login is bound to no person in the directory")]
    NoPerson,
    /// The agent is not one the caller may see: the directory does not hold it, or it answers to another person.
    #[error("AgentNotVisible: no agent by that id is visible to the signed-in caller")]
    AgentNotVisible,
    /// The grant is not one the caller may see: the grants do not hold it, or it is another's.
    #[error("GrantNotVisible: no grant by that id is visible to the signed-in caller")]
    GrantNotVisible,
    /// A grant refusal whose record names a grant or identity the caller may not see.
    #[error("{refusal}: the refusal names a grant or identity the caller may not inspect")]
    Withheld {
        /// The refusal's name, as the whole refusal carries it.
        refusal: String,
    },
    /// The session is not one the caller may end: it is not live, or it belongs to another person.
    #[error("SessionUnknown: no live session by that id is visible to the signed-in caller")]
    SessionUnknown,
    /// A sign-in answer names a state this service did not issue, or one already used.
    #[error("SignInStateUnknown: the sign-in answer does not match a sign-in this service began")]
    SignInStateUnknown,
    /// The issuer could not be reached or its answer did not validate.
    #[error("SignInFailed: {reason}")]
    SignInFailed {
        /// What failed.
        reason: String,
    },
    /// The configuration is not one the service runs under.
    #[error("ConfigInvalid: {reason}")]
    ConfigInvalid {
        /// What is wrong with it.
        reason: String,
    },
    /// A request body or path is not one the route takes.
    #[error("RequestMalformed: {reason}")]
    RequestMalformed {
        /// What is wrong with it.
        reason: String,
    },
    /// The secrets broker is not configured, or could not be reached or read.
    #[error("SecretsUnavailable: {reason}")]
    SecretsUnavailable {
        /// What failed.
        reason: String,
    },
    /// The secrets broker refused the request, by name.
    #[error("{refusal}: {reason}")]
    SecretsRefused {
        /// The status the broker answered with.
        status: StatusCode,
        /// The broker's name for the refusal.
        refusal: String,
        /// The broker's words.
        reason: String,
    },
    /// The directory's worker could not be reached.
    #[error("DirectoryUnavailable: {reason}")]
    DirectoryUnavailable {
        /// What failed.
        reason: String,
    },
}

impl ServerError {
    /// The refusal's name: the first word of its message.
    pub fn name(&self) -> String {
        let text = self.to_string();
        text.split(':').next().unwrap_or_default().to_owned()
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotSignedIn => StatusCode::UNAUTHORIZED,
            Self::NotAdmitted { .. } | Self::NoPerson | Self::Withheld { .. } => {
                StatusCode::FORBIDDEN
            }
            Self::AgentNotVisible | Self::GrantNotVisible | Self::SessionUnknown => {
                StatusCode::NOT_FOUND
            }
            Self::SignInStateUnknown | Self::RequestMalformed { .. } => StatusCode::BAD_REQUEST,
            Self::SignInFailed { .. } | Self::SecretsUnavailable { .. } => StatusCode::BAD_GATEWAY,
            Self::ConfigInvalid { .. } | Self::DirectoryUnavailable { .. } => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::SecretsRefused { status, .. } => *status,
            Self::Identity(error) => identity_status(error),
            Self::Grant(error) => grant_status(error),
        }
    }
}

fn identity_status(error: &IdentityError) -> StatusCode {
    match error {
        IdentityError::IdentityUnknown { .. } => StatusCode::NOT_FOUND,
        IdentityError::AppendUncertain { .. }
        | IdentityError::LogUnavailable { .. }
        | IdentityError::AppendRefused { .. }
        | IdentityError::RandomSourceUnavailable { .. }
        | IdentityError::KeyUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::CONFLICT,
    }
}

fn grant_status(error: &GrantError) -> StatusCode {
    match error {
        GrantError::Identity(error) => identity_status(error),
        GrantError::GrantIdMalformed { .. }
        | GrantError::TokenInvalid { .. }
        | GrantError::AuthorityAbsent { .. }
        | GrantError::PassOnOutside
        | GrantError::WindowInvalid { .. }
        | GrantError::LineageMalformed { .. }
        | GrantError::MemberUnknown { .. }
        | GrantError::MemberMissing { .. }
        | GrantError::RecipientKindUnknown { .. }
        | GrantError::GrantMalformed { .. }
        | GrantError::GrantNotCanonical
        | GrantError::EventMalformed { .. }
        | GrantError::EventNotCanonical
        | GrantError::EventTooLarge { .. } => StatusCode::BAD_REQUEST,
        GrantError::SourceUnknown { .. } | GrantError::GrantUnknown { .. } => StatusCode::NOT_FOUND,
        GrantError::ModelInvalid { .. }
        | GrantError::EventMismatch { .. }
        | GrantError::VersionUnsupported { .. }
        | GrantError::SignerMismatch
        | GrantError::SignatureInvalid
        | GrantError::ReceiptInvalid { .. }
        | GrantError::LogUnavailable { .. }
        | GrantError::LeafNotAnEvent { .. }
        | GrantError::AppendRefused { .. }
        | GrantError::OperationUnresolved { .. }
        | GrantError::ProjectionPending { .. }
        | GrantError::StaleDecision { .. }
        | GrantError::PermissionEngineUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        GrantError::OperationReused { .. }
        | GrantError::GrantExists { .. }
        | GrantError::AlreadyRevoked { .. } => StatusCode::CONFLICT,
        _ => StatusCode::FORBIDDEN,
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "refusal": self.name(), "reason": self.to_string() });
        (self.status(), Json(body)).into_response()
    }
}
