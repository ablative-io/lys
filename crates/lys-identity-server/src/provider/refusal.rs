//! A token request's refusal in OAuth's own words, carrying Lys's name for
//! it beside them, moved out of `endpoints.rs` whole.

use axum::Json;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::apps_error::AppError;
use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::error_provider::ProviderError;

/// An OAuth error answer carrying the refusal by name.
pub(super) fn oauth_refusal(error: &ServerError) -> Response {
    let code = match error {
        ServerError::Provider(ProviderError::ClientUnknown)
        | ServerError::App(
            AppError::CredentialRefused { .. }
            | AppError::AppNotApproved { .. }
            | AppError::AppRetired { .. },
        ) => "invalid_client",
        ServerError::App(AppError::RedirectInvalid { .. })
        | ServerError::Provider(
            ProviderError::CodeUsed
            | ProviderError::CodeExpired
            | ProviderError::CodeUnknown
            | ProviderError::VerifierWrong
            | ProviderError::RedirectUnregistered
            | ProviderError::RefreshUnknown
            | ProviderError::SessionEnded
            | ProviderError::HolderRetired { .. },
        ) => "invalid_grant",
        ServerError::Provider(
            ProviderError::ScopeUnknown { .. } | ProviderError::ScopeNotGranted { .. },
        ) => "invalid_scope",
        ServerError::RequestMalformed { .. }
        | ServerError::BodyTooLarge
        | ServerError::GrantStream(GrantStreamError::BindingUnsupported { .. })
        | ServerError::Holding(..) => "invalid_request",
        ServerError::Team(..)
        | ServerError::Budget(..)
        | ServerError::HarnessCatalogueUnreadable { .. }
        | ServerError::Identity(..)
        | ServerError::Grant(..)
        | ServerError::App(..)
        | ServerError::Goal(..)
        | ServerError::Inactive { .. }
        | ServerError::NotSignedIn
        | ServerError::NotAdmitted { .. }
        | ServerError::NoPerson
        | ServerError::SetupRequired
        | ServerError::AgentNotVisible
        | ServerError::Call(..)
        | ServerError::GrantNotVisible
        | ServerError::Withheld { .. }
        | ServerError::SessionUnknown
        | ServerError::SignInStateUnknown
        | ServerError::SignInRefused
        | ServerError::SignInThrottled
        | ServerError::RegistrationThrottled
        | ServerError::SecondFactorUnsupported
        | ServerError::SetupClosed
        | ServerError::SetupCodeRefused
        | ServerError::SetupUnavailable { .. }
        | ServerError::AccountRefused { .. }
        | ServerError::SignInFailed { .. }
        | ServerError::ConfigInvalid { .. }
        | ServerError::BootstrapInterrupted { .. }
        | ServerError::SecretsUnavailable { .. }
        | ServerError::SecretsRefused { .. }
        | ServerError::RequestsUnavailable { .. }
        | ServerError::McpRequestsUnavailable { .. }
        | ServerError::McpServerUnknown { .. }
        | ServerError::McpServerHeld { .. }
        | ServerError::McpBeyondRemit { .. }
        | ServerError::RequestUnknown
        | ServerError::RequestDecided { .. }
        | ServerError::RequestHeld { .. }
        | ServerError::RequestReused { .. }
        | ServerError::NetworkUnavailable { .. }
        | ServerError::LoginUnbound
        | ServerError::MachineUnknown
        | ServerError::Machine(..)
        | ServerError::Cord(..)
        | ServerError::Canvas(..)
        | ServerError::ProductDraft(..)
        | ServerError::SessionsUnavailable { .. }
        | ServerError::MemoryUnavailable { .. }
        | ServerError::ProvisioningUnavailable { .. }
        | ServerError::ProvisioningChanged { .. }
        | ServerError::ProvisioningReused { .. }
        | ServerError::ProfileVersionUnknown { .. }
        | ServerError::ProfileVersionReplaced { .. }
        | ServerError::ProfileNotReviewed { .. }
        | ServerError::CertificatesUnavailable { .. }
        | ServerError::CertificateUnknown { .. }
        | ServerError::CertificateReused { .. }
        | ServerError::CertificateWithdrawn { .. }
        | ServerError::RolesUnavailable { .. }
        | ServerError::RoleUnknown
        | ServerError::RoleVersionUnknown
        | ServerError::HolderUnknown
        | ServerError::RoleReused { .. }
        | ServerError::RoleHeld { .. }
        | ServerError::HoldingOver { .. }
        | ServerError::HoldingChanged
        | ServerError::LaunchRecordMissing
        | ServerError::OperatorRefused { .. }
        | ServerError::AgentPassRefused { .. }
        | ServerError::AgentSignatureRefused { .. }
        | ServerError::AgentNotActive { .. }
        | ServerError::MachineCannotReach { .. }
        | ServerError::MachineRetired
        | ServerError::MachineNotForAgent
        | ServerError::MachineWithoutRuntime
        | ServerError::MachineWithoutRunner
        | ServerError::SkillUnknown { .. }
        | ServerError::PolicyUnrepresentable { .. }
        | ServerError::ModelUnrepresentable { .. }
        | ServerError::HarnessUndeclared { .. }
        | ServerError::LaunchUnrenderable { .. }
        | ServerError::WorkingFolderUnnamed
        | ServerError::McpCredentialInline { .. }
        | ServerError::McpSettingUnrepresentable { .. }
        | ServerError::McpHandleUnsupported { .. }
        | ServerError::RuntimeUnavailable { .. }
        | ServerError::RuntimeSessionUnknown
        | ServerError::RuntimeSessionStarted { .. }
        | ServerError::RuntimeSessionStopped { .. }
        | ServerError::RuntimeReportReused { .. }
        | ServerError::ServiceAccountsUnavailable { .. }
        | ServerError::ServiceAccountUnknown
        | ServerError::ServiceAccountReused { .. }
        | ServerError::ServiceAccountRetired { .. }
        | ServerError::ServiceAccountOwnerRetired { .. }
        | ServerError::PolicyUnavailable { .. }
        | ServerError::AgentHasNoPolicy { .. }
        | ServerError::PolicyVersionConflict { .. }
        | ServerError::PolicyRefused { .. }
        | ServerError::StopsUnavailable { .. }
        | ServerError::StopReused { .. }
        | ServerError::ReviewsUnavailable { .. }
        | ServerError::ReviewerOnly
        | ServerError::GrantNotDue { .. }
        | ServerError::ReviewReused { .. }
        | ServerError::DirectoryUnavailable { .. }
        | ServerError::SignInProvidersUnavailable { .. }
        | ServerError::Provider(
            ProviderError::Unavailable { .. }
            | ProviderError::TokenUnknown
            | ProviderError::PassRefused { .. },
        )
        | ServerError::ProviderRefused { .. }
        | ServerError::GrantStream(..)
        | ServerError::SignInProvidersRefused { .. }
        | ServerError::NotPermitted { .. }
        | ServerError::NoLiveSession { .. }
        | ServerError::RunnerAbsent { .. }
        | ServerError::IssuerChallengeExpired
        | ServerError::IssuerRefused { .. }
        | ServerError::Runner { .. } => "server_error",
    };
    let body = json!({
        "error": code,
        "error_description": error.to_string(),
        "refusal": error.name(),
        "reason": error.to_string(),
    });
    (
        error.status(),
        [(header::CACHE_CONTROL, "no-store")],
        Json(body),
    )
        .into_response()
}
