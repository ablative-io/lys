//! Every refusal keeps its protocol name when its explanation changes.

use crate::error_agents::AgentsError;
use std::collections::BTreeSet;

use axum::http::StatusCode;
use lys_identity::grants::{GrantError, RecipientKind};
use lys_identity::{IdentityError, LifecycleState, Transition};

use crate::apps_error::{AppError, Strand};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::error_cord::CordError;
use crate::error_grant_stream::GrantStreamError;
use crate::error_provider::ProviderError;
use crate::error_team::TeamError;
use crate::goals_types::GoalError;

fn server_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    let mut cases = server_access::server_delegated(detail);
    cases.extend(server_access::server_admission(detail));
    cases.extend(server_access::server_sign_in(detail));
    cases.extend(server_access::server_requests(detail));
    cases.extend(server_access::server_network(detail));
    cases.extend(server_runtime::server_profiles(detail));
    cases.extend(server_runtime::server_roles(detail));
    cases.extend(server_runtime::server_launch(detail));
    cases.extend(server_runtime::server_runtime(detail));
    cases.extend(server_runtime::server_service_accounts(detail));
    cases.extend(server_runtime::server_policy_stops(detail));
    cases.extend(server_runtime::server_reviews(detail));
    cases.extend(server_provider::server_providers(detail));
    cases.extend(server_provider::server_runner_grants(detail));
    cases
}

fn identity_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    let mut cases = identity::identity_records(detail);
    cases.extend(identity::identity_log(detail));
    cases.extend(identity::identity_holdings(detail));
    cases
}

fn grant_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    let mut cases = grant::grant_shape(detail);
    cases.extend(grant::grant_lineage(detail));
    cases.extend(grant::grant_standing(detail));
    cases.extend(grant::grant_events(detail));
    cases.extend(grant::grant_storage(detail));
    cases
}

fn app_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    let mut cases = app::app_registration(detail);
    cases.extend(app::app_schema(detail));
    cases.extend(app::app_access(detail));
    cases
}

fn goal_cases(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Agents(AgentsError::Goal(GoalError::Unavailable {
                reason: detail.to_owned(),
            })),
            "goals_unavailable",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::Unknown)),
            "goal_unknown",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::Reused {
                operation: detail.to_owned(),
            })),
            "goal_reused",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::Closed {
                goal: detail.to_owned(),
                standing: detail,
            })),
            "goal_closed",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::NotYourJudgement {
                agent: detail.to_owned(),
            })),
            "not_your_judgement",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::ReminderNeedsDeadline)),
            "reminder_needs_deadline",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::WordsMalformed { why: detail })),
            "goal_words_malformed",
        ),
        (
            ServerError::Agents(AgentsError::Goal(GoalError::EvidenceMissing {
                why: detail,
            })),
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
            ServerError::Agents(AgentsError::Budget(BudgetError::BudgetsUnavailable {
                reason: detail.to_owned(),
            })),
            "BudgetsUnavailable",
        ),
        (
            ServerError::Agents(AgentsError::Budget(BudgetError::ConfigurationUnavailable {
                reason: detail.to_owned(),
            })),
            "ConfigurationUnavailable",
        ),
        (
            ServerError::Agents(AgentsError::Budget(
                BudgetError::ConfigurationVersionConflict {
                    held: 7,
                    expected: 7,
                },
            )),
            "ConfigurationVersionConflict",
        ),
        (
            ServerError::Agents(AgentsError::Budget(BudgetError::BudgetExhausted {
                words: detail.to_owned(),
            })),
            "BudgetExhausted",
        ),
        (
            ServerError::Agents(AgentsError::Budget(BudgetError::BudgetVersionConflict {
                held: 7,
                expected: 7,
            })),
            "BudgetVersionConflict",
        ),
        (
            ServerError::Agents(AgentsError::Budget(BudgetError::BudgetRefused {
                refusal: "budget:name",
                words: detail.to_owned(),
            })),
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
                | "MachineRefused"
        ) {
            assert_eq!(error.status(), StatusCode::FORBIDDEN, "{name}");
        }
    }
}

#[path = "error_names_cases/app_tests.rs"]
mod app;
#[path = "error_names_cases/grant_tests.rs"]
mod grant;
#[path = "error_names_cases/identity_tests.rs"]
mod identity;
#[path = "error_names_cases/server_access_tests.rs"]
mod server_access;
#[path = "error_names_cases/server_provider_tests.rs"]
mod server_provider;
#[path = "error_names_cases/server_runtime_tests.rs"]
mod server_runtime;
