//! Every refusal keeps its protocol name when its explanation changes.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use lys_identity::grants::{GrantError, RecipientKind};
use lys_identity::{IdentityError, LifecycleState, Transition};

use crate::apps_error::{AppError, Strand};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::error_cord::CordError;
use crate::error_provider::ProviderError;
use crate::error_team::TeamError;
use crate::goals_types::GoalError;

fn server_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Team(TeamError::Unavailable {
                reason: detail.to_owned(),
            }),
            "TeamsUnavailable",
        ),
        (
            ServerError::Budget(BudgetError::BudgetsUnavailable {
                reason: detail.to_owned(),
            }),
            "BudgetsUnavailable",
        ),
        (
            ServerError::HarnessCatalogueUnreadable {
                file: detail.to_owned(),
                reason: detail.to_owned(),
            },
            "harness_catalogue_unreadable",
        ),
        (
            ServerError::Identity(IdentityError::IdentifierMalformed {
                kind: detail,
                text: detail.to_owned(),
            }),
            "IdentifierMalformed",
        ),
        (
            ServerError::Grant(GrantError::GrantIdMalformed {
                text: detail.to_owned(),
            }),
            "GrantIdMalformed",
        ),
        (
            ServerError::App(AppError::AppIdInvalid {
                id: detail.to_owned(),
                reason: detail,
            }),
            "app_id_invalid",
        ),
        (ServerError::Goal(GoalError::Unknown), "goal_unknown"),
        (
            ServerError::Inactive {
                identity: detail.to_owned(),
                state: LifecycleState::Active,
            },
            "inactive",
        ),
        (ServerError::NotSignedIn, "NotSignedIn"),
        (ServerError::NotAdmitted { reason: detail }, "NotAdmitted"),
        (ServerError::NoPerson, "NoPerson"),
        (
            ServerError::BootstrapInterrupted {
                reason: detail.to_owned(),
            },
            "BootstrapInterrupted",
        ),
        (ServerError::SetupRequired, "SetupRequired"),
        (ServerError::AgentNotVisible, "AgentNotVisible"),
        (ServerError::GrantNotVisible, "GrantNotVisible"),
        (
            ServerError::Withheld {
                refusal: "withheld:name".to_owned(),
            },
            "withheld:name",
        ),
        (ServerError::SessionUnknown, "SessionUnknown"),
        (ServerError::SignInStateUnknown, "SignInStateUnknown"),
        (ServerError::SignInRefused, "SignInRefused"),
        (
            ServerError::IssuerChallengeExpired,
            "IssuerChallengeExpired",
        ),
        (
            ServerError::IssuerRefused {
                status: 403,
                error: detail.to_owned(),
            },
            "IssuerRefused",
        ),
        (ServerError::SignInThrottled, "SignInThrottled"),
        (ServerError::RegistrationThrottled, "RegistrationThrottled"),
        (
            ServerError::SecondFactorUnsupported,
            "SecondFactorUnsupported",
        ),
        (ServerError::SetupClosed, "SetupClosed"),
        (ServerError::SetupCodeRefused, "SetupCodeRefused"),
        (
            ServerError::SetupUnavailable {
                reason: detail.to_owned(),
            },
            "SetupUnavailable",
        ),
        (
            ServerError::AccountRefused {
                reason: detail.to_owned(),
            },
            "AccountRefused",
        ),
        (
            ServerError::SignInFailed {
                reason: detail.to_owned(),
            },
            "SignInFailed",
        ),
        (
            ServerError::ConfigInvalid {
                reason: detail.to_owned(),
            },
            "ConfigInvalid",
        ),
        (
            ServerError::RequestMalformed {
                reason: detail.to_owned(),
            },
            "RequestMalformed",
        ),
        (ServerError::BodyTooLarge, "BodyTooLarge"),
        (
            ServerError::SecretsUnavailable {
                reason: detail.to_owned(),
            },
            "SecretsUnavailable",
        ),
        (
            ServerError::SecretsRefused {
                status: StatusCode::BAD_REQUEST,
                refusal: "secrets:name".to_owned(),
                reason: detail.to_owned(),
            },
            "secrets:name",
        ),
        (
            ServerError::RequestsUnavailable {
                reason: detail.to_owned(),
            },
            "RequestsUnavailable",
        ),
        (
            ServerError::McpRequestsUnavailable {
                reason: detail.to_owned(),
            },
            "McpRequestsUnavailable",
        ),
        (
            ServerError::McpServerUnknown {
                server: detail.to_owned(),
            },
            "mcp_server_unknown",
        ),
        (
            ServerError::McpServerHeld {
                server: detail.to_owned(),
            },
            "mcp_server_held",
        ),
        (
            ServerError::McpBeyondRemit {
                approver: detail.to_owned(),
                agent: detail.to_owned(),
                server: detail.to_owned(),
            },
            "mcp_beyond_remit",
        ),
        (ServerError::RequestUnknown, "RequestUnknown"),
        (
            ServerError::RequestDecided {
                request: detail.to_owned(),
            },
            "RequestDecided",
        ),
        (
            ServerError::RequestHeld {
                request: detail.to_owned(),
                by: detail.to_owned(),
            },
            "RequestHeld",
        ),
        (
            ServerError::RequestReused {
                request: detail.to_owned(),
            },
            "RequestReused",
        ),
        (
            ServerError::NetworkUnavailable {
                reason: detail.to_owned(),
            },
            "NetworkUnavailable",
        ),
        (ServerError::LoginUnbound, "LoginUnbound"),
        (ServerError::MachineUnknown, "MachineUnknown"),
        (
            ServerError::Machine(crate::error_machine::MachineError::Reused {
                machine: detail.to_owned(),
            }),
            "MachineReused",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::TeamReused {
                operation: detail.to_owned(),
            }),
            "MachineTeamReused",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::AgentsReused {
                operation: detail.to_owned(),
            }),
            "MachineAgentsReused",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::JoinOperationReused {
                operation: detail.to_owned(),
            }),
            "RunnerJoinOperationReused",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::JoinUnreachable {
                reason: detail.to_owned(),
            }),
            "RunnerJoinUnreachable",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::JoinRefused),
            "RunnerJoinRefused",
        ),
        (
            ServerError::SessionsUnavailable {
                reason: detail.to_owned(),
            },
            "SessionsUnavailable",
        ),
        (
            ServerError::MemoryUnavailable {
                reason: detail.to_owned(),
            },
            "MemoryUnavailable",
        ),
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
        (
            ServerError::RolesUnavailable {
                reason: detail.to_owned(),
            },
            "RolesUnavailable",
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
        (
            ServerError::Provider(ProviderError::Unavailable {
                reason: detail.to_owned(),
            }),
            "ProviderUnavailable",
        ),
        (
            ServerError::Provider(ProviderError::ClientUnknown),
            "ClientUnknown",
        ),
        (
            ServerError::Provider(ProviderError::RedirectUnregistered),
            "RedirectUnregistered",
        ),
        (
            ServerError::Provider(ProviderError::ScopeUnknown {
                scope: detail.to_owned(),
            }),
            "ScopeUnknown",
        ),
        (
            ServerError::Provider(ProviderError::ScopeNotGranted {
                scope: detail.to_owned(),
                app: detail.to_owned(),
            }),
            "ScopeNotGranted",
        ),
        (
            ServerError::Provider(ProviderError::CodeUnknown),
            "CodeUnknown",
        ),
        (ServerError::Provider(ProviderError::CodeUsed), "CodeUsed"),
        (
            ServerError::Provider(ProviderError::CodeExpired),
            "CodeExpired",
        ),
        (
            ServerError::Provider(ProviderError::VerifierWrong),
            "VerifierWrong",
        ),
        (
            ServerError::Provider(ProviderError::TokenUnknown),
            "TokenUnknown",
        ),
        (
            ServerError::ProviderRefused {
                provider: detail,
                status: 7,
                reason: detail.to_owned(),
            },
            "ProviderRefused",
        ),
        (
            ServerError::SignInProvidersRefused {
                status: 7,
                reason: detail.to_owned(),
            },
            "SignInProvidersRefused",
        ),
        (
            ServerError::NotPermitted {
                reason: detail.to_owned(),
            },
            "not_permitted",
        ),
        (
            ServerError::NoLiveSession {
                agent: detail.to_owned(),
            },
            "no_live_session",
        ),
        (
            ServerError::RunnerAbsent {
                machine: detail.to_owned(),
            },
            "runner_absent",
        ),
        (
            ServerError::Runner {
                refusal: "runner:name".to_owned(),
                words: detail.to_owned(),
            },
            "runner:name",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::DialRefused {
                reason: detail.to_owned(),
            }),
            "runner_dial_refused",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::DialStale {
                reason: detail.to_owned(),
            }),
            "runner_dial_stale",
        ),
    ]
}

fn identity_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Identity(IdentityError::DirectorySnapshotUnmigrated {
                found: 7,
                expected: 7,
            }),
            "DirectorySnapshotUnmigrated",
        ),
        (
            ServerError::Identity(IdentityError::AnswersToUnknown {
                identity: detail.to_owned(),
            }),
            "AnswersToUnknown",
        ),
        (
            ServerError::Identity(IdentityError::AnswersToInactive {
                identity: detail.to_owned(),
                state: LifecycleState::Active,
            }),
            "AnswersToInactive",
        ),
        (
            ServerError::Identity(IdentityError::AnswersToCycle {
                chain: vec![detail.to_owned()],
            }),
            "AnswersToCycle",
        ),
        (
            ServerError::Identity(IdentityError::NoAccountablePerson {
                chain: vec![detail.to_owned()],
            }),
            "NoAccountablePerson",
        ),
        (
            ServerError::Identity(IdentityError::AlreadyBootstrapped {
                person: detail.to_owned(),
            }),
            "AlreadyBootstrapped",
        ),
        (
            ServerError::Identity(IdentityError::RandomSourceUnavailable {
                reason: detail.to_owned(),
            }),
            "RandomSourceUnavailable",
        ),
        (
            ServerError::Identity(IdentityError::IdentifierMalformed {
                kind: detail,
                text: detail.to_owned(),
            }),
            "IdentifierMalformed",
        ),
        (
            ServerError::Identity(IdentityError::BindingMalformed { reason: detail }),
            "BindingMalformed",
        ),
        (
            ServerError::Identity(IdentityError::ProfileInvalid { reason: detail }),
            "ProfileInvalid",
        ),
        (
            ServerError::Identity(IdentityError::ReasonRequired {
                transition: Transition::Suspend,
            }),
            "ReasonRequired",
        ),
        (
            ServerError::Identity(IdentityError::TransitionRefused {
                transition: Transition::Suspend,
                from: LifecycleState::Active,
            }),
            "TransitionRefused",
        ),
        (
            ServerError::Identity(IdentityError::ChangeMismatch { reason: detail }),
            "ChangeMismatch",
        ),
        (
            ServerError::Identity(IdentityError::EventMalformed { reason: detail }),
            "EventMalformed",
        ),
        (
            ServerError::Identity(IdentityError::EventNotCanonical),
            "EventNotCanonical",
        ),
        (
            ServerError::Identity(IdentityError::VersionUnsupported { version: 7 }),
            "VersionUnsupported",
        ),
        (
            ServerError::Identity(IdentityError::SignerMismatch),
            "SignerMismatch",
        ),
        (
            ServerError::Identity(IdentityError::SignatureInvalid),
            "SignatureInvalid",
        ),
        (
            ServerError::Identity(IdentityError::LogUnavailable {
                reason: detail.to_owned(),
            }),
            "LogUnavailable",
        ),
        (
            ServerError::Identity(IdentityError::LeafNotAnEvent {
                index: 7,
                reason: detail.to_owned(),
            }),
            "LeafNotAnEvent",
        ),
        (
            ServerError::Identity(IdentityError::AppendUncertain { index: 7 }),
            "AppendUncertain",
        ),
        (
            ServerError::Identity(IdentityError::AppendRefused {
                reason: detail.to_owned(),
            }),
            "AppendRefused",
        ),
        (
            ServerError::Identity(IdentityError::IdentityUnknown {
                identity: detail.to_owned(),
            }),
            "IdentityUnknown",
        ),
        (
            ServerError::Identity(IdentityError::AlreadyRegistered {
                identity: detail.to_owned(),
            }),
            "AlreadyRegistered",
        ),
        (
            ServerError::Identity(IdentityError::BindingTaken {
                issuer: detail.to_owned(),
                subject: detail.to_owned(),
                person: detail.to_owned(),
            }),
            "BindingTaken",
        ),
        (
            ServerError::Identity(IdentityError::StateMismatch {
                identity: detail.to_owned(),
                recorded: LifecycleState::Active,
                from: LifecycleState::Active,
            }),
            "StateMismatch",
        ),
        (
            ServerError::Identity(IdentityError::OperationReused {
                operation: detail.to_owned(),
            }),
            "OperationReused",
        ),
        (
            ServerError::Identity(IdentityError::LinkSourceSeen {
                source_operation_id: detail.to_owned(),
            }),
            "LinkSourceSeen",
        ),
        (
            ServerError::Identity(IdentityError::ReceiptInvalid { reason: detail }),
            "ReceiptInvalid",
        ),
        (
            ServerError::Identity(IdentityError::KeyUnavailable {
                reason: detail.to_owned(),
            }),
            "KeyUnavailable",
        ),
    ]
}

fn grant_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Grant(GrantError::GrantIdMalformed {
                text: detail.to_owned(),
            }),
            "GrantIdMalformed",
        ),
        (
            ServerError::Grant(GrantError::TokenInvalid {
                kind: detail,
                text: detail.to_owned(),
                max: 64,
            }),
            "TokenInvalid",
        ),
        (
            ServerError::Grant(GrantError::ModelInvalid { reason: detail }),
            "ModelInvalid",
        ),
        (
            ServerError::Grant(GrantError::RelationUnknown {
                relation: detail.to_owned(),
                model_version: 7,
            }),
            "RelationUnknown",
        ),
        (
            ServerError::Grant(GrantError::ActionsOutside {
                relation: detail.to_owned(),
                outside: detail.to_owned(),
                model_version: 7,
            }),
            "ActionsOutside",
        ),
        (
            ServerError::Grant(GrantError::AuthorityAbsent { member: detail }),
            "AuthorityAbsent",
        ),
        (
            ServerError::Grant(GrantError::PassOnOutside),
            "PassOnOutside",
        ),
        (
            ServerError::Grant(GrantError::WindowInvalid { reason: detail }),
            "WindowInvalid",
        ),
        (
            ServerError::Grant(GrantError::LineageMalformed { reason: detail }),
            "LineageMalformed",
        ),
        (
            ServerError::Grant(GrantError::MemberUnknown { key: 7 }),
            "MemberUnknown",
        ),
        (
            ServerError::Grant(GrantError::MemberMissing { member: detail }),
            "MemberMissing",
        ),
        (
            ServerError::Grant(GrantError::RecipientKindUnknown { code: 7 }),
            "RecipientKindUnknown",
        ),
        (
            ServerError::Grant(GrantError::GrantMalformed { reason: detail }),
            "GrantMalformed",
        ),
        (
            ServerError::Grant(GrantError::GrantNotCanonical),
            "GrantNotCanonical",
        ),
        (
            ServerError::Grant(GrantError::SourceUnknown {
                grant: detail.to_owned(),
            }),
            "SourceUnknown",
        ),
        (
            ServerError::Grant(GrantError::LineageCycle {
                grant: detail.to_owned(),
            }),
            "LineageCycle",
        ),
        (
            ServerError::Grant(GrantError::IssuerNotHolder {
                grant: detail.to_owned(),
                source_grant: detail.to_owned(),
            }),
            "IssuerNotHolder",
        ),
        (
            ServerError::Grant(GrantError::NotHolder {
                caller: detail.to_owned(),
                grant: detail.to_owned(),
            }),
            "NotHolder",
        ),
        (
            ServerError::Grant(GrantError::ResourceOutside {
                requested: detail.to_owned(),
                source_grant: detail.to_owned(),
            }),
            "ResourceOutside",
        ),
        (
            ServerError::Grant(GrantError::UseOnly {
                grant: detail.to_owned(),
            }),
            "UseOnly",
        ),
        (
            ServerError::Grant(GrantError::RecipientRefused {
                kind: RecipientKind::Agent,
                grant: detail.to_owned(),
            }),
            "RecipientRefused",
        ),
        (
            ServerError::Grant(GrantError::WithheldFromAgents {
                relation: detail.to_owned(),
                withheld: detail.to_owned(),
            }),
            "WithheldFromAgents",
        ),
        (
            ServerError::Grant(GrantError::PassOnBeyondSource {
                source_grant: detail.to_owned(),
            }),
            "PassOnBeyondSource",
        ),
        (
            ServerError::Grant(GrantError::ExpiryBeyondSource {
                requested: detail.to_owned(),
                source_grant: detail.to_owned(),
                source_ends: 7,
            }),
            "ExpiryBeyondSource",
        ),
        (
            ServerError::Grant(GrantError::ResponsibleMismatch {
                identity: detail.to_owned(),
                named: detail.to_owned(),
                recorded: detail.to_owned(),
            }),
            "ResponsibleMismatch",
        ),
        (
            ServerError::Grant(GrantError::IdentityNotActive {
                identity: detail.to_owned(),
                state: LifecycleState::Active,
            }),
            "IdentityNotActive",
        ),
        (
            ServerError::Grant(GrantError::Revoked {
                grant: detail.to_owned(),
            }),
            "Revoked",
        ),
        (
            ServerError::Grant(GrantError::Expired {
                grant: detail.to_owned(),
                ended_at: 7,
            }),
            "Expired",
        ),
        (
            ServerError::Grant(GrantError::NotStarted {
                grant: detail.to_owned(),
                starts_at: 7,
            }),
            "NotStarted",
        ),
        (
            ServerError::Grant(GrantError::RootAuthorityRefused {
                caller: detail.to_owned(),
            }),
            "RootAuthorityRefused",
        ),
        (
            ServerError::Grant(GrantError::RevokeRefused {
                caller: detail.to_owned(),
                grant: detail.to_owned(),
            }),
            "RevokeRefused",
        ),
        (
            ServerError::Grant(GrantError::AlreadyRevoked {
                grant: detail.to_owned(),
            }),
            "AlreadyRevoked",
        ),
        (
            ServerError::Grant(GrantError::GrantExists {
                grant: detail.to_owned(),
            }),
            "GrantExists",
        ),
        (
            ServerError::Grant(GrantError::NotHeld {
                identity: detail.to_owned(),
                resource: detail.to_owned(),
                action: detail.to_owned(),
            }),
            "NotHeld",
        ),
        (
            ServerError::Grant(GrantError::OperationReused {
                operation: detail.to_owned(),
            }),
            "OperationReused",
        ),
        (
            ServerError::Grant(GrantError::EventMismatch { reason: detail }),
            "EventMismatch",
        ),
        (
            ServerError::Grant(GrantError::GrantUnknown {
                grant: detail.to_owned(),
            }),
            "GrantUnknown",
        ),
        (
            ServerError::Grant(GrantError::EnvelopeMismatch {
                reason: detail.to_owned(),
            }),
            "EnvelopeMismatch",
        ),
        (
            ServerError::Grant(GrantError::EventMalformed { reason: detail }),
            "EventMalformed",
        ),
        (
            ServerError::Grant(GrantError::EventNotCanonical),
            "EventNotCanonical",
        ),
        (
            ServerError::Grant(GrantError::VersionUnsupported { version: 7 }),
            "VersionUnsupported",
        ),
        (
            ServerError::Grant(GrantError::SignerMismatch),
            "SignerMismatch",
        ),
        (
            ServerError::Grant(GrantError::SignatureInvalid),
            "SignatureInvalid",
        ),
        (
            ServerError::Grant(GrantError::ReceiptInvalid { reason: detail }),
            "ReceiptInvalid",
        ),
        (
            ServerError::Grant(GrantError::LogUnavailable {
                reason: detail.to_owned(),
            }),
            "LogUnavailable",
        ),
        (
            ServerError::Grant(GrantError::LeafNotAnEvent {
                index: 7,
                reason: detail.to_owned(),
            }),
            "LeafNotAnEvent",
        ),
        (
            ServerError::Grant(GrantError::AppendRefused {
                reason: detail.to_owned(),
            }),
            "AppendRefused",
        ),
        (
            ServerError::Grant(GrantError::OperationUnresolved {
                operation: detail.to_owned(),
                grant: detail.to_owned(),
            }),
            "OperationUnresolved",
        ),
        (
            ServerError::Grant(GrantError::ProjectionPending {
                operation: detail.to_owned(),
                grant: detail.to_owned(),
                index: 7,
            }),
            "ProjectionPending",
        ),
        (
            ServerError::Grant(GrantError::StaleDecision {
                required: 7,
                projected: 7,
            }),
            "StaleDecision",
        ),
        (
            ServerError::Grant(GrantError::PermissionEngineUnavailable {
                reason: detail.to_owned(),
            }),
            "PermissionEngineUnavailable",
        ),
        (
            ServerError::Grant(GrantError::PermissionAbsent {
                grant: detail.to_owned(),
            }),
            "PermissionAbsent",
        ),
        (
            ServerError::Grant(GrantError::ModeHeld {
                grant: detail.to_owned(),
                mode: "by_draft",
            }),
            "ModeHeld",
        ),
        (
            ServerError::Grant(GrantError::Identity(IdentityError::IdentifierMalformed {
                kind: detail,
                text: detail.to_owned(),
            })),
            "IdentifierMalformed",
        ),
    ]
}

fn app_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::App(AppError::AppIdInvalid {
                id: detail.to_owned(),
                reason: detail,
            }),
            "app_id_invalid",
        ),
        (
            ServerError::App(AppError::SchemaInvalid {
                pointer: detail.to_owned(),
                reason: detail.to_owned(),
            }),
            "schema_invalid",
        ),
        (
            ServerError::App(AppError::AppExists {
                app: detail.to_owned(),
            }),
            "app_exists",
        ),
        (
            ServerError::App(AppError::AppUnknown {
                app: detail.to_owned(),
            }),
            "app_unknown",
        ),
        (
            ServerError::App(AppError::AppNotApproved {
                app: detail.to_owned(),
            }),
            "app_not_approved",
        ),
        (
            ServerError::App(AppError::AppRetired {
                app: detail.to_owned(),
            }),
            "app_retired",
        ),
        (
            ServerError::App(AppError::AppIsLys { reason: detail }),
            "app_is_lys",
        ),
        (
            ServerError::App(AppError::AppDecided {
                app: detail.to_owned(),
            }),
            "app_decided",
        ),
        (
            ServerError::App(AppError::AppOperationReused {
                operation: detail.to_owned(),
            }),
            "app_operation_reused",
        ),
        (
            ServerError::App(AppError::ConnectorNeedsAPerson {
                login: detail.to_owned(),
            }),
            "connector_needs_a_person",
        ),
        (
            ServerError::App(AppError::ConnectorExists {
                app: detail.to_owned(),
            }),
            "connector_exists",
        ),
        (
            ServerError::App(AppError::KindNotRegistered {
                kind: detail.to_owned(),
            }),
            "kind_not_registered",
        ),
        (
            ServerError::App(AppError::ActionNotDeclared {
                kind: detail.to_owned(),
                action: detail.to_owned(),
            }),
            "action_not_declared",
        ),
        (
            ServerError::App(AppError::NotYourApp {
                kind: detail.to_owned(),
                owner: detail.to_owned(),
                acting_for: detail.to_owned(),
            }),
            "not_your_app",
        ),
        (
            ServerError::App(AppError::SchemaVersionMoved {
                replaces: 7,
                current: 7,
            }),
            "schema_version_moved",
        ),
        (
            ServerError::App(AppError::SchemaChangeStrandsGrants {
                stranded: vec![Strand {
                    kind: detail.to_owned(),
                    relation: detail.to_owned(),
                    count: 7,
                }],
            }),
            "schema_change_strands_grants",
        ),
        (
            ServerError::App(AppError::SchemaVersionUnknown {
                app: detail.to_owned(),
                version: 7,
            }),
            "schema_version_unknown",
        ),
        (
            ServerError::App(AppError::SchemaChangePending {
                app: detail.to_owned(),
            }),
            "schema_change_pending",
        ),
        (
            ServerError::App(AppError::RedirectInvalid {
                address: detail.to_owned(),
                reason: detail,
            }),
            "redirect_invalid",
        ),
        (
            ServerError::App(AppError::PlacementInvalid {
                reason: detail.to_owned(),
            }),
            "placement_invalid",
        ),
        (
            ServerError::App(AppError::CredentialRefused { reason: detail }),
            "credential_refused",
        ),
        (ServerError::App(AppError::BenchUnknown), "bench_unknown"),
        (
            ServerError::App(AppError::AppsUnavailable {
                reason: detail.to_owned(),
            }),
            "apps_unavailable",
        ),
    ]
}

fn goal_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Goal(GoalError::Unavailable {
                reason: detail.to_owned(),
            }),
            "goals_unavailable",
        ),
        (ServerError::Goal(GoalError::Unknown), "goal_unknown"),
        (
            ServerError::Goal(GoalError::Reused {
                operation: detail.to_owned(),
            }),
            "goal_reused",
        ),
        (
            ServerError::Goal(GoalError::Closed {
                goal: detail.to_owned(),
                standing: detail,
            }),
            "goal_closed",
        ),
        (
            ServerError::Goal(GoalError::NotYourJudgement {
                agent: detail.to_owned(),
            }),
            "not_your_judgement",
        ),
        (
            ServerError::Goal(GoalError::ReminderNeedsDeadline),
            "reminder_needs_deadline",
        ),
        (
            ServerError::Goal(GoalError::WordsMalformed { why: detail }),
            "goal_words_malformed",
        ),
        (
            ServerError::Goal(GoalError::EvidenceMissing { why: detail }),
            "evidence_missing",
        ),
    ]
}

fn team_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Team(TeamError::Unavailable {
                reason: detail.to_owned(),
            }),
            "TeamsUnavailable",
        ),
        (ServerError::Team(TeamError::Unknown), "TeamUnknown"),
        (
            ServerError::Team(TeamError::Reused {
                operation: detail.to_owned(),
            }),
            "TeamReused",
        ),
        (
            ServerError::Team(TeamError::Retired {
                team: detail.to_owned(),
            }),
            "TeamRetired",
        ),
        (ServerError::Team(TeamError::MemberHeld), "TeamMemberHeld"),
        (
            ServerError::Team(TeamError::MemberAbsent),
            "TeamMemberAbsent",
        ),
        (
            ServerError::Team(TeamError::ParentCycle {
                team: detail.to_owned(),
                parent: detail.to_owned(),
            }),
            "team_parent_cycle",
        ),
        (
            ServerError::Team(TeamError::LeadNotMember {
                team: detail.to_owned(),
                lead: detail.to_owned(),
            }),
            "team_lead_not_member",
        ),
        (
            ServerError::Team(TeamError::MemberUnknown),
            "TeamMemberUnknown",
        ),
    ]
}

fn budget_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Budget(BudgetError::BudgetsUnavailable {
                reason: detail.to_owned(),
            }),
            "BudgetsUnavailable",
        ),
        (
            ServerError::Budget(BudgetError::ConfigurationUnavailable {
                reason: detail.to_owned(),
            }),
            "ConfigurationUnavailable",
        ),
        (
            ServerError::Budget(BudgetError::ConfigurationVersionConflict {
                held: 7,
                expected: 7,
            }),
            "ConfigurationVersionConflict",
        ),
        (
            ServerError::Budget(BudgetError::BudgetExhausted {
                words: detail.to_owned(),
            }),
            "BudgetExhausted",
        ),
        (
            ServerError::Budget(BudgetError::BudgetVersionConflict {
                held: 7,
                expected: 7,
            }),
            "BudgetVersionConflict",
        ),
        (
            ServerError::Budget(BudgetError::BudgetRefused {
                refusal: "budget:name",
                words: detail.to_owned(),
            }),
            "budget:name",
        ),
    ]
}

fn assert_names(cases: Vec<(ServerError, &'static str)>) {
    let mut names = BTreeSet::new();
    for (error, expected) in cases {
        assert!(names.insert(expected), "duplicate refusal name: {expected}");
        assert_eq!(error.name(), expected, "refusal: {error:?}");
    }
}

#[test]
fn refusal_names_are_stable_unique_and_preserve_supplied_names() {
    for detail in [
        "original explanation",
        "changed: explanation: with punctuation",
    ] {
        assert_names(server_cases(detail));
        assert_names(identity_cases(detail));
        assert_names(grant_cases(detail));
        assert_names(app_cases(detail));
        assert_names(goal_cases(detail));
        assert_names(team_cases(detail));
        assert_names(budget_cases(detail));
    }
}

#[test]
fn grant_authority_refusals_keep_their_forbidden_status() {
    let errors = grant_cases("authority: refused");
    for (error, name) in errors {
        if matches!(
            name,
            "RelationUnknown"
                | "ActionsOutside"
                | "LineageCycle"
                | "IssuerNotHolder"
                | "NotHolder"
                | "ResourceOutside"
                | "UseOnly"
                | "RecipientRefused"
                | "WithheldFromAgents"
                | "PassOnBeyondSource"
                | "ExpiryBeyondSource"
                | "ResponsibleMismatch"
                | "IdentityNotActive"
                | "Revoked"
                | "Expired"
                | "NotStarted"
                | "RootAuthorityRefused"
                | "RevokeRefused"
                | "NotHeld"
                | "EnvelopeMismatch"
                | "PermissionAbsent"
                | "ModeHeld"
        ) {
            assert_eq!(error.status(), StatusCode::FORBIDDEN, "{name}");
        }
    }
}
