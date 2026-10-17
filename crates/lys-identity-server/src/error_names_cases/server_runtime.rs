//! Refusal fixtures preserve their protocol names across explanation changes.
use super::*;

pub(super) fn server_profiles(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::ProvisioningUnavailable {
                reason: detail.to_owned(),
            },
            "ProvisioningUnavailable",
        ),
        (
            ServerError::ProvisioningChanged { latest: 7 },
            "ProvisioningChanged",
        ),
        (
            ServerError::ProvisioningReused {
                operation: detail.to_owned(),
            },
            "ProvisioningReused",
        ),
        (
            ServerError::ProfileVersionUnknown { version: 7 },
            "ProfileVersionUnknown",
        ),
        (
            ServerError::ProfileVersionReplaced {
                version: 7,
                latest: 7,
            },
            "ProfileVersionReplaced",
        ),
        (
            ServerError::ProfileNotReviewed { version: 7 },
            "ProfileNotReviewed",
        ),
        (
            ServerError::CertificatesUnavailable {
                reason: detail.to_owned(),
            },
            "CertificatesUnavailable",
        ),
        (
            ServerError::CertificateUnknown {
                serial: detail.to_owned(),
            },
            "CertificateUnknown",
        ),
        (
            ServerError::CertificateReused {
                serial: detail.to_owned(),
            },
            "CertificateReused",
        ),
        (
            ServerError::CertificateWithdrawn {
                serial: detail.to_owned(),
                by: detail.to_owned(),
            },
            "CertificateWithdrawn",
        ),
    ]
}

pub(super) fn server_roles(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::RolesUnavailable {
                reason: detail.to_owned(),
            },
            "RolesUnavailable",
        ),
        (
            ServerError::MembershipUnavailable {
                reason: detail.to_owned(),
            },
            "MembershipUnavailable",
        ),
        (ServerError::RoleUnknown, "RoleUnknown"),
        (ServerError::RoleVersionUnknown, "RoleVersionUnknown"),
        (ServerError::HolderUnknown, "HolderUnknown"),
        (
            ServerError::RoleReused {
                operation: detail.to_owned(),
            },
            "RoleReused",
        ),
        (
            ServerError::RoleHeld {
                holder: detail.to_owned(),
            },
            "RoleHeld",
        ),
        (ServerError::HoldingOver { state: detail }, "HoldingOver"),
        (ServerError::HoldingChanged, "HoldingChanged"),
        (ServerError::LaunchRecordMissing, "LaunchRecordMissing"),
    ]
}

pub(super) fn server_launch(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::OperatorRefused { reason: detail },
            "OperatorRefused",
        ),
        (
            ServerError::AgentSignatureRefused { reason: detail },
            "AgentSignatureRefused",
        ),
        (
            ServerError::AgentNotActive {
                state: detail.to_owned(),
            },
            "AgentNotActive",
        ),
        (
            ServerError::MachineCannotReach {
                host: detail.to_owned(),
            },
            "MachineCannotReach",
        ),
        (ServerError::MachineRetired, "MachineRetired"),
        (ServerError::MachineNotForAgent, "MachineNotForAgent"),
        (ServerError::MachineWithoutRuntime, "MachineWithoutRuntime"),
        (ServerError::WorkingFolderUnnamed, "WorkingFolderUnnamed"),
        (ServerError::MachineWithoutRunner, "MachineWithoutRunner"),
        (
            ServerError::SkillUnknown {
                name: detail.to_owned(),
            },
            "SkillUnknown",
        ),
        (
            ServerError::PolicyUnrepresentable {
                rule: detail.to_owned(),
                reason: detail.to_owned(),
            },
            "PolicyUnrepresentable",
        ),
        (
            ServerError::ModelUnrepresentable {
                harness: detail.to_owned(),
                model: detail.to_owned(),
                reason: detail.to_owned(),
            },
            "ModelUnrepresentable",
        ),
        (
            ServerError::HarnessUndeclared { version: 7 },
            "HarnessUndeclared",
        ),
        (
            ServerError::LaunchUnrenderable {
                reason: detail.to_owned(),
            },
            "LaunchUnrenderable",
        ),
        (
            ServerError::McpCredentialInline {
                server: detail.to_owned(),
                member: detail.to_owned(),
            },
            "McpCredentialInline",
        ),
        (
            ServerError::McpSettingUnrepresentable {
                server: detail.to_owned(),
                member: detail.to_owned(),
                reason: detail.to_owned(),
            },
            "McpSettingUnrepresentable",
        ),
        (
            ServerError::McpHandleUnsupported {
                server: detail.to_owned(),
                member: detail.to_owned(),
                secret: detail.to_owned(),
            },
            "McpHandleUnsupported",
        ),
        (
            ServerError::RuntimeUnavailable {
                reason: detail.to_owned(),
            },
            "RuntimeUnavailable",
        ),
    ]
}

pub(super) fn server_runtime(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (ServerError::RuntimeSessionUnknown, "RuntimeSessionUnknown"),
        (
            ServerError::Call(crate::error_call::CallError::Unknown),
            "CallUnknown",
        ),
        (
            ServerError::Call(crate::error_call::CallError::KeptElsewhere {
                machine: detail.to_owned(),
            }),
            "CallKeptElsewhere",
        ),
        (
            ServerError::Call(crate::error_call::CallError::RecordsUnavailable {
                reason: detail.to_owned(),
            }),
            "CallRecordsUnavailable",
        ),
        (
            ServerError::RuntimeSessionStarted {
                session: detail.to_owned(),
            },
            "RuntimeSessionStarted",
        ),
        (
            ServerError::RuntimeSessionStopped {
                session: detail.to_owned(),
            },
            "RuntimeSessionStopped",
        ),
        (
            ServerError::RuntimeReportReused {
                operation: detail.to_owned(),
            },
            "RuntimeReportReused",
        ),
        (
            ServerError::ServiceAccountsUnavailable {
                reason: detail.to_owned(),
            },
            "ServiceAccountsUnavailable",
        ),
    ]
}

pub(super) fn server_service_accounts(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (ServerError::ServiceAccountUnknown, "ServiceAccountUnknown"),
        (
            ServerError::ServiceAccountReused {
                operation: detail.to_owned(),
            },
            "ServiceAccountReused",
        ),
        (
            ServerError::ServiceAccountRetired {
                account: detail.to_owned(),
            },
            "ServiceAccountRetired",
        ),
        (
            ServerError::ServiceAccountOwnerRetired {
                owner: detail.to_owned(),
            },
            "ServiceAccountOwnerRetired",
        ),
        (
            ServerError::PolicyUnavailable {
                reason: detail.to_owned(),
            },
            "PolicyUnavailable",
        ),
    ]
}

pub(super) fn server_policy_stops(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::AgentHasNoPolicy {
                agent: detail.to_owned(),
            },
            "AgentHasNoPolicy",
        ),
        (
            ServerError::PolicyVersionConflict {
                held: 7,
                expected: 7,
            },
            "PolicyVersionConflict",
        ),
        (
            ServerError::PolicyRefused {
                refusal: "policy:name",
                words: detail.to_owned(),
            },
            "policy:name",
        ),
        (
            ServerError::StopsUnavailable {
                reason: detail.to_owned(),
            },
            "StopsUnavailable",
        ),
        (
            ServerError::StopReused {
                operation: detail.to_owned(),
            },
            "StopReused",
        ),
        (
            ServerError::Cord(CordError::EverythingStopped {
                words: detail.to_owned(),
            }),
            "everything_stopped",
        ),
        (
            ServerError::Cord(CordError::Reused {
                operation: detail.to_owned(),
            }),
            "cord_reused",
        ),
        (ServerError::Cord(CordError::NotPulled), "cord_not_pulled"),
        (
            ServerError::Canvas(crate::error_canvas::CanvasError::Unavailable {
                reason: detail.to_owned(),
            }),
            "CanvasUnavailable",
        ),
        (
            ServerError::Canvas(crate::error_canvas::CanvasError::Refused {
                words: detail.to_owned(),
            }),
            "CanvasRefused",
        ),
        (
            ServerError::Cord(CordError::Unavailable {
                reason: detail.to_owned(),
            }),
            "cord_unavailable",
        ),
        (
            ServerError::ReviewsUnavailable {
                reason: detail.to_owned(),
            },
            "ReviewsUnavailable",
        ),
    ]
}

pub(super) fn server_reviews(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (ServerError::ReviewerOnly, "ReviewerOnly"),
        (
            ServerError::GrantNotDue {
                grant: detail.to_owned(),
                why: detail,
            },
            "GrantNotDue",
        ),
        (
            ServerError::ReviewReused {
                operation: detail.to_owned(),
            },
            "ReviewReused",
        ),
        (
            ServerError::DirectoryUnavailable {
                reason: detail.to_owned(),
            },
            "DirectoryUnavailable",
        ),
        (
            ServerError::SignInProvidersUnavailable {
                reason: detail.to_owned(),
            },
            "SignInProvidersUnavailable",
        ),
    ]
}
