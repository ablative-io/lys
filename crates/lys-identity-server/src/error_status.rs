//! How the directory's and the grants' own refusals are answered over HTTP.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityError;
use lys_identity::grants::GrantError;

use crate::error::ServerError;

pub(crate) fn identity_status(error: &IdentityError) -> StatusCode {
    match error {
        IdentityError::IdentityUnknown { .. }
        | IdentityError::AnswersToUnknown { .. }
        | IdentityError::DraftNotFound { .. } => StatusCode::NOT_FOUND,
        IdentityError::AnswersToInactive { .. } => StatusCode::FORBIDDEN,
        IdentityError::AppendUncertain { .. }
        | IdentityError::LogUnavailable { .. }
        | IdentityError::AppendRefused { .. }
        | IdentityError::RandomSourceUnavailable { .. }
        | IdentityError::KeyUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        IdentityError::IdentifierMalformed { .. } => StatusCode::BAD_REQUEST,
        IdentityError::AnswersToCycle { .. }
        | IdentityError::NoAccountablePerson { .. }
        | IdentityError::DirectorySnapshotUnmigrated { .. }
        | IdentityError::AlreadyBootstrapped { .. }
        | IdentityError::BindingMalformed { .. }
        | IdentityError::ProfileInvalid { .. }
        | IdentityError::ReasonRequired { .. }
        | IdentityError::TransitionRefused { .. }
        | IdentityError::ChangeMismatch { .. }
        | IdentityError::EventMalformed { .. }
        | IdentityError::EventNotCanonical
        | IdentityError::InstallEntry
        | IdentityError::EventTooLarge { .. }
        | IdentityError::VersionUnsupported { .. }
        | IdentityError::SignerMismatch
        | IdentityError::SignatureInvalid
        | IdentityError::LeafNotAnEvent { .. }
        | IdentityError::AlreadyRegistered { .. }
        | IdentityError::BindingTaken { .. }
        | IdentityError::StateMismatch { .. }
        | IdentityError::OperationReused { .. }
        | IdentityError::LinkSourceSeen { .. }
        | IdentityError::ReceiptInvalid { .. }
        | IdentityError::DraftHashMismatch
        | IdentityError::DraftNotPending { .. }
        | IdentityError::DraftChangeInvalid { .. }
        | IdentityError::DraftEntry => StatusCode::CONFLICT,
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
        GrantError::RelationUnknown { .. }
        | GrantError::ActionsOutside { .. }
        | GrantError::LineageCycle { .. }
        | GrantError::IssuerNotHolder { .. }
        | GrantError::NotHolder { .. }
        | GrantError::ResourceOutside { .. }
        | GrantError::UseOnly { .. }
        | GrantError::RecipientRefused { .. }
        | GrantError::WithheldFromAgents { .. }
        | GrantError::PassOnBeyondSource { .. }
        | GrantError::ExpiryBeyondSource { .. }
        | GrantError::ResponsibleMismatch { .. }
        | GrantError::IdentityNotActive { .. }
        | GrantError::Revoked { .. }
        | GrantError::Expired { .. }
        | GrantError::NotStarted { .. }
        | GrantError::RootAuthorityRefused { .. }
        | GrantError::RevokeRefused { .. }
        | GrantError::NotHeld { .. }
        | GrantError::EnvelopeMismatch { .. }
        | GrantError::PermissionAbsent { .. } => StatusCode::FORBIDDEN,
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let mut body = serde_json::json!({
            "refusal": self.name(),
            "reason": self.to_string(),
            "fields": self.fields(),
        });
        if let Self::Identity(error) = &self {
            match error {
                IdentityError::AnswersToCycle { chain }
                | IdentityError::NoAccountablePerson { chain } => {
                    body["chain"] = serde_json::json!(chain);
                }
                IdentityError::AnswersToInactive { identity, state } => {
                    body["identity"] = serde_json::json!(identity);
                    body["state"] = serde_json::json!(state.to_string());
                }
                IdentityError::AnswersToUnknown { identity } => {
                    body["identity"] = serde_json::json!(identity);
                }
                _ => {}
            }
        }
        (self.status(), Json(body)).into_response()
    }
}

impl ServerError {
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
            | Self::AgentPassRefused { .. }
            | Self::AgentSignatureRefused { .. }
            | Self::OperatorRefused { .. }
            | Self::SignInRefused
            | Self::SetupCodeRefused
            | Self::ClientUnknown
            | Self::DialRefused { .. }
            | Self::TokenUnknown => StatusCode::UNAUTHORIZED,
            Self::SignInThrottled => StatusCode::TOO_MANY_REQUESTS,
            Self::Inactive { .. }
            | Self::AgentHasNoPolicy { .. }
            | Self::NotAdmitted { .. }
            | Self::NoPerson
            | Self::SetupRequired
            | Self::Withheld { .. }
            | Self::MachineNotForAgent
            | Self::SecondFactorUnsupported
            | Self::NotPermitted { .. }
            | Self::ReviewerOnly
            | Self::McpBeyondRemit { .. } => StatusCode::FORBIDDEN,
            Self::AgentNotVisible
            | Self::McpServerUnknown { .. }
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
            | Self::CertificateUnknown { .. } => StatusCode::NOT_FOUND,
            Self::RequestDecided { .. }
            | Self::RequestHeld { .. }
            | Self::RequestReused { .. }
            | Self::McpServerHeld { .. }
            | Self::MachineReused { .. }
            | Self::MachineTeamReused { .. }
            | Self::MachineAgentsReused { .. }
            | Self::RoleReused { .. }
            | Self::RoleHeld { .. }
            | Self::HoldingOver { .. }
            | Self::HoldingChanged
            | Self::ProvisioningChanged { .. }
            | Self::ProfileVersionReplaced { .. }
            | Self::ProvisioningReused { .. }
            | Self::CertificateReused { .. }
            | Self::CertificateWithdrawn { .. }
            | Self::MachineRetired
            | Self::AgentNotActive { .. }
            | Self::ProfileNotReviewed { .. }
            | Self::MachineCannotReach { .. }
            | Self::MachineWithoutRuntime
            | Self::MachineWithoutRunner
            | Self::LaunchUnrenderable { .. }
            | Self::HarnessUndeclared { .. }
            | Self::McpHandleUnsupported { .. }
            | Self::RuntimeSessionStarted { .. }
            | Self::RuntimeSessionStopped { .. }
            | Self::RuntimeReportReused { .. }
            | Self::ServiceAccountReused { .. }
            | Self::ServiceAccountRetired { .. }
            | Self::ServiceAccountOwnerRetired { .. }
            | Self::StopReused { .. }
            | Self::PolicyVersionConflict { .. }
            | Self::GrantNotDue { .. }
            | Self::NoLiveSession { .. }
            | Self::RunnerAbsent { .. }
            | Self::DialStale { .. }
            | Self::SetupClosed
            | Self::ReviewReused { .. }
            | Self::BootstrapInterrupted { .. } => StatusCode::CONFLICT,
            Self::SignInStateUnknown
            | Self::RequestMalformed { .. }
            | Self::AccountRefused { .. }
            | Self::ProviderRefused { .. }
            | Self::RedirectUnregistered
            | Self::CodeUnknown
            | Self::CodeUsed
            | Self::CodeExpired
            | Self::VerifierWrong
            | Self::McpCredentialInline { .. }
            | Self::McpSettingUnrepresentable { .. }
            | Self::ModelUnrepresentable { .. }
            | Self::PolicyUnrepresentable { .. }
            | Self::SkillUnknown { .. }
            | Self::PolicyRefused { .. } => StatusCode::BAD_REQUEST,
            Self::SignInFailed { .. }
            | Self::SecretsUnavailable { .. }
            | Self::SignInProvidersRefused { .. } => StatusCode::BAD_GATEWAY,
            Self::ConfigInvalid { .. }
            | Self::HarnessCatalogueUnreadable { .. }
            | Self::DirectoryUnavailable { .. }
            | Self::RequestsUnavailable { .. }
            | Self::McpRequestsUnavailable { .. }
            | Self::NetworkUnavailable { .. }
            | Self::RolesUnavailable { .. }
            | Self::SessionsUnavailable { .. }
            | Self::MemoryUnavailable { .. }
            | Self::ProvisioningUnavailable { .. }
            | Self::CertificatesUnavailable { .. }
            | Self::RuntimeUnavailable { .. }
            | Self::ServiceAccountsUnavailable { .. }
            | Self::StopsUnavailable { .. }
            | Self::PolicyUnavailable { .. }
            | Self::SignInProvidersUnavailable { .. }
            | Self::SetupUnavailable { .. }
            | Self::ProviderUnavailable { .. }
            | Self::ReviewsUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::SecretsRefused { status, .. } => *status,
            Self::Runner { refusal, .. } => runner_status(refusal),
            Self::Identity(error) => identity_status(error),
            Self::Grant(error) => grant_status(error),
            Self::App(error) => error.status(),
            Self::Goal(error) => error.status(),
            Self::Team(error) => error.status(),
            Self::Budget(error) => error.status(),
            Self::Holding(error) => error.status(),
        }
    }
}

/// How a runner's refusal is answered: one the runner never gave, because
/// it could not be reached or spoke another protocol, is the gateway's; a
/// session it does not hold is not found; the rest conflict with the
/// session's state.
fn runner_status(refusal: &str) -> StatusCode {
    match refusal {
        "runner_unreachable"
        | "runner_reply_malformed"
        | "runner_protocol_mismatch"
        | "runner_request_unsigned"
        | "runner_request_replayed" => StatusCode::BAD_GATEWAY,
        "session_unknown" => StatusCode::NOT_FOUND,
        "pattern_invalid" | "size_invalid" | "cursor_ahead" | "session_invalid" => {
            StatusCode::BAD_REQUEST
        }
        _ => StatusCode::CONFLICT,
    }
}
