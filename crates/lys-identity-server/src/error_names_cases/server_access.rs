//! Refusal fixtures preserve their protocol names across explanation changes.
use super::*;

pub(super) fn server_delegated(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Team(TeamError::Unavailable {
                reason: detail.to_owned(),
            }),
            "TeamsUnavailable",
        ),
        (
            ServerError::Agents(AgentsError::Budget(BudgetError::BudgetsUnavailable {
                reason: detail.to_owned(),
            })),
            "BudgetsUnavailable",
        ),
        (
            ServerError::AuthorityUnavailable {
                reason: detail.to_owned(),
            },
            "authority_unavailable",
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
        (
            ServerError::Agents(AgentsError::Goal(GoalError::Unknown)),
            "goal_unknown",
        ),
        (
            ServerError::Inactive {
                identity: detail.to_owned(),
                state: LifecycleState::Active,
            },
            "inactive",
        ),
    ]
}

pub(super) fn server_admission(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
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
    ]
}

pub(super) fn server_sign_in(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
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
    ]
}

pub(super) fn server_requests(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
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
    ]
}

pub(super) fn server_network(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
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
    ]
}
