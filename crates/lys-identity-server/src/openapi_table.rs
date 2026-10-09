//! The route table's untyped entries and the authentication every entry of
//! the table is written with; the refusal sets are `openapi_refusals.rs`'s.
//! `openapi.rs` turns them into the document; nothing here is read by
//! anything else.

use lys_openapi::{Auth, Method};
use lys_runner::console_stop::PATH as CONSOLE_PATH;

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
/// The service's own signature over the console request.
pub(crate) const CONSOLE_ONLY: &[Auth] = &[Auth::ConsoleSignature {
    header: lys_runner::console_stop::SIGNATURE_HEADER,
}];

/// One untyped entry: method, path, words, authentication and the refusal
/// sets it answers with.
pub(crate) struct E(
    pub(crate) Method,
    pub(crate) &'static str,
    pub(crate) &'static str,
    pub(crate) &'static [Auth],
    pub(crate) &'static [&'static [&'static str]],
    pub(crate) Option<Scope>,
);

/// A route's declared token scope, resolved only from named path parameters.
#[derive(Clone, Copy)]
pub(crate) struct Scope {
    kind: &'static str,
    action: &'static str,
    parameters: &'static [&'static str],
}

macro_rules! entries {
    ($($method:ident $path:tt $words:literal $auth:ident [$($set:expr),*] $(scope($kind:literal, $action:literal, [$($parameter:literal),*]))?;)*) => {
        &[$(E($method, $path, $words, $auth, &[$($set),*], entries!(@scope $($kind, $action, [$($parameter),*])?)),)*]
    };
    (@scope) => { None };
    (@scope $kind:literal, $action:literal, [$($parameter:literal),*]) => {
        Some(Scope { kind: $kind, action: $action, parameters: &[$($parameter),*] })
    };
}

/// Resolve one declared scope; an absent declaration never implies authority.
pub(crate) fn token_scope(
    method: &str,
    path: &str,
) -> Result<
    (lys_identity::grants::Resource, lys_identity::grants::Action),
    crate::grant_tokens::TokenError,
> {
    use crate::grant_tokens::TokenError;
    use lys_identity::grants::{Action, Resource};
    static DECLARED: std::sync::OnceLock<Vec<&'static E>> = std::sync::OnceLock::new();
    let rows = DECLARED.get_or_init(|| TABLE.iter().filter(|row| row.5.is_some()).collect());
    for row in rows {
        if !method.eq_ignore_ascii_case(row.0.word()) {
            continue;
        }
        let mut values = std::collections::BTreeMap::new();
        let mut concrete = path.split('/');
        let matched = row.1.split('/').all(|part| {
            let Some(value) = concrete.next() else {
                return false;
            };
            if let Some(name) = part
                .strip_prefix('{')
                .and_then(|part| part.strip_suffix('}'))
            {
                if value.is_empty() {
                    return false;
                }
                values.insert(name, value);
                true
            } else {
                part == value
            }
        }) && concrete.next().is_none();
        if !matched {
            continue;
        }
        let scope = row.5.ok_or(TokenError::Undeclared)?;
        let id = scope
            .parameters
            .iter()
            .map(|name| {
                name.strip_prefix('=')
                    .or_else(|| values.get(name).copied())
                    .ok_or(TokenError::Undeclared)
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(".");
        return Ok((
            Resource::new(
                scope.kind,
                if scope.parameters.is_empty() {
                    "all"
                } else {
                    &id
                },
            )
            .map_err(crate::error::ServerError::from)?,
            Action::new(scope.action).map_err(crate::error::ServerError::from)?,
        ));
    }
    Err(TokenError::Undeclared)
}

#[cfg(test)]
#[path = "route_scope_tests.rs"]
mod tests;

/// Every route of the table; `openapi_types.rs` names the types each takes and answers.
pub(crate) const TABLE: &[E] = entries! {
    GET "/authority" "The authority this service speaks for" P [];
    GET "/health" "That this service is serving, with its name and build" P [];
    GET "/login" "Redirect to the sign-in screen" P [];
    GET "/callback" "Finish a sign-in and begin a session" P [&["SignInStateUnknown", "SignInFailed", "SignInRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable"]];
    POST "/setup" "Finish the administrator's first-run setup" S [SIGNED_BODY, &["AlreadyBootstrapped", "NotAdmitted", "OperationReused", "ProfileInvalid"]];
    POST "/people" "Register a person" S [ADMIN_BODY, &["OperationReused"]] scope("person", "person.create", []);
    POST "/agents" "Register an agent" A [ADMIN_BODY, REPORTING, &["credential_refused", "ServiceAccountUnknown", "NotHeld", "PolicyUnavailable"]] scope("agent", "agent.create", []);
    POST "/drafts" "Record an agent's immutable draft" G [SIGNED_BODY, AGENT, &["NoPerson", "NotAdmitted", "DraftChangeInvalid", "OperationReused"]];
    GET "/drafts" "The drafts the signed-in person may decide" C [SIGNED, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    GET "/dashboard" "The caller's own agents, how each is going, and what waits on the caller" C [SIGNED, &["NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    POST "/drafts/{id}/approve" "Record approval without applying the action" C [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"]];
    POST "/drafts/{id}/refuse" "Refuse one immutable draft" C [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"]];
    POST "/drafts/{id}/correct" "Refuse and save the responsible person's correction" C [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"]];
    POST "/agents/{id}/reports-to" "Change an agent's reporting edge" S [ADMIN_BODY, REPORTING] scope("agent", "agent.reports-to.set", ["id"]);
    GET "/identities" "Every identity the directory holds" S [ADMIN] scope("identity", "read", []);
    GET "/identities/{id}" "One identity" S [ADMIN, &["IdentifierMalformed"]] scope("identity", "read", ["id"]);
    POST "/identities/{id}/profile" "Change an identity's profile" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "IdentityUnknown", "OperationReused", "ProfileInvalid"]] scope("person", "person.profile.set", ["id"]);
    POST "/identities/{id}/transitions" "Move an identity's state" S [ADMIN_BODY] scope("identity", "identity.transition", ["id"]);
    POST "/people/{id}/logins" "Bind a login to a person" S [ADMIN_BODY] scope("person", "person.login.bind", ["id"]);
    POST "/people/admit" "Admit a person in one act: register, issuer account by email, bind, activate, first root grant" S [ADMIN_BODY, &["RelationUnknown", "OperationReused"]] scope("person", "person.create", []);
    GET "/me" "The signed-in caller" C [SIGNED, &["NoPerson", "SetupRequired"]];
    GET "/people" "The people the caller may see" S [SIGNED, &["NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("person", "read", []);
    GET "/agents/{id}" "An agent the caller answers for" S [PERSON, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/directory/people" "Every person, for the administrator" S [ADMIN, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("person", "read", []);
    GET "/directory/agents/{id}" "Any agent, for the administrator" S [ADMIN, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    POST "/grants/{id}/tokens" "Issue a credential for one grant" C [SIGNED_BODY, &["GrantTokenResponsibleRequired", "GrantTokenExpiryInvalid", "GrantTokenUnavailable", "GrantTokenUnknown"]];
    POST "/grants/{id}/tokens/{token_id}/revoke" "Revoke one grant credential" C [SIGNED, &["GrantTokenResponsibleRequired", "GrantTokenUnavailable", "GrantTokenUnknown"]] scope("grant", "grant.revoke", ["id"]);
    GET "/grants" "The grants the caller may see" S [SIGNED, &["NotAdmitted"]] scope("grant", "read", []);
    GET "/agent/grants" "The signed agent's own live grants and their chain admission" AGENT_ONLY [AGENT, &["DirectoryUnavailable", "LogUnavailable", "ServiceAccountsUnavailable", "CertificatesUnavailable"]];
    POST "/grants" "Pass on part of a grant" S [GRANT_MADE, RECORDED, &["NoPerson"], &["ExpiryBeyondSource", "UseOnly", "NotAdmitted", "NotHolder", "IdentityNotActive", "ResponsibleMismatch", "DirectoryUnavailable", "SourceUnknown", "ActionsOutside", "PassOnBeyondSource", "RecipientRefused", "WithheldFromAgents", "MachineRefused"]] scope("grant", "grant.delegate", []);
    GET "/grants/model" "Lys's own permission model" S [SIGNED] scope("grant", "read", []);
    POST "/grants/roots" "Issue a root grant, outright or held by draft or by two" S [GRANT_MADE, RECORDED, &["RelationUnknown"], &["RootAuthorityRefused", "WithheldFromAgents", "MachineRefused", "grant_mode_on_hot_action"]];
    POST "/grants/agent-roots" "Record the administrator's roots for giving people and agents access" S [ADMIN, &["NoPerson", "RelationUnknown"], &["RootAuthorityRefused", "WithheldFromAgents"]];
    POST "/grants/check" "Check, and record, an exercise" S [GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked"]] scope("grant", "grant.check", []);
    POST "/grants/why" "Why the caller may act" S [GRANT_ASKED, UNANSWERED, &["NotHeld"]] scope("grant", "grant.why", []);
    POST "/grants/who" "Who may act on a resource" S [GRANT_ASKED, UNANSWERED] scope("grant", "grant.who", []);
    POST "/grants/reach" "Who may act on each of many resources" S [GRANT_ASKED, UNANSWERED] scope("grant", "grant.reach", []);
    GET "/grants/cannot-give" "What the caller cannot pass on" S [GRANT_READ, &["IdentityUnknown", "NotHolder", "IdentityNotActive"]] scope("grant", "read", []);
    GET "/grants/{id}" "One grant the caller may see" S [GRANT_READ, &["GrantIdMalformed"]] scope("grant", "read", ["id"]);
    POST "/grants/{id}/revoke" "Revoke a grant and all it derives" S [GRANT_READ, RECORDED, &["GrantUnknown", "RevokeRefused"]] scope("grant", "grant.revoke", ["id"]);
    GET "/receipts/{index}" "A directory receipt, publicly" P [&["RequestMalformed", "InstallEntry"]];
    GET "/service-key" "The service's public key" P [];
    GET "/reviews" "The grants due for review" S [SIGNED, &["NoPerson"]] scope("review", "read", []);
    POST "/reviews/{grant}/keep" "Keep a grant under review" S [SIGNED_BODY, &["GrantNotDue", "GrantNotVisible", "ReviewReused"], &["NoPerson", "ReviewerOnly"]] scope("review", "review.keep", ["grant"]);
    GET "/roles" "Every role" S [SIGNED] scope("role", "read", []);
    POST "/roles" "Make a role" S [ADMIN_BODY, &["RelationUnknown", "RoleReused"]] scope("role", "role.create", []);
    GET "/roles/{id}" "One role" S [SIGNED] scope("role", "read", ["id"]);
    POST "/roles/{id}/versions" "Revise a role" S [ADMIN_BODY, &["RoleReused"]] scope("role", "role.revise", ["id"]);
    POST "/roles/{id}/holders" "Assign a role" S [ADMIN_BODY, &["HolderUnknown"], &["RoleHeld", "RoleUnknown"]] scope("role", "role.holder.assign", ["id"]);
    POST "/roles/{id}/holders/{holder}/move" "Move a holding" S [ADMIN_BODY, &["HoldingOver", "RoleVersionUnknown"], &["HolderUnknown", "HoldingChanged"]] scope("role", "role.holder.move", ["id"]);
    POST "/roles/{id}/holders/{holder}/end" "End a holding" S [ADMIN_BODY] scope("role", "role.holder.end", ["id"]);
    GET "/requests" "The access requests" S [SIGNED, &["NoPerson", "NotAdmitted", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("request", "read", []);
    POST "/requests" "Ask for access" S [SIGNED_BODY, &["RelationUnknown"], &["NoPerson", "NotAdmitted", "RequestReused"]] scope("request", "request.create", []);
    POST "/requests/{id}/approve" "Approve an access request" S [SIGNED_BODY, &["NotAdmitted", "RequestDecided"], &["SourceUnknown", "WithheldFromAgents"]] scope("request", "request.approve", ["id"]);
    POST "/requests/{id}/decline" "Decline an access request" S [SIGNED_BODY, &["RequestDecided", "RequestUnknown"], &["NotAdmitted"]] scope("request", "request.decline", ["id"]);
    POST "/requests/{id}/reconcile" "Settle an approval" S [SIGNED_BODY, &["NoPerson", "NotAdmitted"]] scope("request", "request.reconcile", ["id"]);
    GET "/connections" "What the service is connected to" S [ADMIN] scope("connection", "read", []);
    GET "/sign-in-providers" "The sign-in providers" S [ADMIN, &["SignInProvidersUnavailable"]];
    POST "/sign-in-providers" "Set a sign-in provider" S [ADMIN_BODY, &["ProviderRefused"]];
    POST "/link-audit" "Deliver a link-audit record" G [AGENT, &["NotAdmitted", "NotSignedIn", "RequestMalformed"]];
    POST "/link-audit/person" "Look up a link-audit holder" G [AGENT, &[ "LoginUnbound", "NotAdmitted", "NotSignedIn", "RequestMalformed", ]];
    GET "/network" "The machines" S [SIGNED, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable", "NoPerson"]] scope("machine", "read", []);
    GET "/harnesses" "The programmes Lys describes and their reviewed builds" S [SIGNED, &["ProvisioningUnavailable"]] scope("harness", "read", []);
    POST "/network/machines" "Name a machine" S [ADMIN_BODY, &["MachineReused"], &["IdentifierMalformed", "TeamUnknown", "TeamRetired", "TeamsUnavailable"]] scope("machine", "machine.create", []);
    POST "/network/machines/{id}/retire" "Retire a machine" S [ADMIN_BODY, &["MachineUnknown"]] scope("machine", "machine.retire", ["id"]);
    POST "/network/machines/{id}/team" "Assign or clear a computer's owning team" S [SIGNED_BODY, &["NotAdmitted", "NoPerson", "IdentifierMalformed", "MachineUnknown", "MachineRetired", "MachineTeamReused", "TeamUnknown", "TeamRetired", "TeamsUnavailable"]] scope("machine", "machine.team.set", ["id"]);
    POST "/network/machines/{id}/agents" "Allow or remove one agent on a computer" S [ADMIN_BODY, MACHINE_AGENTS, &["NoPerson", "IdentifierMalformed", "AgentNotVisible", "MachineUnknown", "MachineRetired", "MachineWithoutRuntime", "NetworkUnavailable", "RuntimeUnavailable"]] scope("machine", "machine.agent.set", ["id"]);
    GET "/agents/{id}/provisioning" "An agent's profile" S [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/mcp-requests" "An agent's MCP requests" S [SIGNED, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "ProvisioningReused"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/mcp-requests" "Ask for a declared MCP server" S [SIGNED_BODY, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "ProvisioningReused", "ProfileNotReviewed", "RequestReused", "mcp_server_unknown", "mcp_server_held"]] scope("agent", "agent.mcp-request.create", ["id"]);
    POST "/agents/{id}/mcp-requests/{request}/approve" "Approve an MCP request within remit" G [SIGNED_BODY, AGENT, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "TeamsUnavailable", "ProfileNotReviewed", "ProvisioningReused", "RequestUnknown", "RequestDecided", "RequestReused", "mcp_beyond_remit", "mcp_server_held", "McpSettingUnrepresentable"]] scope("agent", "agent.mcp-request.approve", ["id"]);
    POST "/agents/{id}/provisioning" "Set an agent's profile" S [&["ProvisioningUnavailable"], ADMIN_BODY, &["ProvisioningChanged"], &["AgentNotVisible", "ProvisioningReused"], &["McpCredentialInline", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown"]] scope("agent", "agent.provisioning.set", ["id"]);
    GET "/skills" "The skills Lys keeps" S [SIGNED] scope("skill", "read", []);
    POST "/skills" "Keep a skill's text" S [&["ProvisioningUnavailable"], ADMIN_BODY] scope("skill", "skill.keep", []);
    POST "/agents/{id}/provisioning/{version}/review" "Review a profile" S [&["ProvisioningUnavailable"], ADMIN_BODY, PROFILE_REVIEW] scope("agent", "agent.provisioning.review", ["id"]);
    POST "/agents/{id}/start-command" "An agent's start command" S [SIGNED_BODY, &["AgentNotVisible", "MachineCannotReach"], &["LaunchRecordMissing", "MachineNotForAgent", "MachineRetired", "MachineUnknown", "MachineWithoutRuntime", "NotAdmitted", "WorkingFolderUnnamed"], &["HarnessUndeclared", "LaunchUnrenderable", "McpHandleUnsupported", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown"], &["SecretsUnavailable"], START_RUNNER, START_BUDGET, &["everything_stopped"]] scope("agent", "agent.start-command", ["id"]);
    POST "/agents/{id}/restart" "Restart an agent on its latest reviewed profile" S [SIGNED_BODY, RESTART, &["everything_stopped"]] scope("agent", "agent.restart", ["id"]);
    POST "/agents/{id}/runtime/sessions/{session}/reports" "A runtime report" G [AGENT, &["NotAdmitted"], &["AgentNotVisible", "RequestMalformed", "RuntimeSessionUnknown"]] scope("agent", "agent.runtime.report", ["id"]);
    GET "/agents/{id}/control-sessions" "An agent's control session identities, stopped sessions included" S [SIGNED, &["AgentNotVisible", "RuntimeUnavailable", "RequestMalformed"]] scope("agent", "read", ["id"]);
    GET "/runtime/sessions/{session}/controls" "A session's current control identifiers" S [SIGNED, &["RuntimeSessionUnknown", "runner_absent", "control_status_mismatch"]] scope("runtime-session", "read", ["session"]);
    GET "/runtime/sessions/{session}/control-receipts" "A bounded page of session control evidence" S [SIGNED, &["RuntimeSessionUnknown", "runner_absent", "RequestMalformed", "control_page_mismatch"]] scope("runtime-session", "read", ["session"]);
    POST "/runtime/sessions/{session}/control-receipts/{operation}/reconcile" "The responsible person's decision about uncertain delivery" S [SIGNED_BODY, &["NoPerson", "RuntimeSessionUnknown", "not_permitted", "control_not_uncertain", "control_already_reconciled"]] scope("runtime-session", "runtime-session.reconcile-control", ["session"]);
    GET "/goals/{goal}/resends/{prior}" "The kept resend of one uncertain goal delivery" S [SIGNED, &["NoPerson", "goal_unknown", "goal_prior_unknown", "control_not_uncertain", "goal_resend_invalid"]] scope("goal", "goal.resend", ["goal"]);
    POST "/goals/{goal}/resend" "A distinct authorised occurrence naming possible prior delivery" S [SIGNED_BODY, &["NoPerson", "goal_unknown", "goal_authority_revoked", "goal_prior_already_resent", "goal_resend_kept_reconciliation_unavailable"]] scope("goal", "goal.resend", ["goal"]);
    GET "/agents/{id}/runtime/sessions" "An agent's runtime sessions" S [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/runtime/sessions" "Every runtime session" S [ADMIN] scope("runtime-session", "read", []);
    GET "/runtime/message-edges" "Caller-visible message addresses from the message service, mapped to Lys identities" S [PERSON, &["MessageEdgesUnavailable"]] scope("runtime-session", "read", []);
    POST "/runtime/found/{session}/reports" "Report a found session" S [ADMIN_BODY, &["MachineUnknown"], &["RuntimeSessionStopped", "RuntimeSessionUnknown"]] scope("runtime-session", "runtime-session.found.report", ["session"]);
    GET "/runtime/found" "The sessions found" S [ADMIN] scope("runtime-session", "read", []);
    POST "/agents/{id}/stop" "Stop an agent" S [SIGNED_BODY, &["AgentNotVisible", "StopReused"]] scope("agent", "agent.stop", ["id"]);
    GET "/agents/{id}/stops" "An agent's stops" S [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/runtime/stop-everything" "Whether everything is stopped, by whom, when and why" S [SIGNED, &["cord_unavailable"]] scope("runtime-session", "read", []);
    POST "/runtime/stop-everything" "Stop every session on every computer and refuse every start" S [ADMIN_BODY, &["IdentifierMalformed", "cord_reused", "cord_unavailable", "RuntimeUnavailable"]] scope("runtime-session", "runtime.stop-everything", []);
    POST CONSOLE_PATH "Stop every session under the service's own console signature" CONSOLE_ONLY [SIGNED_BODY, &["console_signature_refused", "IdentifierMalformed", "cord_reused", "cord_unavailable", "RuntimeUnavailable"]];
    POST "/runtime/stop-everything/release" "Let agents start again" S [ADMIN_BODY, &["IdentifierMalformed", "cord_not_pulled", "cord_reused", "cord_unavailable"]] scope("runtime-session", "runtime.stop-everything", []);
    POST "/budgets/person/{id}/confirm" "Confirm a legacy personal budget" S [ADMIN_BODY, &["BudgetsUnavailable", "BudgetVersionConflict", "budget_invalid", "not_permitted"]];
    GET "/budgets/{kind}/{id}" "A holder's limits and measured usage" S [SIGNED, BUDGET_READ] scope("budget", "read", ["kind", "id"]);
    PUT "/budgets/{kind}/{id}" "Replace a holder's limit collection" G [SIGNED_BODY, BUDGET_READ, BUDGET_SET, AGENT, GRANT_ASKED, UNANSWERED, &["HoldingNotHeld", "NotHeld", "Revoked"]] scope("budget", "budget.set", ["kind", "id"]);
    GET "/teams/{id}/budget" "A team's limits and measured usage" S [SIGNED, BUDGET_READ] scope("team", "read", ["id"]);
    PUT "/teams/{id}/budget" "Replace a team's limit collection" G [SIGNED_BODY, BUDGET_READ, BUDGET_SET, AGENT, GRANT_ASKED, UNANSWERED, &["HoldingNotHeld", "NotHeld", "Revoked"]] scope("team", "team.budget.set", ["id"]);
    POST "/runtime/sessions/{id}/input" "Type into a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "session_ended"]] scope("runtime-session", "runtime-session.input", ["id"]);
    GET "/agents/{id}/refusals" "An agent's refused tool calls" S [SIGNED, &["AgentNotVisible", "not_permitted"]] scope("agent", "read", ["id"]);
    POST "/runtime/sessions/{id}/input-bytes" "Write exact bytes to a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "session_ended"]] scope("runtime-session", "runtime-session.input-bytes", ["id"]);
    POST "/runtime/sessions/{id}/read-bytes" "Read a session's exact output bytes" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "read", ["id"]);
    POST "/runtime/sessions/{id}/keys" "Send a session keys" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.keys", ["id"]);
    POST "/runtime/sessions/{id}/read" "Read a session's screen" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "read", ["id"]);
    POST "/runtime/sessions/{id}/wait" "Wait for a session to show a pattern" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "read", ["id"]);
    POST "/runtime/sessions/{id}/resize" "Resize a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.resize", ["id"]);
    POST "/runtime/sessions/{id}/compact" "Compact a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.compact", ["id"]);
    POST "/runtime/sessions/{id}/end" "End a session" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.end", ["id"]);
    POST "/agents/{id}/wake" "Wake an agent's live session" S [SIGNED_BODY, &["AgentNotVisible", "not_permitted", "no_live_session"]] scope("agent", "agent.wake", ["id"]);
    GET "/runtime/live" "The sessions still running" S [SIGNED, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable", "AgentNotVisible", "IdentifierMalformed", "RuntimeSessionUnknown"]] scope("runtime-session", "read", []);
    GET "/network/machines/{id}/runner" "A machine's runner" S [SIGNED, &["MachineUnknown"]] scope("machine", "read", ["id"]);
    POST "/network/machines/{id}/runner" "Name a machine's runner" S [ADMIN_BODY, &["MachineUnknown"]] scope("machine", "machine.runner.set", ["id"]);
    POST "/network/machines/{id}/folders" "The folders inside one folder of a computer" S [ADMIN_BODY, &["MachineUnknown", "runner_absent"], &["runner_request_unsigned", "runner_request_malformed", "runner_protocol_mismatch", "runner_request_replayed", "runner_request_misaddressed", "runner_unreachable", "runner_reply_malformed", "runner_stopping", "home_unknown", "folder_unreadable"]] scope("machine", "read", ["id"]);
    GET "/runner/protocol" "The runner protocol" P [];
    POST "/runner/dial/{machine}/next" "A dialled runner's next request" P [&["runner_dial_refused", "runner_dial_stale", "runner_unreachable"]];
    POST "/runner/dial/{machine}/replies/{ticket}" "A dialled runner's reply" P [&["runner_dial_refused", "runner_dial_stale", "runner_unreachable"]];
    POST "/network/machines/{id}/join-code" "Give a one-time connection code for another computer" S [ADMIN_BODY, &["NoPerson", "IdentifierMalformed", "MachineUnknown", "MachineRetired", "NetworkUnavailable", "RunnerJoinUnreachable", "RunnerJoinOperationReused"]] scope("machine", "machine.runner.set", ["id"]);
    POST "/runner/join" "Join another computer's runner with its connection code" P [&["RequestMalformed", "RunnerJoinRefused", "MachineRetired", "NetworkUnavailable"]];
    GET "/network/machine-identities" "The computers that joined, as machine identities, and the grants each holds" S [ADMIN, &["NetworkUnavailable", "IdentifierMalformed"]] scope("machine", "read", []);
    GET "/runner-receipts/{index}" "A runner act's receipt" P [&["RequestMalformed"]];
    GET "/agents/{id}/usage" "An agent's budget crossings and what came of each" S [SIGNED, BUDGET_READ, &["AgentNotVisible", "RuntimeSessionUnknown"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/calls" "An agent's model calls, newest first" S [SIGNED, BUDGET_READ, &["AgentNotVisible", "RequestMalformed"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/calls/{call}" "One model call whole: its record and both bodies" S [SIGNED, BUDGET_READ, &["AgentNotVisible", "not_permitted", "CallUnknown", "CallKeptElsewhere", "CallRecordsUnavailable"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/usage" "Report a use an agent made" S [SIGNED_BODY, BUDGET_READ, &["AgentNotVisible", "RuntimeSessionUnknown", "RuntimeUnavailable", "ProvisioningUnavailable"]] scope("agent", "agent.usage.report", ["id"]);
    GET "/agents/{id}/policy" "An agent's tool-boundary policy" S [SIGNED, &["not_permitted"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/policy" "Set an agent's tool-boundary policy, from its next launch" S [SIGNED_BODY, &["not_permitted", "PolicyVersionConflict"], &["policy_invalid", "policy_rule_duplicate", "policy_target_ambiguous"]] scope("agent", "agent.policy.set", ["id"]);
    GET "/.well-known/openid-configuration" "The issuer's discovery document" P [];
    GET "/oauth/authorize" "Begin an authorization" P [&["RequestMalformed", "RedirectUnregistered", "ScopeUnknown", "ScopeNotGranted", "NoPerson", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"], &["credential_refused", "app_not_approved", "app_retired", "redirect_invalid", "apps_unavailable"]];
    POST "/oauth/token" "Exchange a code or a refresh token for a pass" P [&["RequestMalformed", "ClientUnknown", "CodeExpired", "CodeUnknown", "CodeUsed", "RedirectUnregistered", "VerifierWrong", "RefreshUnknown", "SessionEnded", "HolderRetired", "NotAdmitted", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"], UNANSWERED, &["credential_refused", "app_not_approved", "app_retired", "redirect_invalid", "apps_unavailable", "SecretsUnavailable"], &["grant_log_identity_unavailable", "grant_binding_unsupported", "grant_binding_revision_moved", "grant_binding_degraded"]];
    GET "/oauth/jwks" "The issuer's signing keys" P [];
    GET "/.well-known/oauth-protected-resource" "Where the MCP door's authorization is found" P [];
    GET "/.well-known/oauth-protected-resource/mcp" "Where the MCP door's authorization is found, by its address" P [];
    GET "/.well-known/oauth-protected-resource/api/mcp" "Where the MCP door's authorization is found, by its screens address" P [];
    GET "/.well-known/oauth-authorization-server" "How a connected app is authorized" P [];
    POST "/oauth/mcp/register" "An app registers to connect to the MCP door" P [&["RequestMalformed", "RegistrationThrottled", "ConfigInvalid"]];
    GET "/oauth/mcp/authorize" "Ask the person to connect an app" P [&["RequestMalformed", "RedirectUnregistered"]];
    POST "/oauth/mcp/consent" "The person connects an app or refuses it" P [&["RequestMalformed", "RedirectUnregistered", "CodeUnknown", "NotSignedIn"]];
    POST "/oauth/mcp/token" "Exchange a connected app's code or refresh token for tokens" P [];
    GET "/oauth/userinfo" "The signed-in subject's claims" B [SIGNED, &["TokenUnknown", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"], &["apps_unavailable"]];
    POST "/sign-in" "Sign in with a password" P [&["SignInFailed", "SecondFactorUnsupported", "SignInRefused", "IssuerChallengeExpired", "IssuerRefused", "SignInThrottled", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable"]];
    GET "/sign-in/providers" "The providers the sign-in page offers" P [];
    GET "/sign-in/providers/{id}" "Begin sign-in through a provider" P [&["IssuerChallengeExpired"]];
    POST "/setup/open" "Open first-run setup with its code" P [&["SignInFailed", "SignInThrottled", "SetupClosed", "SetupCodeRefused", "DirectoryUnavailable"]];
    POST "/setup/administrator" "Register the first administrator" P [&["SignInFailed", "SignInThrottled", "AccountRefused", "SetupClosed", "SetupCodeRefused", "SignInRefused", "IssuerChallengeExpired", "IssuerRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "DirectoryUnavailable"]];
    POST "/setup/password" "Set the first administrator's password" P [&["SignInFailed", "SignInThrottled", "SetupCodeRefused", "SignInRefused", "IssuerChallengeExpired", "IssuerRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "DirectoryUnavailable"]];
    GET "/me/account" "The caller's sign-in account" C [SIGNED, &["AccountRefused"]];
    POST "/me/account/email" "Change the caller's email" C [SIGNED_BODY, &["AccountRefused"]];
    POST "/me/account/password" "Change the caller's password" C [SIGNED_BODY, &["AccountRefused"]];
    GET "/directory/people/{id}/account" "A person's sign-in account" S [ADMIN, &["AccountRefused", "IdentityUnknown"]] scope("account", "read", ["id"]);
    POST "/directory/people/{id}/account/email" "Change a person's email" S [ADMIN_BODY, &["AccountRefused", "IdentityUnknown"]] scope("account", "person.email.set", ["id"]);
    POST "/directory/people/{id}/account/enabled" "Enable or disable a person's sign-in" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "AccountRefused", "IdentityUnknown", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "ProviderUnavailable"]] scope("account", "person.sign-in.set", ["id"]);
    POST "/directory/people/{id}/account/password" "Set a person's password" S [ADMIN_BODY, &["AccountRefused", "IdentityUnknown"]] scope("account", "person.password.set", ["id"]);
    GET "/agents/{id}/goals" "An agent's goals" S [SIGNED, &["AgentNotVisible", "goals_unavailable"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/goals" "Set a goal on an agent" S [SIGNED_BODY, &["AgentNotVisible", "evidence_missing", "goal_reused", "reminder_needs_deadline"]];
    GET "/teams/{id}/goals" "A team's goals" S [SIGNED, &["NotAdmitted", "TeamUnknown"]] scope("team", "read", ["id"]);
    POST "/teams/{id}/goals" "Set a goal on a team" S [SIGNED_BODY, &["NotAdmitted", "TeamUnknown", "evidence_missing", "goal_reused", "reminder_needs_deadline"]];
    POST "/goals/{goal}/mark" "Judge a goal" G [AGENT, SIGNED_BODY, &["evidence_missing", "goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement"]];
    POST "/goals/{goal}/active" "Suspend or resume a standing aim" G [AGENT, SIGNED_BODY, &["goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement", "goals_unavailable", "TeamsUnavailable", "TeamUnknown"]];
    POST "/goals/{goal}/words" "Reword a standing aim" G [AGENT, SIGNED_BODY, &["goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement", "goals_unavailable", "TeamsUnavailable", "TeamUnknown"]];
    GET "/service-accounts" "The service accounts" S [SIGNED] scope("service-account", "read", []);
    POST "/service-accounts" "Create a service account" S [SIGNED_BODY, &["NotAdmitted"], &["ServiceAccountReused"]] scope("service-account", "service-account.create", []);
    POST "/service-accounts/{id}/retire" "Retire a service account" S [SIGNED_BODY, &["ServiceAccountRetired", "ServiceAccountUnknown"]] scope("service-account", "service-account.retire", ["id"]);
    GET "/tree" "The caller's owned and led teams and descendants" G [AGENT, SIGNED, &["TeamsUnavailable", "RolesUnavailable", "ProvisioningUnavailable", "RuntimeUnavailable", "BudgetsUnavailable", "ConfigurationUnavailable", "goals_unavailable"]] scope("team", "read", []);
    GET "/teams" "Every team" S [SIGNED, &["NoPerson"]] scope("team", "read", []);
    POST "/teams" "Create a team" G [SIGNED_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotAdmitted", "team_parent_cycle", "team_lead_not_member", "TeamsUnavailable", "HoldingNotHeld", "NotHeld", "Revoked", "TeamUnknown", "NoPerson"]] scope("team", "team.create", []);
    GET "/teams/{id}" "One team" S [SIGNED, &["NoPerson", "TeamUnknown"]] scope("team", "read", ["id"]);
    POST "/teams/{id}/members" "Add a team member" G [SIGNED_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["AgentNotVisible", "NotAdmitted", "not_permitted", "HoldingNotHeld", "NotHeld", "Revoked", "TeamUnknown", "TeamsUnavailable", "NoPerson"]] scope("team", "team.member.add", ["id"]);
    POST "/teams/{id}/members/{member}/remove" "Remove a member" S [SIGNED_BODY] scope("team", "team.member.remove", ["id"]);
    POST "/teams/{id}/members/{member}/confirm" "Confirm a held membership" S [ADMIN_BODY, &["TeamsUnavailable"]] scope("team", "team.member.confirm", ["id"]);
    POST "/teams/{id}/nesting" "Replace a team parent and lead" S [SIGNED_BODY, &["NotAdmitted", "team_parent_cycle", "team_lead_not_member", "TeamsUnavailable"]] scope("team", "team.parent.set", ["id"]);
    POST "/teams/{id}/retire" "Retire a team" S [SIGNED_BODY] scope("team", "team.retire", ["id"]);
    GET "/resources" "The resources grants are on" S [SIGNED, &["NotAdmitted"]] scope("resource", "read", []);
    GET "/secrets" "The secrets" S [SIGNED, &["NoPerson", "SecretsUnavailable"]] scope("secret", "read", []);
    GET "/secrets/grants" "The secrets' grants" S [SIGNED] scope("secret", "read", []);
    GET "/secrets/audit" "The secrets' audit" S [SIGNED] scope("secret", "read", []);
    GET "/secrets/revocation" "The secrets' revocations" S [SIGNED, &["RequestMalformed"]] scope("secret", "read", []);
    GET "/secrets/settings" "The secrets' settings" S [SIGNED, &["RequestMalformed"]] scope("secret", "read", []);
    GET "/secrets/handles" "The secret handles" S [SIGNED, &["RequestMalformed"]] scope("secret", "read", []);
    POST "/secrets/scope" "Scope a secret" S [SIGNED_BODY, &["LendingNotPermitted"]] scope("secret", "secret.scope", []);
    POST "/secrets/recipients" "A secret's recipients" S [SIGNED_BODY] scope("secret", "secret.recipients", []);
    POST "/secrets/drop" "Drop a secret handle" S [SIGNED_BODY] scope("secret", "secret.drop", []);
    POST "/secrets/add" "Add a secret with its value" S [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "SecretExists", "SecretRetired", "OperationReused", "RouteInvalid", "ValueEmpty"]] scope("secret", "secret.add", []);
    POST "/secrets/replace" "Change a secret's value" S [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "LendingNotPermitted", "ValueEmpty"]] scope("secret", "secret.replace", []);
    POST "/secrets/retire" "Retire a secret" S [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "LendingNotPermitted"]] scope("secret", "secret.retire", []);
    GET "/canvas" "The caller's canvas: their arrangement and saved layouts" S [SIGNED, &["NoPerson", "CanvasUnavailable"]];
    PUT "/canvas" "Keep the caller's arrangement" S [SIGNED_BODY, &["NoPerson", "CanvasUnavailable"]];
    POST "/canvas/layouts" "Save an arrangement as a layout of the caller's, by name" S [SIGNED_BODY, &["NoPerson", "CanvasRefused", "CanvasUnavailable"]];
    POST "/canvas/layouts/remove" "Remove a layout of the caller's, by name" S [SIGNED_BODY, &["NoPerson", "CanvasRefused", "CanvasUnavailable"]];
    GET "/sessions" "The caller's sessions" S [SIGNED, &["NoPerson"]];
    POST "/sessions/{id}/end" "End one's own session" S [SIGNED, &["SessionUnknown"], &["NoPerson"]];
    GET "/directory/people/{id}/sessions" "A person's sessions" S [ADMIN, &["IdentityUnknown"]] scope("person", "read", ["id"]);
    POST "/directory/people/{id}/sessions/{session}/end" "End a session" S [ADMIN, &["SessionUnknown"], &["IdentityUnknown"]] scope("session", "person.session.end", ["session"]);
    GET "/configuration" "The service's configuration" S [ADMIN, &["ConfigurationUnavailable"]] scope("configuration", "read", []);
    PUT "/configuration" "Set the organisation zone at its current version" S [ADMIN_BODY, &["ConfigurationMalformed", "ConfigurationVersionConflict", "ConfigurationZoneRefused", "ConfigurationUnavailable", "NoPerson"]] scope("configuration", "configuration.zone.set", []);
    GET "/agents/{id}/memory" "An agent's memory" S [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/certificates" "An agent's certificates" S [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/certificates" "Issue a certificate" S [ADMIN_BODY, &["AgentNotVisible", "CertificateReused"], &["CertificateReused"]] scope("agent", "agent.certificate.issue", ["id"]);
    POST "/agents/{id}/certificates/{serial}/withdrawal" "Withdraw one" S [ADMIN_BODY, &["CertificateUnknown", "CertificateWithdrawn", "CertificatesUnavailable"], &["CertificateWithdrawn"]] scope("agent", "agent.certificate.withdraw", ["id"]);
    POST "/agents/{id}/start" "Start an agent" S [SIGNED_BODY, START_BUDGET, &["AgentHasNoPolicy", "PolicyUnavailable", "everything_stopped"]] scope("agent", "agent.start", ["id"]);
    POST "/launch-records/{id}/start-again" "Start a launch again" S [SIGNED_BODY, &["AgentHasNoPolicy", "PolicyUnavailable", "AgentNotActive", "AgentNotVisible", "CertificatesUnavailable", "everything_stopped"]] scope("launch-record", "launch-record.start-again", ["id"]);
    POST "/launch-records/{id}/withdraw" "Withdraw a launch" S [SIGNED_BODY] scope("launch-record", "launch-record.withdraw", ["id"]);
    GET "/launch-records/{id}/state" "A launch's state" S [SIGNED] scope("launch-record", "read", ["id"]);
    GET "/changes" "Wait for the next change signal" S [SIGNED, &["RequestMalformed", "RuntimeUnavailable"]] scope("change", "read", []);
    GET "/mcp" "MCP server stream availability" G [SIGNED];
    POST "/mcp" "MCP calls through the admitted HTTP router" G [SIGNED, AGENT, &["GrantTokenCookieConflict", "TokenScopeUndeclared", "TokenHolderNotAgent", "TokenUnknown"]];
    GET "/surface-contract" "The surface registration and computer admission contract" P [];
    GET "/openapi.json" "This document" P [];
};
