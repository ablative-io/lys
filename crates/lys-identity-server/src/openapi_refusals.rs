//! The refusal sets the route table is written with: each route's entry
//! names the sets it answers with and any refusal of its own, and the
//! document lists their union. `tests/openapi.rs` fails on a listed refusal
//! no test produces, and the contract harness fails any test that meets a
//! refusal its route does not list, so the lists are held complete from
//! both sides.

/// Signed in.
pub(crate) const SIGNED: &[&str] = &["NotSignedIn"];
/// Signed in, with a body.
pub(crate) const SIGNED_BODY: &[&str] = &["NotSignedIn", "RequestMalformed"];
/// The administrator.
pub(crate) const ADMIN: &[&str] = &["NotSignedIn", "NotAdmitted"];
/// The administrator, with a body.
pub(crate) const ADMIN_BODY: &[&str] = &["NotSignedIn", "NotAdmitted", "RequestMalformed"];
/// An allowance operation reused for different words.
pub(crate) const MACHINE_AGENTS: &[&str] = &["MachineAgentsReused"];
/// A reporting edge must resolve to an active accountable person.
pub(crate) const REPORTING: &[&str] = &[
    "IdentifierMalformed",
    "AnswersToUnknown",
    "AnswersToInactive",
    "AnswersToCycle",
    "NoAccountablePerson",
    "OperationReused",
    "IdentityUnknown",
];
/// A signed-in person bound to a person.
pub(crate) const PERSON: &[&str] = &["NotSignedIn", "NoPerson"];
/// A grant the caller may see.
pub(crate) const GRANT_READ: &[&str] = &["NotSignedIn", "GrantNotVisible"];
/// A grant made on a kind an approved app declares.
pub(crate) const GRANT_MADE: &[&str] = &[
    "NotSignedIn",
    "RequestMalformed",
    "kind_not_registered",
    "app_not_approved",
    "app_retired",
];
/// A grant change the log recorded that the permission engine has not yet
/// taken, or whose recording is not yet known.
pub(crate) const RECORDED: &[&str] = &["ProjectionPending", "OperationUnresolved"];
/// A question on a kind an approved app declares.
pub(crate) const GRANT_ASKED: &[&str] = &[
    "NotSignedIn",
    "RequestMalformed",
    "kind_not_registered",
    "app_not_approved",
    "app_retired",
    "action_not_declared",
];
/// A decision the permission engine or the grant log could not make.
pub(crate) const UNANSWERED: &[&str] = &[
    "PermissionEngineUnavailable",
    "ProjectionPending",
    "OperationUnresolved",
    "StaleDecision",
];
/// An agent's signed request.
pub(crate) const AGENT: &[&str] = &["AgentSignatureRefused", "CertificatesUnavailable"];
/// A registration.
pub(crate) const REGISTER: &[&str] = &[
    "NotHeld",
    "ServiceAccountUnknown",
    "NotAdmitted",
    "RequestMalformed",
    "app_id_invalid",
    "schema_invalid",
    "app_exists",
    "redirect_invalid",
    "credential_refused",
    "ServiceAccountsUnavailable",
    "app_operation_reused",
];
/// An app the caller may see.
pub(crate) const APP_READ: &[&str] = &["NotSignedIn", "app_unknown", "credential_refused"];
/// An app's own sign-in.
pub(crate) const APP_SELF: &[&str] = &[
    "credential_refused",
    "ServiceAccountsUnavailable",
    "app_retired",
];
/// A decision on a registration.
pub(crate) const DECIDE: &[&str] = &[
    "NotAdmitted",
    "app_unknown",
    "app_decided",
    "app_operation_reused",
];
/// A retirement.
pub(crate) const RETIRE: &[&str] = &["NotAdmitted", "app_unknown", "app_is_lys", "app_retired"];
/// A schema change.
pub(crate) const CHANGE: &[&str] = &[
    "NotAdmitted",
    "app_unknown",
    "app_not_approved",
    "schema_invalid",
    "schema_version_moved",
    "schema_change_strands_grants",
    "schema_change_pending",
];
/// A schema change's dry run.
pub(crate) const CHECK: &[&str] = &["NotAdmitted", "app_unknown", "schema_invalid"];
/// A schema version.
pub(crate) const VERSION: &[&str] = &["app_unknown", "schema_version_unknown"];
/// A decision on a waiting schema change.
pub(crate) const CHANGE_DECIDE: &[&str] =
    &["NotAdmitted", "app_decided", "schema_change_strands_grants"];
/// A placement.
pub(crate) const PLACE: &[&str] = &[
    "NotAdmitted",
    "not_your_app",
    "placement_invalid",
    "kind_not_registered",
];
/// A batch of checks.
pub(crate) const BATCH: &[&str] = &[
    "NotAdmitted",
    "RequestMalformed",
    "batch_too_large",
    "credential_refused",
    "ServiceAccountsUnavailable",
];
/// The ids a subject may act on.
pub(crate) const WHICH: &[&str] = &[
    "NotAdmitted",
    "RequestMalformed",
    "not_your_app",
    "kind_not_registered",
];

/// A profile review that can approve only the latest recorded version.
pub(crate) const PROFILE_REVIEW: &[&str] = &["ProfileVersionReplaced"];

/// A start and the status read that settles an already-held session.
pub(crate) const START_RUNNER: &[&str] = &[
    "AgentHasNoPolicy",
    "AgentNotActive",
    "AgentNotVisible",
    "CertificatesUnavailable",
    "PolicyUnavailable",
    "MachineWithoutRunner",
    "runner_request_unsigned",
    "runner_request_malformed",
    "runner_protocol_mismatch",
    "runner_request_replayed",
    "runner_request_misaddressed",
    "runner_unreachable",
    "runner_reply_malformed",
    "runner_state_unavailable",
    "session_invalid",
    "runner_stopping",
    "spawn_failed",
    "size_invalid",
    "rotation_invalid",
    "launch_config_refused",
    "policy_invalid",
    "policy_rule_duplicate",
    "policy_target_ambiguous",
    "policy_target_uninspectable",
    "policy_digest_mismatch",
    "session_unknown",
];

/// An operator restart shares the start and runner's named refusals.
pub(crate) const RESTART: &[&str] = &[
    "AgentHasNoPolicy",
    "CertificatesUnavailable",
    "PolicyUnavailable",
    "AgentNotVisible",
    "AgentNotActive",
    "NotAdmitted",
    "NoPerson",
    "RuntimeSessionUnknown",
    "RuntimeReportReused",
    "RuntimeUnavailable",
    "ProvisioningUnavailable",
    "profile_version_unreviewed",
    "MachineCannotReach",
    "MachineNotForAgent",
    "MachineRetired",
    "MachineUnknown",
    "MachineWithoutRuntime",
    "MachineWithoutRunner",
    "WorkingFolderUnnamed",
    "HarnessUndeclared",
    "LaunchUnrenderable",
    "McpHandleUnsupported",
    "McpSettingUnrepresentable",
    "ModelUnrepresentable",
    "PolicyUnrepresentable",
    "SkillUnknown",
    "SecretsUnavailable",
    "runner_absent",
    "runner_protocol_mismatch",
    "runner_unreachable",
    "runner_reply_malformed",
    "session_unknown",
];

/// A current limit collection and its measured sources.
pub(crate) const BUDGET_READ: &[&str] = &[
    "not_permitted",
    "holder_unknown",
    "BudgetsUnavailable",
    "ConfigurationUnavailable",
    "TeamsUnavailable",
    "NoPerson",
];
/// A whole-holder change is validated before a signed leaf is appended.
pub(crate) const BUDGET_SET: &[&str] = &[
    "budget_malformed",
    "BudgetVersionConflict",
    "TeamUnitRefused",
    "BudgetUnitUnavailable",
    "BudgetAmountRefused",
    "BudgetPeriodRefused",
    "BudgetWarningRefused",
    "BudgetLimitsRefused",
    "BudgetZoneRefused",
];
/// Fresh starts check authority before enforcing a known stop threshold.
pub(crate) const START_BUDGET: &[&str] = &[
    "AgentNotActive",
    "AgentNotVisible",
    "CertificatesUnavailable",
    "BudgetExhausted",
    "BudgetsUnavailable",
    "ConfigurationUnavailable",
    "TeamsUnavailable",
    "NoPerson",
    "NotAdmitted",
    "AgentNotVisible",
    "RequestMalformed",
    "RuntimeUnavailable",
];
