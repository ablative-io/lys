//! The route table's untyped entries and the authentication every entry of
//! the table is written with; the refusal sets are `openapi_refusals.rs`'s.
//! `openapi.rs` turns them into the document; nothing here is read by
//! anything else.

use lys_openapi::{Auth, Method};

use crate::openapi_refusals::{
    ADMIN, ADMIN_BODY, AGENT, BUDGET_READ, BUDGET_SET, GRANT_ASKED, GRANT_MADE, GRANT_READ,
    MACHINE_AGENTS, PERSON, PROFILE_REVIEW, RECORDED, REPORTING, RESTART, SIGNED, SIGNED_BODY,
    START_BUDGET, START_RUNNER, UNANSWERED,
};

pub(crate) const GET: Method = Method::Get;
pub(crate) const POST: Method = Method::Post;
pub(crate) const PUT: Method = Method::Put;

/// Anyone.
pub(crate) const P: &[Auth] = &[Auth::Public];
/// A personal session; the operator has no personal account.
pub(crate) const C: &[Auth] = &[Auth::Session];
/// A signed-in person or the install's operator.
pub(crate) const S: &[Auth] = &[Auth::Session, Auth::Operator];
/// A signed-in person, or an app or registrar through its credential.
pub(crate) const A: &[Auth] = &[Auth::Session, Auth::Bearer, Auth::Operator];
/// An agent's signed request, or a signed-in person.
pub(crate) const G: &[Auth] = &[Auth::AgentSignature, Auth::Session, Auth::Operator];
/// A provider-issued access token; a session or operator token cannot replace it.
pub(crate) const B: &[Auth] = &[Auth::Bearer];
/// An agent's signed request alone.
pub(crate) const AGENT_ONLY: &[Auth] = &[Auth::AgentSignature];

/// One untyped entry: method, path, words, authentication and the refusal
/// sets it answers with.
pub(crate) struct E(
    pub(crate) Method,
    pub(crate) &'static str,
    pub(crate) &'static str,
    pub(crate) &'static [Auth],
    pub(crate) &'static [&'static [&'static str]],
);

/// The table's entries, one line each: method, path, words, authentication
/// and refusal sets, written as the table macro reads them.
macro_rules! entries {
    ($($method:ident $path:literal $words:literal $auth:ident [$($set:expr),*];)*) => {
        &[$(E($method, $path, $words, $auth, &[$($set),*]),)*]
    };
}

/// Every route of the table; `openapi_types.rs` names the types each takes and answers.
pub(crate) const TABLE: &[E] = entries! {
    GET "/authority" "The authority this service speaks for" P [];
    GET "/login" "Redirect to the sign-in screen" P [];
    GET "/callback" "Finish a sign-in and begin a session" P [&["SignInStateUnknown", "SignInFailed", "SignInRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable"]];
    POST "/setup" "Finish the administrator's first-run setup" S [SIGNED_BODY, &["AlreadyBootstrapped", "NotAdmitted", "OperationReused", "ProfileInvalid"]];
    POST "/people" "Register a person" S [ADMIN_BODY, &["OperationReused"]];
    POST "/agents" "Register an agent" A [ADMIN_BODY, REPORTING, &["credential_refused", "ServiceAccountUnknown", "NotHeld", "PolicyUnavailable"]];
    POST "/drafts" "Record an agent's immutable draft" G [SIGNED_BODY, AGENT, &["NoPerson", "NotAdmitted", "DraftChangeInvalid", "OperationReused", "EventTooLarge"]];
    POST "/drafts/{id}/approve" "Record approval without applying the action" C [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"]];
    POST "/drafts/{id}/refuse" "Refuse one immutable draft" C [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"]];
    POST "/drafts/{id}/correct" "Refuse and save the responsible person's correction" C [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed", "EventTooLarge"]];
    POST "/agents/{id}/reports-to" "Change an agent's reporting edge" S [ADMIN_BODY, REPORTING];
    GET "/identities" "Every identity the directory holds" S [ADMIN];
    GET "/identities/{id}" "One identity" S [ADMIN];
    POST "/identities/{id}/profile" "Change an identity's profile" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "IdentityUnknown", "OperationReused", "ProfileInvalid"]];
    POST "/identities/{id}/transitions" "Move an identity's state" S [ADMIN_BODY];
    POST "/people/{id}/logins" "Bind a login to a person" S [ADMIN_BODY];
    GET "/me" "The signed-in caller" C [SIGNED, &["NoPerson", "SetupRequired"]];
    GET "/people" "The people the caller may see" S [SIGNED, &["NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    GET "/agents/{id}" "An agent the caller answers for" S [PERSON, &["AgentNotVisible"]];
    GET "/directory/people" "Every person, for the administrator" S [ADMIN, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    GET "/directory/agents/{id}" "Any agent, for the administrator" S [ADMIN, &["AgentNotVisible"]];
    POST "/grants/{id}/tokens" "Issue a credential for one grant" C [SIGNED_BODY, &["GrantTokenResponsibleRequired", "GrantTokenExpiryInvalid", "GrantTokenStoreFull", "GrantTokenUnavailable", "GrantTokenUnknown"]];
    POST "/grants/{id}/tokens/{token_id}/revoke" "Revoke one grant credential" C [SIGNED, &["GrantTokenResponsibleRequired", "GrantTokenUnavailable", "GrantTokenUnknown"]];
    GET "/grants" "The grants the caller may see" S [SIGNED, &["NotAdmitted"]];
    GET "/agent/grants" "The signed agent's own live grants and their chain admission" AGENT_ONLY [AGENT, &["DirectoryUnavailable", "AppsUnavailable", "LogUnavailable", "ServiceAccountsUnavailable", "CertificatesUnavailable"]];
    POST "/grants" "Pass on part of a grant" S [GRANT_MADE, RECORDED, &["NoPerson"], &["ExpiryBeyondSource", "UseOnly", "NotAdmitted", "NotHolder", "IdentityNotActive", "ResponsibleMismatch", "DirectoryUnavailable", "SourceUnknown", "ActionsOutside", "PassOnBeyondSource", "RecipientRefused"]];
    GET "/grants/model" "Lys's own permission model" S [SIGNED];
    POST "/grants/roots" "Issue a root grant" S [GRANT_MADE, RECORDED, &["RelationUnknown"], &["RootAuthorityRefused"]];
    POST "/grants/check" "Check, and record, an exercise" S [GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked"]];
    POST "/grants/why" "Why the caller may act" S [GRANT_ASKED, UNANSWERED, &["NotHeld"]];
    POST "/grants/who" "Who may act on a resource" S [GRANT_ASKED, UNANSWERED];
    POST "/grants/reach" "Who may act on each of many resources" S [GRANT_ASKED, UNANSWERED];
    GET "/grants/cannot-give" "What the caller cannot pass on" S [GRANT_READ, &["IdentityUnknown", "NotHolder", "IdentityNotActive"]];
    GET "/grants/{id}" "One grant the caller may see" S [GRANT_READ, &["GrantIdMalformed"]];
    POST "/grants/{id}/revoke" "Revoke a grant and all it derives" S [GRANT_READ, RECORDED, &["GrantUnknown", "RevokeRefused"]];
    GET "/receipts/{index}" "A directory receipt, publicly" P [&["RequestMalformed", "InstallEntry"]];
    GET "/service-key" "The service's public key" P [];
    GET "/reviews" "The grants due for review" S [SIGNED, &["NoPerson"]];
    POST "/reviews/{grant}/keep" "Keep a grant under review" S [SIGNED_BODY, &["GrantNotDue", "GrantNotVisible", "ReviewReused"], &["NoPerson", "ReviewerOnly"]];
    GET "/roles" "Every role" S [SIGNED];
    POST "/roles" "Make a role" S [ADMIN_BODY, &["RelationUnknown", "RoleReused"]];
    GET "/roles/{id}" "One role" S [SIGNED];
    POST "/roles/{id}/versions" "Revise a role" S [ADMIN_BODY, &["RoleReused"]];
    POST "/roles/{id}/holders" "Assign a role" S [ADMIN_BODY, &["HolderUnknown"], &["RoleHeld", "RoleUnknown"]];
    POST "/roles/{id}/holders/{holder}/move" "Move a holding" S [ADMIN_BODY, &["HoldingOver", "RoleVersionUnknown"], &["HolderUnknown", "HoldingChanged"]];
    POST "/roles/{id}/holders/{holder}/end" "End a holding" S [ADMIN_BODY];
    GET "/requests" "The access requests" S [SIGNED, &["NoPerson", "NotAdmitted", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    POST "/requests" "Ask for access" S [SIGNED_BODY, &["RelationUnknown"], &["NoPerson", "NotAdmitted", "RequestReused"]];
    POST "/requests/{id}/approve" "Approve an access request" S [SIGNED_BODY, &["NotAdmitted", "RequestDecided"], &["SourceUnknown"]];
    POST "/requests/{id}/decline" "Decline an access request" S [SIGNED_BODY, &["RequestDecided", "RequestUnknown"], &["NotAdmitted"]];
    POST "/requests/{id}/reconcile" "Settle an approval" S [SIGNED_BODY, &["NoPerson", "NotAdmitted"]];
    GET "/connections" "What the service is connected to" S [ADMIN];
    GET "/sign-in-providers" "The sign-in providers" S [ADMIN, &["SignInProvidersUnavailable"]];
    POST "/sign-in-providers" "Set a sign-in provider" S [ADMIN_BODY, &["ProviderRefused"]];
    POST "/link-audit" "Deliver a link-audit record" G [AGENT, &["NotAdmitted", "NotSignedIn", "RequestMalformed"]];
    POST "/link-audit/person" "Look up a link-audit holder" G [AGENT, &[ "LoginUnbound", "NotAdmitted", "NotSignedIn", "RequestMalformed", ]];
    GET "/network" "The machines" S [SIGNED, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    GET "/harnesses" "The programmes Lys describes and their reviewed builds" S [SIGNED, &["ProvisioningUnavailable"]];
    POST "/network/machines" "Name a machine" S [ADMIN_BODY, &["MachineReused"], &["IdentifierMalformed", "TeamUnknown", "TeamRetired", "TeamsUnavailable"]];
    POST "/network/machines/{id}/retire" "Retire a machine" S [ADMIN_BODY, &["MachineUnknown"]];
    POST "/network/machines/{id}/team" "Assign or clear a computer's owning team" S [SIGNED_BODY, &["NotAdmitted", "NoPerson", "IdentifierMalformed", "MachineUnknown", "MachineRetired", "MachineTeamReused", "TeamUnknown", "TeamRetired", "TeamsUnavailable"]];
    POST "/network/machines/{id}/agents" "Allow or remove one agent on a computer" S [ADMIN_BODY, MACHINE_AGENTS, &["NoPerson", "IdentifierMalformed", "AgentNotVisible", "MachineUnknown", "MachineRetired", "MachineWithoutRuntime", "NetworkUnavailable", "RuntimeUnavailable"]];
    GET "/agents/{id}/provisioning" "An agent's profile" S [SIGNED, &["AgentNotVisible"]];
    GET "/agents/{id}/mcp-requests" "An agent's MCP requests" S [SIGNED, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "ProvisioningReused"]];
    POST "/agents/{id}/mcp-requests" "Ask for a declared MCP server" S [SIGNED_BODY, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "ProvisioningReused", "ProfileNotReviewed", "RequestReused", "mcp_server_unknown", "mcp_server_held"]];
    POST "/agents/{id}/mcp-requests/{request}/approve" "Approve an MCP request within remit" G [SIGNED_BODY, AGENT, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "TeamsUnavailable", "ProfileNotReviewed", "ProvisioningReused", "RequestUnknown", "RequestDecided", "RequestReused", "mcp_beyond_remit", "mcp_server_held", "McpSettingUnrepresentable"]];
    POST "/agents/{id}/provisioning" "Set an agent's profile" S [&["ProvisioningUnavailable"], ADMIN_BODY, &["ProvisioningChanged"], &["AgentNotVisible", "ProvisioningReused"], &["McpCredentialInline", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown"]];
    GET "/skills" "The skills Lys keeps" S [SIGNED];
    POST "/skills" "Keep a skill's text" S [&["ProvisioningUnavailable"], ADMIN_BODY];
    POST "/agents/{id}/provisioning/{version}/review" "Review a profile" S [&["ProvisioningUnavailable"], ADMIN_BODY, PROFILE_REVIEW];
    POST "/agents/{id}/start-command" "An agent's start command" S [SIGNED_BODY, &["AgentNotVisible", "MachineCannotReach"], &["LaunchRecordMissing", "MachineNotForAgent", "MachineRetired", "MachineUnknown", "MachineWithoutRuntime", "NotAdmitted"], &["HarnessUndeclared", "LaunchUnrenderable", "McpHandleUnsupported", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown"], &["SecretsUnavailable"], START_RUNNER, START_BUDGET];
    POST "/agents/{id}/restart" "Restart an agent on its latest reviewed profile" S [SIGNED_BODY, RESTART];
    POST "/agents/{id}/runtime/sessions/{session}/reports" "A runtime report" G [AGENT, &["NotAdmitted"], &["AgentNotVisible", "RequestMalformed", "RuntimeSessionUnknown"]];
    GET "/agents/{id}/runtime/sessions" "An agent's runtime sessions" S [SIGNED, &["AgentNotVisible"]];
    GET "/runtime/sessions" "Every runtime session" S [ADMIN];
    GET "/runtime/message-edges" "Caller-visible message addresses from the message service, mapped to Lys identities" S [PERSON, &["MessageEdgesUnavailable"]];
    POST "/runtime/found/{session}/reports" "Report a found session" S [ADMIN_BODY, &["MachineUnknown"], &["RuntimeSessionStopped", "RuntimeSessionUnknown"]];
    GET "/runtime/found" "The sessions found" S [ADMIN];
    POST "/agents/{id}/stop" "Stop an agent" S [SIGNED_BODY, &["AgentNotVisible", "StopReused"]];
    GET "/agents/{id}/stops" "An agent's stops" S [SIGNED, &["AgentNotVisible"]];
    POST "/budgets/person/{id}/confirm" "Confirm a legacy personal budget" S [ADMIN_BODY, &["BudgetsUnavailable", "BudgetVersionConflict", "budget_invalid", "not_permitted"]];
    GET "/budgets/{kind}/{id}" "A holder's limits and measured usage" S [SIGNED, BUDGET_READ];
    PUT "/budgets/{kind}/{id}" "Replace a holder's limit collection" S [SIGNED_BODY, BUDGET_READ, BUDGET_SET, AGENT, GRANT_ASKED, UNANSWERED, &["HoldingNotHeld", "NotHeld", "Revoked"]];
    GET "/teams/{id}/budget" "A team's limits and measured usage" S [SIGNED, BUDGET_READ];
    PUT "/teams/{id}/budget" "Replace a team's limit collection" S [SIGNED_BODY, BUDGET_READ, BUDGET_SET, AGENT, GRANT_ASKED, UNANSWERED, &["HoldingNotHeld", "NotHeld", "Revoked"]];
    POST "/runtime/sessions/{id}/input" "Type into a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "session_ended"]];
    GET "/agents/{id}/refusals" "An agent's refused tool calls" S [SIGNED, &["AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/input-bytes" "Write exact bytes to a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "session_ended"]];
    POST "/runtime/sessions/{id}/read-bytes" "Read a session's exact output bytes" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/keys" "Send a session keys" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/read" "Read a session's screen" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/wait" "Wait for a session to show a pattern" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/resize" "Resize a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/compact" "Compact a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/runtime/sessions/{id}/end" "End a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]];
    POST "/agents/{id}/wake" "Wake an agent's live session" S [SIGNED_BODY, &["AgentNotVisible", "not_permitted", "no_live_session"]];
    GET "/runtime/live" "The sessions still running" S [SIGNED, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable", "AgentNotVisible", "IdentifierMalformed", "RuntimeSessionUnknown"]];
    GET "/network/machines/{id}/runner" "A machine's runner" S [SIGNED, &["MachineUnknown"]];
    POST "/network/machines/{id}/runner" "Name a machine's runner" S [ADMIN_BODY, &["MachineUnknown"]];
    GET "/runner/protocol" "The runner protocol" P [];
    POST "/runner/dial/{machine}/next" "A dialled runner's next request" P [&["runner_dial_refused", "runner_dial_stale", "runner_unreachable"]];
    POST "/runner/dial/{machine}/replies/{ticket}" "A dialled runner's reply" P [&["runner_dial_refused", "runner_dial_stale", "runner_unreachable"]];
    GET "/runner-receipts/{index}" "A runner act's receipt" P [&["RequestMalformed"]];
    GET "/agents/{id}/usage" "An agent's budget crossings and what came of each" S [SIGNED, BUDGET_READ, &["AgentNotVisible", "RuntimeSessionUnknown"]];
    POST "/agents/{id}/usage" "Report a use an agent made" S [SIGNED_BODY, BUDGET_READ, &["AgentNotVisible", "RuntimeSessionUnknown", "RuntimeUnavailable", "ProvisioningUnavailable"]];
    GET "/agents/{id}/policy" "An agent's tool-boundary policy" S [SIGNED, &["not_permitted"]];
    POST "/agents/{id}/policy" "Set an agent's tool-boundary policy, from its next launch" S [SIGNED_BODY, &["not_permitted", "PolicyVersionConflict"], &["policy_invalid", "policy_rule_duplicate", "policy_target_ambiguous"]];
    GET "/.well-known/openid-configuration" "The issuer's discovery document" P [];
    GET "/oauth/authorize" "Begin an authorization" P [&["RedirectUnregistered"]];
    POST "/oauth/token" "Exchange a code for tokens" P [&["CodeExpired", "CodeUnknown", "CodeUsed", "RedirectUnregistered", "VerifierWrong", "ProviderUnavailable", "SessionsUnavailable"]];
    GET "/oauth/jwks" "The issuer's signing keys" P [];
    GET "/oauth/userinfo" "The signed-in subject's claims" B [SIGNED, &["TokenUnknown", "ProviderUnavailable", "SessionsUnavailable"]];
    POST "/sign-in" "Sign in with a password" P [&["SignInFailed", "SecondFactorUnsupported", "SignInRefused", "SignInThrottled", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable"]];
    GET "/sign-in/providers" "The providers the sign-in page offers" P [];
    GET "/sign-in/providers/{id}" "Begin sign-in through a provider" P [];
    POST "/setup/open" "Open first-run setup with its code" P [&["SignInFailed", "SignInThrottled", "SetupClosed", "SetupCodeRefused", "DirectoryUnavailable"]];
    POST "/setup/administrator" "Register the first administrator" P [&["SignInFailed", "SignInThrottled", "AccountRefused", "SetupClosed", "SetupCodeRefused", "SignInRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "DirectoryUnavailable"]];
    POST "/setup/password" "Set the first administrator's password" P [&["SignInFailed", "SignInThrottled", "SetupCodeRefused", "SignInRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "DirectoryUnavailable"]];
    GET "/me/account" "The caller's sign-in account" C [SIGNED, &["AccountRefused"]];
    POST "/me/account/email" "Change the caller's email" C [SIGNED_BODY, &["AccountRefused"]];
    POST "/me/account/password" "Change the caller's password" C [SIGNED_BODY, &["AccountRefused"]];
    GET "/directory/people/{id}/account" "A person's sign-in account" S [ADMIN, &["AccountRefused", "IdentityUnknown"]];
    POST "/directory/people/{id}/account/email" "Change a person's email" S [ADMIN_BODY, &["AccountRefused", "IdentityUnknown"]];
    POST "/directory/people/{id}/account/enabled" "Enable or disable a person's sign-in" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "AccountRefused", "IdentityUnknown", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "ProviderUnavailable"]];
    POST "/directory/people/{id}/account/password" "Set a person's password" S [ADMIN_BODY, &["AccountRefused", "IdentityUnknown"]];
    GET "/agents/{id}/goals" "An agent's goals" S [SIGNED, &["AgentNotVisible", "goals_unavailable"]];
    POST "/agents/{id}/goals" "Set a goal on an agent" S [SIGNED_BODY, &["AgentNotVisible", "evidence_missing", "goal_reused", "reminder_needs_deadline"]];
    GET "/teams/{id}/goals" "A team's goals" S [SIGNED, &["NotAdmitted", "TeamUnknown"]];
    POST "/teams/{id}/goals" "Set a goal on a team" S [SIGNED_BODY, &["NotAdmitted", "TeamUnknown", "evidence_missing", "goal_reused", "reminder_needs_deadline"]];
    POST "/goals/{goal}/mark" "Judge a goal" G [AGENT, SIGNED_BODY, &["evidence_missing", "goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement"]];
    POST "/goals/{goal}/active" "Suspend or resume a standing aim" G [AGENT, SIGNED_BODY, &["goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement", "goals_unavailable", "TeamsUnavailable", "TeamUnknown"]];
    POST "/goals/{goal}/words" "Reword a standing aim" G [AGENT, SIGNED_BODY, &["goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement", "goals_unavailable", "TeamsUnavailable", "TeamUnknown"]];
    GET "/service-accounts" "The service accounts" S [SIGNED];
    POST "/service-accounts" "Create a service account" S [SIGNED_BODY, &["NotAdmitted"], &["ServiceAccountReused"]];
    POST "/service-accounts/{id}/retire" "Retire a service account" S [SIGNED_BODY, &["ServiceAccountRetired", "ServiceAccountUnknown"]];
    GET "/tree" "The caller's owned and led teams and descendants" G [AGENT, SIGNED, &["TeamsUnavailable", "RolesUnavailable", "ProvisioningUnavailable", "RuntimeUnavailable", "BudgetsUnavailable", "ConfigurationUnavailable", "goals_unavailable"]];
    GET "/teams" "Every team" S [SIGNED];
    POST "/teams" "Create a team" G [SIGNED_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotAdmitted", "team_parent_cycle", "team_lead_not_member", "TeamsUnavailable", "HoldingNotHeld", "NotHeld", "Revoked", "TeamUnknown", "NoPerson"]];
    GET "/teams/{id}" "One team" S [SIGNED];
    POST "/teams/{id}/members" "Add a team member" G [SIGNED_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["AgentNotVisible", "NotAdmitted", "not_permitted", "HoldingNotHeld", "NotHeld", "Revoked", "TeamUnknown", "TeamsUnavailable", "NoPerson"]];
    POST "/teams/{id}/members/{member}/remove" "Remove a member" S [SIGNED_BODY];
    POST "/teams/{id}/members/{member}/confirm" "Confirm a held membership" S [ADMIN_BODY, &["TeamsUnavailable"]];
    POST "/teams/{id}/nesting" "Replace a team parent and lead" S [SIGNED_BODY, &["NotAdmitted", "team_parent_cycle", "team_lead_not_member", "TeamsUnavailable"]];
    POST "/teams/{id}/retire" "Retire a team" S [SIGNED_BODY];
    GET "/resources" "The resources grants are on" S [SIGNED, &["NotAdmitted"]];
    GET "/secrets" "The secrets" S [SIGNED, &["NoPerson", "SecretsUnavailable"]];
    GET "/secrets/grants" "The secrets' grants" S [SIGNED];
    GET "/secrets/audit" "The secrets' audit" S [SIGNED];
    GET "/secrets/revocation" "The secrets' revocations" S [SIGNED, &["RequestMalformed"]];
    GET "/secrets/settings" "The secrets' settings" S [SIGNED, &["RequestMalformed"]];
    GET "/secrets/handles" "The secret handles" S [SIGNED, &["RequestMalformed"]];
    POST "/secrets/scope" "Scope a secret" S [SIGNED_BODY, &["LendingNotPermitted"]];
    POST "/secrets/recipients" "A secret's recipients" S [SIGNED_BODY];
    POST "/secrets/drop" "Drop a secret handle" S [SIGNED_BODY];
    GET "/sessions" "The caller's sessions" S [SIGNED, &["NoPerson"]];
    POST "/sessions/{id}/end" "End one's own session" S [SIGNED, &["SessionUnknown"], &["NoPerson"]];
    GET "/directory/people/{id}/sessions" "A person's sessions" S [ADMIN, &["IdentityUnknown"]];
    POST "/directory/people/{id}/sessions/{session}/end" "End a session" S [ADMIN, &["SessionUnknown"], &["IdentityUnknown"]];
    GET "/configuration" "The service's configuration" S [ADMIN, &["ConfigurationUnavailable"]];
    PUT "/configuration" "Set the organisation zone at its current version" S [ADMIN_BODY, &["ConfigurationMalformed", "ConfigurationVersionConflict", "ConfigurationZoneRefused", "ConfigurationUnavailable", "NoPerson"]];
    GET "/agents/{id}/memory" "An agent's memory" S [SIGNED, &["AgentNotVisible"]];
    GET "/agents/{id}/certificates" "An agent's certificates" S [SIGNED, &["AgentNotVisible"]];
    POST "/agents/{id}/certificates" "Issue a certificate" S [ADMIN_BODY, &["AgentNotVisible", "CertificateReused"], &["CertificateReused"]];
    POST "/agents/{id}/certificates/{serial}/withdrawal" "Withdraw one" S [ADMIN_BODY, &["CertificateUnknown", "CertificateWithdrawn"], &["CertificateWithdrawn"]];
    POST "/agents/{id}/start" "Start an agent" S [SIGNED_BODY, START_BUDGET, &["AgentHasNoPolicy", "PolicyUnavailable"]];
    POST "/launch-records/{id}/start-again" "Start a launch again" S [SIGNED_BODY, &["AgentHasNoPolicy", "PolicyUnavailable"]];
    POST "/launch-records/{id}/withdraw" "Withdraw a launch" S [SIGNED_BODY];
    GET "/launch-records/{id}/state" "A launch's state" S [SIGNED];
    GET "/changes" "Wait for the next change signal" S [SIGNED, &["RequestMalformed", "RuntimeUnavailable"]];
    GET "/mcp" "MCP server stream availability" G [SIGNED];
    POST "/mcp" "MCP calls through the admitted HTTP router" G [SIGNED, AGENT];
    GET "/surface-contract" "The surface registration and computer admission contract" P [];
    GET "/openapi.json" "This document" P [];
};
