//! How every refusal is answered over HTTP: the service's own by name, and
//! the directory's and the grants' as they class them.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityError;
use lys_identity::grants::GrantError;

use crate::error::ServerError;

pub(crate) fn identity_status(error: &IdentityError) -> StatusCode {
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

pub(crate) fn grant_status(error: &GrantError) -> StatusCode {
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

impl ServerError {
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::NotSignedIn | Self::AgentSignatureRefused { .. } => StatusCode::UNAUTHORIZED,
            Self::NotAdmitted { .. }
            | Self::NoPerson
            | Self::SetupRequired
            | Self::Withheld { .. }
            | Self::MachineNotForAgent
            | Self::ReviewerOnly => StatusCode::FORBIDDEN,
            Self::AgentNotVisible
            | Self::GrantNotVisible
            | Self::SessionUnknown
            | Self::RequestUnknown
            | Self::MachineUnknown
            | Self::LaunchRecordMissing
            | Self::ProfileVersionUnknown { .. }
            | Self::RuntimeSessionUnknown
            | Self::RoleUnknown
            | Self::RoleVersionUnknown
            | Self::HolderUnknown
            | Self::ServiceAccountUnknown
            | Self::TeamUnknown
            | Self::TeamMemberUnknown
            | Self::CertificateUnknown { .. } => StatusCode::NOT_FOUND,
            Self::RequestDecided { .. }
            | Self::RequestHeld { .. }
            | Self::RequestReused { .. }
            | Self::MachineReused { .. }
            | Self::RoleReused { .. }
            | Self::RoleHeld { .. }
            | Self::HoldingOver { .. }
            | Self::HoldingChanged
            | Self::ProvisioningChanged { .. }
            | Self::ProvisioningReused { .. }
            | Self::CertificateReused { .. }
            | Self::CertificateWithdrawn { .. }
            | Self::MachineRetired
            | Self::AgentNotActive { .. }
            | Self::ProfileNotReviewed { .. }
            | Self::MachineCannotReach { .. }
            | Self::MachineWithoutRuntime
            | Self::LaunchUnrenderable { .. }
            | Self::RuntimeSessionStarted { .. }
            | Self::RuntimeSessionStopped { .. }
            | Self::RuntimeReportReused { .. }
            | Self::ServiceAccountReused { .. }
            | Self::ServiceAccountRetired { .. }
            | Self::ServiceAccountOwnerRetired { .. }
            | Self::TeamReused { .. }
            | Self::TeamRetired { .. }
            | Self::TeamMemberHeld
            | Self::TeamMemberAbsent
            | Self::GrantNotDue { .. }
            | Self::ReviewReused { .. } => StatusCode::CONFLICT,
            Self::SignInStateUnknown | Self::RequestMalformed { .. } => StatusCode::BAD_REQUEST,
            Self::SignInFailed { .. } | Self::SecretsUnavailable { .. } => StatusCode::BAD_GATEWAY,
            Self::ConfigInvalid { .. }
            | Self::DirectoryUnavailable { .. }
            | Self::RequestsUnavailable { .. }
            | Self::NetworkUnavailable { .. }
            | Self::RolesUnavailable { .. }
            | Self::MemoryUnavailable { .. }
            | Self::ProvisioningUnavailable { .. }
            | Self::CertificatesUnavailable { .. }
            | Self::RuntimeUnavailable { .. }
            | Self::ServiceAccountsUnavailable { .. }
            | Self::TeamsUnavailable { .. }
            | Self::ReviewsUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::SecretsRefused { status, .. } => *status,
            Self::Identity(error) => identity_status(error),
            Self::Grant(error) => grant_status(error),
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "refusal": self.name(), "reason": self.to_string() });
        (self.status(), Json(body)).into_response()
    }
}
