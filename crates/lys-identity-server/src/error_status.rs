//! How the directory's and the grants' own refusals are answered over HTTP.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityError;
use lys_identity::grants::GrantError;

use crate::error::ServerError;

macro_rules! unavailable_refusals {
    () => {
        ServerError::IssuerChallengeExpired
            | ServerError::ConfigInvalid { .. }
            | ServerError::HarnessCatalogueUnreadable { .. }
            | ServerError::DirectoryUnavailable { .. }
            | ServerError::OrganisationBootRefused { .. }
            | ServerError::RequestsUnavailable { .. }
            | ServerError::McpRequestsUnavailable { .. }
            | ServerError::NetworkUnavailable { .. }
            | ServerError::RolesUnavailable { .. }
            | ServerError::MembershipUnavailable { .. }
            | ServerError::SessionsUnavailable { .. }
            | ServerError::ClockUnavailable { .. }
            | ServerError::MemoryUnavailable { .. }
            | ServerError::ProvisioningUnavailable { .. }
            | ServerError::CertificatesUnavailable { .. }
            | ServerError::RuntimeUnavailable { .. }
            | ServerError::ServiceAccountsUnavailable { .. }
            | ServerError::StopsUnavailable { .. }
            | ServerError::PolicyUnavailable { .. }
            | ServerError::SignInProvidersUnavailable { .. }
            | ServerError::SetupUnavailable { .. }
            | ServerError::ReviewsUnavailable { .. }
            | ServerError::SignInFailed { .. }
            | ServerError::IssuerRefused { .. }
            | ServerError::SecretsUnavailable { .. }
            | ServerError::SignInProvidersRefused { .. }
            | ServerError::MailNotSent { .. }
    };
}

macro_rules! conflicting_refusals {
    () => {
        ServerError::EmailHeldByAnother { .. }
            | ServerError::RequestDecided { .. }
            | ServerError::RequestHeld { .. }
            | ServerError::RequestReused { .. }
            | ServerError::McpServerHeld { .. }
            | ServerError::RoleReused { .. }
            | ServerError::RoleHeld { .. }
            | ServerError::HoldingOver { .. }
            | ServerError::HoldingChanged
            | ServerError::ProvisioningChanged { .. }
            | ServerError::ProfileVersionReplaced { .. }
            | ServerError::ProvisioningReused { .. }
            | ServerError::CertificateReused { .. }
            | ServerError::CertificateWithdrawn { .. }
            | ServerError::MachineRetired
            | ServerError::AgentNotActive { .. }
            | ServerError::ProfileNotReviewed { .. }
            | ServerError::MachineCannotReach { .. }
            | ServerError::MachineWithoutRuntime
            | ServerError::MachineWithoutRunner
            | ServerError::WorkingFolderUnnamed
            | ServerError::LaunchUnrenderable { .. }
            | ServerError::HarnessUndeclared { .. }
            | ServerError::McpHandleUnsupported { .. }
            | ServerError::RuntimeSessionStarted { .. }
            | ServerError::RuntimeSessionStopped { .. }
            | ServerError::RuntimeReportReused { .. }
            | ServerError::ServiceAccountReused { .. }
            | ServerError::ServiceAccountRetired { .. }
            | ServerError::ServiceAccountOwnerRetired { .. }
            | ServerError::StopReused { .. }
            | ServerError::PolicyVersionConflict { .. }
            | ServerError::GrantNotDue { .. }
            | ServerError::NoLiveSession { .. }
            | ServerError::RunnerAbsent { .. }
            | ServerError::AuthorityUnavailable { .. }
            | ServerError::SetupClosed
            | ServerError::ReviewReused { .. }
            | ServerError::BootstrapInterrupted { .. }
    };
}

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
        | IdentityError::VersionUnsupported { .. }
        | IdentityError::SignerMismatch
        | IdentityError::SignatureInvalid
        | IdentityError::LeafNotAnEvent { .. }
        | IdentityError::AlreadyRegistered { .. }
        | IdentityError::BindingTaken { .. }
        | IdentityError::StateMismatch { .. }
        | IdentityError::OperationReused { .. }
        | IdentityError::LinkSourceSeen { .. }
        | IdentityError::OrganisationMismatch { .. }
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
        | GrantError::ResourceKindUnheld { .. } => StatusCode::BAD_REQUEST,
        GrantError::SourceUnknown { .. } | GrantError::GrantUnknown { .. } => StatusCode::NOT_FOUND,
        GrantError::ModelInvalid { .. }
        | GrantError::EventMismatch { .. }
        | GrantError::VersionUnsupported { .. }
        | GrantError::SignerMismatch
        | GrantError::SignatureInvalid
        | GrantError::ReceiptInvalid { .. }
        | GrantError::LogUnavailable { .. }
        | GrantError::CheckpointRefused { .. }
        | GrantError::LeafNotAnEvent { .. }
        | GrantError::AppendRefused { .. }
        | GrantError::OperationUnresolved { .. }
        | GrantError::ProjectionPending { .. }
        | GrantError::StaleDecision { .. }
        | GrantError::PermissionEngineUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        GrantError::OperationReused { .. }
        | GrantError::ResetRefused { .. }
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
        | GrantError::OrganisationOutside { .. }
        | GrantError::WithheldFromAgents { .. }
        | GrantError::PassOnBeyondSource { .. }
        | GrantError::ExpiryBeyondSource { .. }
        | GrantError::ResponsibleMismatch { .. }
        | GrantError::IdentityNotActive { .. }
        | GrantError::Revoked { .. }
        | GrantError::Expired { .. }
        | GrantError::NotStarted { .. }
        | GrantError::RootAuthorityRefused { .. }
        | GrantError::RootOwnedElsewhere { .. }
        | GrantError::RevokeRefused { .. }
        | GrantError::NotHeld { .. }
        | GrantError::EnvelopeMismatch { .. }
        | GrantError::PermissionAbsent { .. }
        | GrantError::ModeHeld { .. }
        | GrantError::MachineRefused { .. } => StatusCode::FORBIDDEN,
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        if self.status().is_server_error() {
            tracing::error!(refusal = %self.name(), status = self.status().as_u16(),
                "service_dependency_unavailable: a service request failed; the service remains running and runner sessions are left alive");
        }
        let mut body = serde_json::json!({
            "refusal": self.name(),
            "reason": self.to_string(),
            "fields": self.fields(),
        });
        if let Self::EmailHeldByAnother {
            person,
            display_name,
            email,
        } = &self
        {
            body["person"] = serde_json::json!(person);
            body["display_name"] = serde_json::json!(display_name);
            body["email"] = serde_json::json!(email);
        }
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
    #[must_use]
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
            | Self::UpgradePending { .. }
            | Self::SignInRefused
            | Self::SetupCodeRefused => StatusCode::UNAUTHORIZED,
            Self::SignInThrottled | Self::RegistrationThrottled => StatusCode::TOO_MANY_REQUESTS,
            Self::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Inactive { .. }
            | Self::AgentHasNoPolicy { .. }
            | Self::NotAdmitted { .. }
            | Self::ProofRefused
            | Self::AlreadyAdmitted
            | Self::OrganisationNotOwn { .. }
            | Self::NoPerson
            | Self::ProductPassPersonRequired
            | Self::SetupRequired
            | Self::Withheld { .. }
            | Self::MachineNotForAgent
            | Self::SecondFactorUnsupported
            | Self::NotPermitted { .. }
            | Self::ReviewerOnly
            | Self::McpBeyondRemit { .. } => StatusCode::FORBIDDEN,
            Self::NoUnspentAdmission
            | Self::AgentNotVisible
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
            conflicting_refusals!() => StatusCode::CONFLICT,
            Self::SignInStateUnknown
            | Self::AppAdmitGrantRequired
            | Self::RequestMalformed { .. }
            | Self::AccountRefused { .. }
            | Self::ProviderRefused { .. }
            | Self::McpCredentialInline { .. }
            | Self::McpSettingUnrepresentable { .. }
            | Self::ModelUnrepresentable { .. }
            | Self::PolicyUnrepresentable { .. }
            | Self::SkillUnknown { .. }
            | Self::PolicyRefused { .. } => StatusCode::BAD_REQUEST,
            unavailable_refusals!() => StatusCode::SERVICE_UNAVAILABLE,
            Self::SecretsRefused { status, .. } => *status,
            Self::Runner { refusal, .. } => runner_status(refusal),
            Self::Identity(error) => identity_status(error),
            Self::Grant(error) => grant_status(error),
            Self::App(error) => error.status(),
            Self::Team(error) => error.status(),
            Self::Agents(error) => error.status(),
            Self::Call(error) => error.status(),
            Self::Provider(error) => error.status(),
            Self::GrantStream(error) => error.status(),
            Self::Holding(error) => error.status(),
            Self::Machine(error) => error.status(),
            Self::Cord(error) => error.status(),
            Self::Canvas(error) => error.status(),
            Self::ProductDraft(error) => error.status(),
        }
    }
}

/// How a runner's refusal is answered: one the runner never gave, because
/// it could not be reached or spoke another protocol, is a dependency that is
/// unavailable (503, never 502: a proxy in front of Lys, such as Cloudflare,
/// replaces a 502's body with its own page, and the refusal's name is lost); a
/// session it does not hold is not found; the rest conflict with the
/// session's state.
fn runner_status(refusal: &str) -> StatusCode {
    match refusal {
        "runner_unreachable"
        | "runner_reply_malformed"
        | "runner_protocol_mismatch"
        | "runner_request_unsigned"
        | "runner_request_replayed" => StatusCode::SERVICE_UNAVAILABLE,
        "session_unknown" | "runner_reply_unknown" | "pair_unknown" => StatusCode::NOT_FOUND,
        "runner_reply_expired" => StatusCode::GONE,
        "pattern_invalid"
        | "size_invalid"
        | "cursor_ahead"
        | "session_invalid"
        | "folder_invalid"
        | "pair_request_invalid" => StatusCode::BAD_REQUEST,
        _ => StatusCode::CONFLICT,
    }
}

#[cfg(test)]
mod dependency_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn a_dependency_failure_is_loud_and_does_not_end_the_service()
    -> Result<(), Box<dyn std::error::Error>> {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        let say: crate::routes::Say = Arc::new(move |line| {
            if let Ok(mut lines) = kept.lock() {
                lines.push(line.to_owned());
            }
        });
        tracing::subscriber::with_default(crate::warn_lines::WarnLines::new(say), || {
            let response = ServerError::SignInFailed {
                reason: "the sign-in dependency is unreachable".into(),
            }
            .into_response();
            assert!(response.status().is_server_error());
        });
        assert!(
            heard
                .lock()
                .map_err(|error| error.to_string())?
                .iter()
                .any(|line| line.contains("service_dependency_unavailable")
                    && line.contains("remains running")),
            "a lost compose dependency was not named in the service log"
        );
        Ok(())
    }
}
