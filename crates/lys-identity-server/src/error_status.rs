//! How the directory's and the grants' own refusals are answered over HTTP.

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

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "refusal": self.name(),
            "reason": self.to_string(),
            "fields": self.fields(),
        });
        (self.status(), Json(body)).into_response()
    }
}

impl ServerError {
    /// The refusal's name: the first word of its message.
    pub fn name(&self) -> String {
        let text = self.to_string();
        text.split(':').next().unwrap_or_default().to_owned()
    }

    /// The fields at fault, as JSON pointers into the request body; none
    /// when the refusal is not about a field.
    pub fn fields(&self) -> Vec<crate::apps_error::Field> {
        match self {
            Self::App(error) => error.fields(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::NotSignedIn
            | Self::AgentSignatureRefused { .. }
            | Self::SignInRefused
            | Self::SetupCodeRefused
            | Self::ClientUnknown
            | Self::TokenUnknown => StatusCode::UNAUTHORIZED,
            Self::SignInThrottled => StatusCode::TOO_MANY_REQUESTS,
            Self::NotAdmitted { .. }
            | Self::NoPerson
            | Self::SetupRequired
            | Self::Withheld { .. }
            | Self::MachineNotForAgent
            | Self::SecondFactorUnsupported
            | Self::ReviewerOnly => StatusCode::FORBIDDEN,
            Self::AgentNotVisible
            | Self::GrantNotVisible
            | Self::SessionUnknown
            | Self::RequestUnknown
            | Self::MachineUnknown
            | Self::LoginUnbound
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
            | Self::StopReused { .. }
            | Self::TeamRetired { .. }
            | Self::TeamMemberHeld
            | Self::TeamMemberAbsent
            | Self::GrantNotDue { .. }
            | Self::SetupClosed
            | Self::ReviewReused { .. } => StatusCode::CONFLICT,
            Self::SignInStateUnknown
            | Self::RequestMalformed { .. }
            | Self::AccountRefused { .. }
            | Self::ProviderRefused { .. }
            | Self::RedirectUnregistered
            | Self::CodeUnknown
            | Self::CodeUsed
            | Self::CodeExpired
            | Self::VerifierWrong => StatusCode::BAD_REQUEST,
            Self::SignInFailed { .. }
            | Self::SecretsUnavailable { .. }
            | Self::SignInProvidersRefused { .. } => StatusCode::BAD_GATEWAY,
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
            | Self::StopsUnavailable { .. }
            | Self::SignInProvidersUnavailable { .. }
            | Self::SetupUnavailable { .. }
            | Self::ProviderUnavailable { .. }
            | Self::ReviewsUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::SecretsRefused { status, .. } => *status,
            Self::Identity(error) => identity_status(error),
            Self::Grant(error) => grant_status(error),
            Self::App(error) => error.status(),
        }
    }
}
