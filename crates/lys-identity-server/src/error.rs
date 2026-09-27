//! The mapping of every refusal to an HTTP answer, each by name.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityError;

/// Everything the service refuses.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// A directory refusal.
    #[error(transparent)]
    Identity(#[from] IdentityError),
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
            Self::NotAdmitted { .. } | Self::NoPerson => StatusCode::FORBIDDEN,
            Self::AgentNotVisible => StatusCode::NOT_FOUND,
            Self::SignInStateUnknown | Self::RequestMalformed { .. } => StatusCode::BAD_REQUEST,
            Self::SignInFailed { .. } => StatusCode::BAD_GATEWAY,
            Self::ConfigInvalid { .. } | Self::DirectoryUnavailable { .. } => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::Identity(error) => match error {
                IdentityError::IdentityUnknown { .. } => StatusCode::NOT_FOUND,
                IdentityError::AppendUncertain { .. }
                | IdentityError::LogUnavailable { .. }
                | IdentityError::AppendRefused { .. }
                | IdentityError::RandomSourceUnavailable { .. }
                | IdentityError::KeyUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::CONFLICT,
            },
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "refusal": self.name(), "reason": self.to_string() });
        (self.status(), Json(body)).into_response()
    }
}
