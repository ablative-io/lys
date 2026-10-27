//! The route table's untyped entries and the authentication every entry of
//! the table is written with; the refusal sets are `openapi_refusals.rs`'s.
//! `openapi.rs` turns them into the document. The door reads each entry's
//! scope and the closed list of routes a person keeps from every agent,
//! [`HUMAN_ONLY`] (`route_actions.rs`, `signed_first.rs`).

use lys_openapi::{Auth, Method};
use lys_runner::console_stop::PATH as CONSOLE_PATH;

use crate::openapi_refusals::{
    ADMIN, ADMIN_BODY, AGENT, APP_PERSON, BUDGET_READ, BUDGET_SET, GRANT_ASKED, GRANT_MADE,
    GRANT_READ, MACHINE_AGENTS, PERSON, PROFILE_REVIEW, RECORDED, REPORTING, RESTART, SIGNED,
    SIGNED_BODY, START_BUDGET, START_RUNNER, UNANSWERED, VERSION,
};

pub(crate) const GET: Method = Method::Get;
pub(crate) const POST: Method = Method::Post;
pub(crate) const PUT: Method = Method::Put;

/// Anyone.
pub(crate) const P: &[Auth] = &[Auth::Public];
/// A personal session; the operator has no personal account.
pub(crate) const C: &[Auth] = &[Auth::Session, Auth::Bearer];
/// A signed-in person or the install's operator.
pub(crate) const S: &[Auth] = &[Auth::Session, Auth::Bearer, Auth::Operator];
/// A signed-in person, or an app or registrar through its credential.
pub(crate) const A: &[Auth] = &[Auth::Session, Auth::Bearer, Auth::Operator];
/// An agent's signed request, or a signed-in person.
pub(crate) const G: &[Auth] = &[
    Auth::AgentSignature,
    Auth::Session,
    Auth::Bearer,
    Auth::Operator,
];
/// An agent's signed request, or a personal session; the operator has no
/// personal account.
pub(crate) const GC: &[Auth] = &[Auth::AgentSignature, Auth::Session, Auth::Bearer];
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

mod admission;
pub use admission::HUMAN_ONLY;
pub(crate) use admission::{declared_scope, human_only, listed_human_only, token_scope};

/// Whether `row`'s own handler judges an agent's call, its signature and
/// its grants, so the door does not judge it again: every route that
/// answers the agent signature's own refusals, and the MCP envelope, whose
/// dispatched route is judged. Every other route an agent reaches is judged
/// at the door by its declared scope (`route_actions::admit`).
pub(crate) fn judged_by_handler(row: &E) -> bool {
    row.4.contains(&crate::openapi_refusals::AGENT) || row.1 == "/mcp"
}

/// [`judged_by_handler`] for the route `method path`, the path as the table
/// writes it; false for a route the table does not name.
pub(crate) fn handler_judges(method: &str, path: &str) -> bool {
    TABLE.iter().any(|row| {
        row.1 == path && method.eq_ignore_ascii_case(row.0.word()) && judged_by_handler(row)
    })
}

/// One route of the table as the agent route parity reads it (AGENTS-019 R4).
#[derive(Debug, Clone, Copy)]
pub struct Reach {
    /// Its method, as the table's word.
    pub method: &'static str,
    /// Its path, each parameter as `{name}`.
    pub path: &'static str,
    /// Whether a person reaches it: its doors take a session.
    pub person: bool,
    /// Whether it admits an agent's signature.
    pub agent: bool,
    /// Whether it declares the scope an agent is judged by at the door.
    pub scoped: bool,
    /// Whether its own handler judges an agent's call ([`judged_by_handler`]).
    pub handler: bool,
}

/// Every route of the table, as the agent route parity reads it.
#[must_use]
pub fn reaches() -> Vec<Reach> {
    TABLE
        .iter()
        .map(|row| Reach {
            method: row.0.word(),
            path: row.1,
            person: row.3.contains(&Auth::Session),
            agent: row.3.contains(&Auth::AgentSignature),
            scoped: row.5.is_some(),
            handler: judged_by_handler(row),
        })
        .collect()
}

#[cfg(test)]
#[path = "route_scope_tests.rs"]
mod tests;

/// Every route of the table; `openapi_types.rs` names the types each takes and answers.
pub(crate) const TABLE: &[E] = entries! {
    GET "/authority" "The authority this service speaks for" P [];
    GET "/health" "That this service is serving, with its name and build" P [];
    GET "/waits" "Outbound calls in flight, oldest first; after waits for a change event" S [ADMIN, &["waits_unavailable"]];
    POST "/waits/{id}/cancel" "An administrator cancels one outbound call" S [ADMIN, &["wait_unknown", "waits_unavailable"]];
    GET "/clients" "The client platforms and commits held by this install" P [&["clients_unreadable"]];
    GET "/clients/{platform}/{name}" "Download a held client executable with its commit and SHA-256" P [&["client_platform_unknown", "client_file_unknown", "client_digest_mismatch", "clients_unreadable"]];
    GET "/login" "Redirect to the sign-in screen" P [];
    GET "/callback" "Finish a sign-in and begin a session" P [&["SignInStateUnknown", "SignInFailed", "SignInRefused", "SignInNotTheirs", "NotAdmittedYet", "AccountDisabled", "SignInCancelled", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable"]];
    POST "/setup" "Finish the administrator's first-run setup" S [SIGNED_BODY, &["AlreadyBootstrapped", "NotAdmitted", "OperationReused", "ProfileInvalid"]];
    POST "/people" "Register a person" G [ADMIN_BODY, &["OperationReused"]] scope("person", "person.create", []);
    POST "/agents" "Register an own agent with a person's app pass or sign-in; administration requires sign-in" G [AGENT, &["not_permitted"], APP_PERSON, SIGNED_BODY, REPORTING, &["NotAdmitted", "credential_refused", "ServiceAccountUnknown", "NotHeld", "PolicyUnavailable"]] scope("agent", "agent.create", []);
    POST "/drafts" "Record an agent's immutable draft" G [SIGNED_BODY, AGENT, &["NoPerson", "NotAdmitted", "DraftChangeInvalid", "OperationReused"]];
    GET "/drafts" "The drafts the signed-in person may decide" GC [SIGNED, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("draft", "read", []);
    GET "/dashboard" "The caller's own agents, how each is going, what waits on the caller, and what the caller's agents registered and started for them" C [SIGNED, &["NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]];
    GET "/operations/{operation}" "Whether Lys recorded a change the caller sent under an operation, and what it made; another person's is refused" GC [SIGNED, &["NoPerson", "NotAdmitted", "IdentifierMalformed", "RequestsUnavailable", "RolesUnavailable"]] scope("operation", "read", ["operation"]);
    GET "/waiting" "Everything waiting on the caller's decision, oldest first; a part the caller may not read is its refusal in place" GC [SIGNED] scope("draft", "read", []);
    POST "/drafts/{id}/approve" "Approve and carry out the prepared action through its admitted route" GC [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"], &["own_draft"]] scope("draft", "draft.decide", ["id"]);
    POST "/drafts/{id}/refuse" "Refuse one immutable draft" GC [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"], &["own_draft"]] scope("draft", "draft.decide", ["id"]);
    POST "/drafts/{id}/correct" "Refuse and save the responsible person's correction" GC [SIGNED_BODY, &["AgentSignatureRefused", "NotAdmitted", "NoPerson", "DraftNotFound", "DraftHashMismatch", "DraftNotPending", "DraftChangeInvalid", "OperationReused", "IdentifierMalformed"], &["own_draft"]] scope("draft", "draft.decide", ["id"]);
    POST "/agents/{id}/reports-to" "Change an agent's reporting edge" G [ADMIN_BODY, REPORTING] scope("agent", "agent.reports-to.set", ["id"]);
    GET "/identities" "Every identity of the caller's organisation" G [ADMIN] scope("identity", "read", []);
    GET "/identities/{id}" "One identity of the caller's organisation" G [ADMIN, &["IdentifierMalformed", "IdentityUnknown"]] scope("identity", "read", ["id"]);
    POST "/identities/{id}/profile" "Change an identity's profile" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "IdentityUnknown", "OperationReused", "ProfileInvalid"]] scope("person", "person.profile.set", ["id"]);
    POST "/identities/{id}/transitions" "Move an identity's state; a person's app pass only activates an own registered agent" G [APP_PERSON, ADMIN_BODY, &["TransitionRefused"]] scope("identity", "identity.transition", ["id"]);
    POST "/people/{id}/logins/move" "Move a sign-in to its rightful person in one recorded batch; grants stay on their people" G [ADMIN_BODY, &["LoginNotBound", "SameTarget", "NotSameOrganisation", "NotAdmitted"]] scope("person", "person.login.bind", ["id"]);
    POST "/people/{id}/logins" "Bind a login to a person" S [ADMIN_BODY, &["WithheldFromAgents"]] scope("person", "person.login.bind", ["id"]);
    GET "/people/{id}/admission" "Whether an admission link is waiting for this person" S [ADMIN, &["NoPerson", "IdentifierMalformed", "DirectoryUnavailable"]] scope("organisation", "organisation.administer", []);
    POST "/people/{id}/admission/withdraw" "Withdraw this person's unspent admission link, retaining its digest and actor" S [ADMIN, &["NoPerson", "IdentifierMalformed", "NoUnspentAdmission", "DirectoryUnavailable"]] scope("organisation", "organisation.administer", []);
    POST "/people/admit" "Admit a person in one act: register, issuer account by email, bind, activate, a first root grant only when one is asked for, and the added letter; a person's app pass admits only with a first grant on its app's own kind" G [AGENT, APP_PERSON, ADMIN_BODY, &["EmailHeldByAnother", "organisation_not_own", "RelationUnknown", "OperationReused", "mail_not_sent", "app_admit_grant_required", "not_your_app", "kind_not_registered"]] scope("organisation", "organisation.admit", []);
    POST "/organisations/found" "A stranger founds an organisation: the organisation and its founder recorded, a proving link written to the founder's email; one answer shape to everyone" P [&["RequestMalformed", "mail_not_sent"]];
    GET "/organisations/found/{proof}/whom" "Whose record a proof opens, with a masked address, without spending it" P [&["proof_refused"]];
    POST "/organisations/found/prove" "The proving link: the founder's account, binding, activation, the organisation's activation, the founder's administrator grant and their first session, the proof spent last" P [&["proof_refused", "already_admitted", "SignedInAsAnother", "SignInNotTheirs", "AccountRefused", "RequestMalformed"]];
    POST "/pair" "An agent with no identity asks for a pairing code over its fresh key: the four words to tell its person, the wait id and the address where the person enters them" P [&["pair_request_invalid"]];
    GET "/pair/{wait}" "The waiting agent's answer, held open until its person decides: its identity, certificate and the organisation's approved apps, or the decline; an unknown or already answered wait is refused at once" P [&["pair_unknown", "pair_declined"]];
    POST "/pair/{wait}/taken" "The agent has its answer: the decided pairing is removed, once; an undecided, unknown or already taken wait is refused at once" P [&["pair_unknown"]];
    GET "/pair/{code}/waiting" "The agent waiting with these four words, for the signed-in person who will decide" S [SIGNED, &["pair_unknown"]];
    POST "/pair/{code}/approve" "Approve a pairing: register the agent under the signed-in person, activate it, give it the roots named, certify its key and answer its wait with its identity and the organisation's approved apps" S [SIGNED_BODY, REPORTING, &["pair_unknown", "pair_request_invalid", "NotAdmitted", "ProfileInvalid", "TransitionRefused", "PolicyUnavailable", "CertificateReused", "CertificatesUnavailable", "AgentNotVisible", "DirectoryUnavailable", "apps_unavailable"], &["ExpiryBeyondSource", "UseOnly", "NotHolder", "IdentityNotActive", "ResponsibleMismatch", "SourceUnknown", "ActionsOutside", "PassOnBeyondSource", "RecipientRefused", "WithheldFromAgents", "MachineRefused", "OrganisationOutside", "RelationUnknown", "RootAuthorityRefused", "kind_not_registered", "app_not_approved", "app_retired", "ProjectionPending", "OperationUnresolved", "PermissionEngineUnavailable"]];
    POST "/pair/{code}/decline" "Decline a pairing: the agent's wait answers pair_declined and nothing of its key is kept" S [SIGNED, &["pair_unknown"]];
    GET "/me" "The signed-in caller and their organisation" C [SIGNED, &["NoPerson", "SetupRequired"]];
    PUT "/me/getting-started" "Keep the caller's getting-started answers, read back on GET /me" C [SIGNED_BODY, &["NoPerson", "DirectoryUnavailable"]];
    GET "/people" "The people the caller may see" G [SIGNED, &["NoPerson", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("person", "read", []);
    GET "/agents/{id}" "An agent the caller answers for" G [PERSON, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/directory/people" "Every person of the administrator's organisation" G [ADMIN, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("person", "read", []);
    GET "/directory/agents/{id}" "Any agent, for the administrator" G [ADMIN, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    POST "/grants/{id}/tokens" "Issue a credential for one grant" GC [SIGNED_BODY, &["GrantTokenResponsibleRequired", "GrantTokenExpiryInvalid", "GrantTokenUnavailable", "GrantTokenUnknown"]] scope("grant", "grant.delegate", ["id"]);
    POST "/grants/{id}/tokens/{token_id}/revoke" "Revoke one grant credential" GC [SIGNED, &["GrantTokenResponsibleRequired", "GrantTokenUnavailable", "GrantTokenUnknown"]] scope("grant", "grant.revoke", ["id"]);
    GET "/grants" "The grants the caller may see, within their organisation" G [SIGNED, &["NotAdmitted"]] scope("grant", "read", []);
    GET "/agent/grants" "The signed agent's own live grants and their chain admission" AGENT_ONLY [AGENT, &["DirectoryUnavailable", "LogUnavailable", "ServiceAccountsUnavailable", "CertificatesUnavailable"]];
    POST "/grants" "Pass on part of a grant, or standing as the signed-in administrator or as an agent holding administrator standing on its organisation, through its own signed message" G [AGENT, &["not_permitted"], GRANT_MADE, RECORDED, &["NoPerson", "RootAuthorityRefused", "grant_mode_on_hot_action"], &["ExpiryBeyondSource", "UseOnly", "NotAdmitted", "NotHolder", "IdentityNotActive", "ResponsibleMismatch", "DirectoryUnavailable", "SourceUnknown", "ActionsOutside", "PassOnBeyondSource", "RecipientRefused", "WithheldFromAgents", "MachineRefused", "OrganisationOutside"]] scope("grant", "grant.delegate", []);
    GET "/grants/model" "Lys's own permission model" G [SIGNED] scope("grant", "read", []);
    POST "/grants/roots" "Issue a root grant, outright or held by draft or by two" S [GRANT_MADE, RECORDED, &["RelationUnknown"], &["RootAuthorityRefused", "RootOwnedElsewhere", "OrganisationOutside", "WithheldFromAgents", "MachineRefused", "grant_mode_on_hot_action", "mail_not_sent"]];
    POST "/grants/agent-roots" "Record the administrator's roots for giving people and agents access" S [ADMIN, &["NoPerson", "RelationUnknown"], &["RootAuthorityRefused", "WithheldFromAgents"]];
    POST "/grants/check" "Check, and record, an exercise" G [GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked"]] scope("grant", "grant.check", []);
    POST "/grants/why" "Why the caller may act" G [GRANT_ASKED, UNANSWERED, &["NotHeld"]] scope("grant", "grant.why", []);
    POST "/grants/who" "Who, within the caller's organisation, may act on a resource" G [GRANT_ASKED, UNANSWERED] scope("grant", "grant.who", []);
    POST "/grants/held" "Whether anyone may act on a resource, naming no one" G [GRANT_ASKED, UNANSWERED, &["NotAdmitted", "not_your_app", "credential_refused", "ServiceAccountsUnavailable"]] scope("grant", "grant.who", []);
    POST "/grants/reach" "Who, within the caller's organisation, may act on each of many resources" G [GRANT_ASKED, UNANSWERED] scope("grant", "grant.reach", []);
    GET "/grants/cannot-give" "What the caller cannot pass on" G [GRANT_READ, &["IdentityUnknown", "NotHolder", "IdentityNotActive"]] scope("grant", "read", []);
    GET "/grants/{id}" "One grant the caller may see" G [GRANT_READ, &["GrantIdMalformed"]] scope("grant", "read", ["id"]);
    POST "/grants/{id}/revoke" "Revoke a grant and all it derives" G [AGENT, &["not_permitted"], GRANT_READ, RECORDED, &["GrantUnknown", "RevokeRefused"]] scope("grant", "grant.revoke", ["id"]);
    GET "/receipts/{index}" "A directory receipt, publicly" P [&["RequestMalformed", "InstallEntry"]];
    GET "/service-key" "The service's public key" P [];
    GET "/reviews" "The grants due for review" G [SIGNED, &["NoPerson"]] scope("review", "read", []);
    POST "/reviews/agents/{agent}/keep" "Keep exactly an agent's shown due grants in one durable act" G [SIGNED_BODY, &["GrantNotDue", "GrantNotVisible", "AgentMismatch", "ReviewReused", "ReviewsUnavailable", "NoPerson", "ReviewerOnly"]] scope("review", "review.keep", ["agent"]);
    POST "/reviews/{grant}/keep" "Keep a grant under review" G [SIGNED_BODY, &["GrantNotDue", "GrantNotVisible", "ReviewReused"], &["NoPerson", "ReviewerOnly"]] scope("review", "review.keep", ["grant"]);
    GET "/roles" "Every role" G [SIGNED] scope("role", "read", []);
    POST "/roles" "Make a role" G [ADMIN_BODY, &["RelationUnknown", "RoleReused"]] scope("role", "role.create", []);
    GET "/roles/{id}" "One role" G [SIGNED] scope("role", "read", ["id"]);
    POST "/roles/{id}/versions" "Revise a role" G [ADMIN_BODY, &["RoleReused"]] scope("role", "role.revise", ["id"]);
    POST "/roles/{id}/holders" "Assign a role" G [ADMIN_BODY, &["HolderUnknown"], &["RoleHeld", "RoleUnknown"]] scope("role", "role.holder.assign", ["id"]);
    POST "/roles/{id}/holders/{holder}/move" "Move a holding" G [ADMIN_BODY, &["HoldingOver", "RoleVersionUnknown"], &["HolderUnknown", "HoldingChanged"]] scope("role", "role.holder.move", ["id"]);
    POST "/roles/{id}/holders/{holder}/end" "End a holding" G [ADMIN_BODY] scope("role", "role.holder.end", ["id"]);
    GET "/requests" "The access requests" G [SIGNED, &["NoPerson", "NotAdmitted", "RequestMalformed", "TeamUnknown", "TeamsUnavailable"]] scope("request", "read", []);
    POST "/requests" "Ask for access" G [SIGNED_BODY, &["RelationUnknown"], &["NoPerson", "NotAdmitted", "RequestReused", "WithheldFromAgents"]] scope("request", "request.create", []);
    POST "/requests/{id}/approve" "Approve an access request; one whose relation the model no longer defines is refused by name" S [SIGNED_BODY, &["NotAdmitted", "RequestDecided", "RelationUnknown"], &["SourceUnknown", "WithheldFromAgents", "RootAuthorityRefused", "OrganisationOutside"]] scope("request", "request.approve", ["id"]);
    POST "/requests/{id}/decline" "Decline an access request" S [SIGNED_BODY, &["RequestDecided", "RequestUnknown"], &["NotAdmitted"]] scope("request", "request.decline", ["id"]);
    POST "/requests/{id}/reconcile" "Settle an approval" G [SIGNED_BODY, &["NoPerson", "NotAdmitted", "RelationUnknown"]] scope("request", "request.reconcile", ["id"]);
    GET "/connections" "What the service is connected to" G [ADMIN] scope("connection", "read", []);
    GET "/sign-in-providers" "The sign-in providers" S [ADMIN, &["SignInProvidersUnavailable"]];
    POST "/sign-in-providers" "Set a sign-in provider" S [ADMIN_BODY, &["ProviderRefused"]];
    POST "/link-audit" "Deliver a link-audit record" G [AGENT, &["NotAdmitted", "NotSignedIn", "RequestMalformed"]];
    POST "/link-audit/person" "Look up a link-audit holder" G [AGENT, &[ "LoginUnbound", "NotAdmitted", "NotSignedIn", "RequestMalformed", ]];
    GET "/network" "The machines" G [SIGNED, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable", "NoPerson", "NetworkUnavailable"]] scope("machine", "read", []);
    GET "/harnesses" "The programmes Lys describes and their reviewed builds" G [SIGNED_BODY, &["NotAdmitted", "NoPerson", "not_your_computer", "MachineUnknown", "MachineRetired", "NetworkUnavailable", "RuntimeUnavailable", "harness_programs_unreadable", "runner_reply_malformed", "ProvisioningUnavailable"]] scope("harness", "read", []);
    GET "/harnesses/plugins" "The Claude Code plugins installed in one config folder of a computer, as its runner reads them" G [ADMIN, &["RequestMalformed", "MachineUnknown", "runner_absent"], &["runner_request_unsigned", "runner_request_malformed", "runner_protocol_mismatch", "runner_request_replayed", "runner_request_misaddressed", "runner_unreachable", "runner_reply_malformed", "runner_stopping", "plugins_unreadable", "plugin_not_installed", "runner_reply_malformed"]] scope("harness", "read", []);
    POST "/network/machines" "Name an owned computer, or administer a machine" G [SIGNED_BODY, &["NotAdmitted", "MachineReused", "NoPerson", "not_your_computer", "agent_not_yours", "NetworkUnavailable"], &["IdentifierMalformed", "TeamUnknown", "TeamRetired", "TeamsUnavailable"]] scope("machine", "machine.create", []);
    GET "/network/machines/{id}/connection" "Read a runner's reported connection schedule" G [SIGNED, &["NoPerson", "not_your_computer", "MachineUnknown", "runner_socket_unavailable", "runner_reconnect_control_failed"]] scope("machine", "machine.read", ["id"]);
    POST "/network/machines/{id}/reconnect" "End a connected runner's reconnect wait" G [SIGNED_BODY, &["NoPerson", "not_your_computer", "MachineUnknown", "runner_reconnect_unavailable", "runner_reconnect_control_failed"]] scope("machine", "machine.runner.set", ["id"]);
    POST "/runner/dial/{machine}/control" "A signed runner's standing reconnect control stream" P [&["runner_dial_refused", "runner_dial_stale", "runner_dial_capacity", "RequestMalformed"]];
    POST "/runner/dial/{machine}/connection" "A signed runner reports its failed dial schedule" P [&["runner_dial_refused", "runner_dial_stale", "runner_dial_capacity", "RequestMalformed"]];
    GET "/network/machines/{id}/completions" "List a computer's retained completion tickets" G [SIGNED, &["NoPerson", "not_your_computer", "MachineUnknown", "NetworkUnavailable"]] scope("machine", "machine.read", ["id"]);
    GET "/network/machines/{id}/completions/{ticket}" "Read a retained runner completion" G [SIGNED, &["NoPerson", "not_your_computer", "MachineUnknown", "NetworkUnavailable", "runner_completion_unknown"]] scope("machine", "machine.read", ["id"]);
    POST "/network/machines/{id}/completions/{ticket}/acknowledge" "Acknowledge receipt of a runner completion" G [SIGNED_BODY, &["NoPerson", "not_your_computer", "MachineUnknown", "NetworkUnavailable"]] scope("machine", "machine.completion.acknowledge", ["id"]);
    POST "/network/machines/{id}/retire" "Retire a machine" G [ADMIN_BODY, &["MachineUnknown"]] scope("machine", "machine.retire", ["id"]);
    POST "/network/machines/{id}/team" "Assign or clear a computer's owning team" G [SIGNED_BODY, &["NotAdmitted", "NoPerson", "IdentifierMalformed", "MachineUnknown", "MachineRetired", "MachineTeamReused", "TeamUnknown", "TeamRetired", "TeamsUnavailable"]] scope("machine", "machine.team.set", ["id"]);
    POST "/network/machines/{id}/agents" "Allow an own agent on an own computer, or administer its agents" G [AGENT, &["not_permitted"], SIGNED_BODY, MACHINE_AGENTS, &["NoPerson", "not_your_computer", "agent_not_yours", "IdentifierMalformed", "AgentNotVisible", "MachineUnknown", "MachineRetired", "MachineWithoutRuntime", "NetworkUnavailable", "RuntimeUnavailable"]] scope("machine", "machine.agent.set", ["id"]);
    GET "/agents/{id}/provisioning" "An agent's profile" G [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/mcp-requests" "An agent's MCP requests" G [SIGNED, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "ProvisioningReused"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/mcp-requests" "Ask for a declared MCP server" G [SIGNED_BODY, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "ProvisioningReused", "ProfileNotReviewed", "RequestReused", "mcp_server_unknown", "mcp_server_held"]] scope("agent", "agent.mcp-request.create", ["id"]);
    POST "/agents/{id}/mcp-requests/{request}/approve" "Approve an MCP request within remit" G [SIGNED_BODY, AGENT, &["AgentNotVisible", "NoPerson", "McpRequestsUnavailable", "ProvisioningUnavailable", "TeamsUnavailable", "ProfileNotReviewed", "ProvisioningReused", "RequestUnknown", "RequestDecided", "RequestReused", "mcp_beyond_remit", "mcp_server_held", "McpSettingUnrepresentable"]] scope("agent", "agent.mcp-request.approve", ["id"]);
    POST "/agents/{id}/provisioning" "Set an own agent's profile" G [AGENT, &["not_permitted"], &["ProvisioningUnavailable"], SIGNED_BODY, &["NotAdmitted", "NoPerson", "MachineUnknown", "MachineRetired", "not_your_computer", "NetworkUnavailable", "RuntimeUnavailable", "harness_program_unknown", "runner_unreachable", "harness_programs_unreadable", "runner_reply_malformed", "ProvisioningChanged"], &["AgentNotVisible", "ProvisioningReused"], &["McpCredentialInline", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown", "ModelWireMissing"]] scope("agent", "agent.provisioning.set", ["id"]);
    GET "/skills" "The skills Lys keeps" G [SIGNED] scope("skill", "read", []);
    POST "/skills" "Keep a skill's text" G [&["ProvisioningUnavailable"], ADMIN_BODY] scope("skill", "skill.keep", []);
    POST "/agents/{id}/provisioning/{version}/review" "Review a profile" G [AGENT, &["not_permitted"], &["provisioning_review_own_version"], &["ProvisioningUnavailable"], ADMIN_BODY, PROFILE_REVIEW] scope("agent", "agent.provisioning.review", ["id"]);
    POST "/agents/{id}/provisioning/{version}/seat-program-review" "Review changed seat program digests" G [AGENT, &["not_permitted"], &["provisioning_review_own_version"], &["ProvisioningUnavailable"], ADMIN_BODY, PROFILE_REVIEW] scope("agent", "agent.provisioning.review", ["id"]);
    POST "/agents/{id}/start-command" "An agent's start command" G [AGENT, &["not_permitted"], SIGNED_BODY, &["AgentNotVisible", "MachineCannotReach"], &["LaunchRecordMissing", "MachineNotForAgent", "MachineRetired", "MachineUnknown", "MachineWithoutRuntime", "NotAdmitted", "WorkingFolderUnnamed"], &["HarnessUndeclared", "LaunchUnrenderable", "McpHandleUnsupported", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "ProfileNotReviewed", "SkillUnknown"], &["SecretsUnavailable"], START_RUNNER, START_BUDGET, &["everything_stopped"]] scope("agent", "agent.start-command", ["id"]);
    GET "/agents/{id}/start-command" "Preview the previous conversation's working folder" G [SIGNED, &["RequestMalformed", "IdentifierMalformed", "AgentNotVisible", "NotAdmitted", "RuntimeUnavailable", "pty_resume_unavailable"]] scope("agent", "agent.start-command", ["id"]);
    POST "/agents/{id}/restart" "Restart an agent on its latest reviewed profile" G [AGENT, &["not_permitted"], SIGNED_BODY, RESTART, &["everything_stopped"]] scope("agent", "agent.restart", ["id"]);
    POST "/agents/{id}/runtime/sessions/{session}/reports" "A runtime report" G [AGENT, &["NotAdmitted"], &["AgentNotVisible", "RequestMalformed", "RuntimeSessionUnknown"]] scope("agent", "agent.runtime.report", ["id"]);
    GET "/agents/{id}/control-sessions" "An agent's control session identities, stopped sessions included" G [SIGNED, &["AgentNotVisible", "RuntimeUnavailable", "RequestMalformed"]] scope("agent", "read", ["id"]);
    GET "/runtime/sessions/{session}/controls" "A session's current control identifiers" G [SIGNED, &["RuntimeSessionUnknown", "runner_absent", "runner_unreachable", "control_status_mismatch"]] scope("runtime-session", "read", ["session"]);
    GET "/runtime/sessions/{session}/control-receipts" "A bounded page of session control evidence" G [SIGNED, &["RuntimeSessionUnknown", "runner_absent", "RequestMalformed", "control_page_mismatch"]] scope("runtime-session", "read", ["session"]);
    POST "/runtime/sessions/{session}/control-receipts/{operation}/reconcile" "The responsible person's decision about uncertain delivery" G [SIGNED_BODY, &["NoPerson", "RuntimeSessionUnknown", "not_permitted", "control_not_uncertain", "control_already_reconciled"]] scope("runtime-session", "runtime-session.reconcile-control", ["session"]);
    GET "/goals/{goal}/resends/{prior}" "The kept resend of one uncertain goal delivery" G [SIGNED, &["NoPerson", "goal_unknown", "goal_prior_unknown", "control_not_uncertain", "goal_resend_invalid"]] scope("goal", "goal.resend", ["goal"]);
    POST "/goals/{goal}/resend" "A distinct authorised occurrence naming possible prior delivery" G [SIGNED_BODY, &["NoPerson", "goal_unknown", "goal_authority_revoked", "goal_prior_already_resent", "goal_resend_kept_reconciliation_unavailable"]] scope("goal", "goal.resend", ["goal"]);
    GET "/agents/{id}/runtime/sessions" "An agent's runtime sessions" G [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/runtime/sessions" "Every runtime session" G [ADMIN] scope("runtime-session", "read", []);
    GET "/runtime/message-edges" "Caller-visible message addresses from the message service, mapped to Lys identities" G [PERSON, &["MessageEdgesUnavailable"]] scope("runtime-session", "read", []);
    POST "/runtime/found/{session}/reports" "Report a found session" G [ADMIN_BODY, &["MachineUnknown"], &["RuntimeSessionStopped", "RuntimeSessionUnknown"]] scope("runtime-session", "runtime-session.found.report", ["session"]);
    GET "/runtime/found" "The sessions found" G [ADMIN] scope("runtime-session", "read", []);
    POST "/agents/{id}/stop" "Suspend an agent's access, withdraw certificates and end its sessions; ordinary Stop uses the runtime session end route" G [AGENT, &["not_permitted"], SIGNED_BODY, &["AgentNotVisible", "StopReused"]] scope("agent", "agent.stop", ["id"]);
    GET "/agents/{id}/stops" "An agent's stops" G [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/runtime/stop-everything" "Whether everything is stopped, by whom, when and why" G [SIGNED, &["cord_unavailable"]] scope("runtime-session", "read", []);
    POST "/runtime/stop-everything" "Stop every session on every computer and refuse every start" G [ADMIN_BODY, &["IdentifierMalformed", "cord_reused", "cord_unavailable", "RuntimeUnavailable"]] scope("runtime-session", "runtime.stop-everything", []);
    POST CONSOLE_PATH "Stop every session under the service's own console signature" CONSOLE_ONLY [SIGNED_BODY, &["console_signature_refused", "IdentifierMalformed", "cord_reused", "cord_unavailable", "RuntimeUnavailable"]];
    POST "/runtime/stop-everything/release" "Let agents start again" G [ADMIN_BODY, &["IdentifierMalformed", "cord_not_pulled", "cord_reused", "cord_unavailable"]] scope("runtime-session", "runtime.stop-everything", []);
    POST "/budgets/person/{id}/confirm" "Confirm a legacy personal budget" G [ADMIN_BODY, &["BudgetsUnavailable", "BudgetVersionConflict", "budget_invalid", "not_permitted"]] scope("person", "budget.set", ["id"]);
    GET "/budgets/{kind}/{id}" "A holder's limits and measured usage" G [SIGNED, BUDGET_READ] scope("budget", "read", ["kind", "id"]);
    PUT "/budgets/{kind}/{id}" "Replace a holder's limit collection" G [SIGNED_BODY, BUDGET_READ, BUDGET_SET, AGENT, GRANT_ASKED, UNANSWERED, &["HoldingNotHeld", "NotHeld", "Revoked"]] scope("budget", "budget.set", ["kind", "id"]);
    GET "/teams/{id}/budget" "A team's limits and measured usage" G [SIGNED, BUDGET_READ] scope("team", "read", ["id"]);
    PUT "/teams/{id}/budget" "Replace a team's limit collection" G [SIGNED_BODY, BUDGET_READ, BUDGET_SET, AGENT, GRANT_ASKED, UNANSWERED, &["HoldingNotHeld", "NotHeld", "Revoked"]] scope("team", "team.budget.set", ["id"]);
    POST "/runtime/sessions/{id}/input" "Type into a session" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "session_ended"]] scope("runtime-session", "runtime-session.input", ["id"]);
    GET "/agents/{id}/refusals" "An agent's refused tool calls" G [SIGNED, &["AgentNotVisible", "not_permitted"]] scope("agent", "read", ["id"]);
    POST "/runtime/sessions/{id}/input-bytes" "Write exact bytes to a session" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "session_ended"]] scope("runtime-session", "runtime-session.input-bytes", ["id"]);
    POST "/runtime/sessions/{id}/read-bytes" "Read a session's exact output bytes" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "read", ["id"]);
    POST "/runtime/sessions/{id}/keys" "Send a session keys" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.keys", ["id"]);
    POST "/runtime/sessions/{id}/read" "Read a session's screen" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "read", ["id"]);
    POST "/runtime/sessions/{id}/wait" "Wait for a session to show a pattern" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "read", ["id"]);
    POST "/runtime/sessions/{id}/resize" "Resize a session" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.resize", ["id"]);
    POST "/runtime/sessions/{id}/compact" "Compact a session" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.compact", ["id"]);
    POST "/runtime/sessions/{id}/end" "End a session" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted"]] scope("runtime-session", "runtime-session.end", ["id"]);
    POST "/runtime/sessions/{id}/stop" "Ask a session to stop and wait for the runner's reap" G [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "runner_unreachable", "runner_reply_malformed", "operation_refused", "RuntimeUnavailable"]] scope("runtime-session", "runtime-session.end", ["id"]);
    POST "/runtime/sessions/{id}/end-now" "A person asks the runner to end a waiting process now" S [SIGNED_BODY, &["RuntimeSessionUnknown", "AgentNotVisible", "not_permitted", "runner_unreachable", "RuntimeUnavailable"]];
    POST "/agents/{id}/wake" "Wake an agent's live session" G [SIGNED_BODY, &["AgentNotVisible", "not_permitted", "no_live_session"]] scope("agent", "agent.wake", ["id"]);
    GET "/runtime/live" "The sessions still running" G [SIGNED, &["RequestMalformed", "TeamUnknown", "TeamsUnavailable", "AgentNotVisible", "IdentifierMalformed", "RuntimeSessionUnknown"]] scope("runtime-session", "read", []);
    GET "/network/machines/{id}/runner" "A machine's runner" G [SIGNED, &["NoPerson", "not_your_computer", "MachineUnknown", "MachineRetired", "NetworkUnavailable"]] scope("machine", "read", ["id"]);
    POST "/network/machines/{id}/runner" "Name a machine's runner" G [AGENT, &["not_permitted"], ADMIN_BODY, &["MachineUnknown", "runner_lys_taken", "runner_held_by_machine"]] scope("machine", "machine.runner.set", ["id"]);
    POST "/network/machines/{id}/folders" "The folders inside one folder of a computer" G [SIGNED_BODY, &["NetworkUnavailable", "NoPerson", "not_your_computer", "MachineRetired", "MachineUnknown", "runner_absent"], &["runner_request_unsigned", "runner_request_malformed", "runner_protocol_mismatch", "runner_request_replayed", "runner_request_misaddressed", "runner_unreachable", "runner_reply_malformed", "runner_stopping", "home_unknown", "folder_unreadable"]] scope("machine", "read", ["id"]);
    POST "/network/machines/{id}/choose-folder" "The folder picked in a computer's own folder dialog" G [SIGNED_BODY, &["NetworkUnavailable", "NoPerson", "not_your_computer", "MachineRetired", "MachineUnknown", "runner_absent"], &["runner_request_unsigned", "runner_request_malformed", "runner_protocol_mismatch", "runner_request_replayed", "runner_request_misaddressed", "runner_unreachable", "runner_reply_malformed", "runner_stopping", "home_unknown", "folder_invalid", "folder_choice_cancelled", "folder_dialog_unavailable"]] scope("machine", "read", ["id"]);
    GET "/runner/protocol" "The runner protocol" P [];
    POST "/runner/dial/{machine}/next" "A dialled runner's next request" P [&["runner_dial_refused", "runner_dial_stale", "runner_dial_capacity", "runner_unreachable"]];
    POST "/runner/dial/{machine}/replies/{ticket}" "A dialled runner's reply" P [&["runner_dial_refused", "runner_dial_stale", "runner_dial_capacity", "runner_unreachable", "runner_reply_expired", "runner_reply_duplicate", "runner_reply_unknown"]];
    POST "/runner/dial/{machine}/pass" "A joined machine's pass to an app, asked with its join's key" P [&["runner_dial_refused", "runner_dial_stale", "runner_dial_capacity", "runner_unreachable", "RequestMalformed", "HolderRetired", "NotAdmitted", "ProviderUnavailable", "NetworkUnavailable", "IdentifierMalformed", "DirectoryUnavailable"], UNANSWERED, &["credential_refused", "app_not_approved", "app_retired", "apps_unavailable"], &["grant_log_identity_unavailable", "grant_binding_unsupported", "grant_binding_revision_moved", "grant_binding_degraded"]];
    POST "/network/machines/{id}/join-code" "Give a one-time connection code for an own computer, or administer a join" G [AGENT, &["not_permitted"], SIGNED_BODY, &["NoPerson", "not_your_computer", "IdentifierMalformed", "MachineUnknown", "MachineRetired", "NetworkUnavailable", "RunnerJoinUnreachable", "RunnerJoinOperationReused"]] scope("machine", "machine.runner.set", ["id"]);
    POST "/network/machines/{id}/installer/{kind}" "Download an installer with a fresh connection for an own computer, or administer its installation" G [AGENT, &["not_permitted"], SIGNED_BODY, &["NoPerson", "not_your_computer", "MachineUnknown", "MachineRetired", "NetworkUnavailable", "RunnerJoinUnreachable", "RunnerJoinOperationReused", "RandomSourceUnavailable", "client_platform_unknown", "client_file_unknown", "client_digest_mismatch", "clients_unreadable", "installer_kind_unknown", "installer_build_failed", "installer_pending_invalid", "installer_commit_invalid", "installer_binary_missing", "installer_binary_unreadable", "installer_archive_limit", "installer_archive_path", "installer_archive_failed", "installer_staging_failed", "installer_cleanup_failed", "installer_pkgbuild_unavailable", "installer_pkgbuild_failed", "installer_productbuild_unavailable", "installer_productbuild_failed", "installer_package_unreadable", "installer_package_invalid"]] scope("machine", "machine.runner.set", ["id"]);
    POST "/runner/join" "Join another computer's runner with its connection code" P [&["RequestMalformed", "RunnerJoinRefused", "MachineRetired", "NetworkUnavailable"]];
    GET "/network/machine-identities" "The computers that joined, as machine identities, and the grants each holds" G [ADMIN, &["NetworkUnavailable", "IdentifierMalformed"]] scope("machine", "read", []);
    GET "/runner-receipts/{index}" "A runner act's receipt" P [&["RequestMalformed"]];
    GET "/agents/{id}/usage" "An agent's budget crossings and what came of each" G [SIGNED, BUDGET_READ, &["AgentNotVisible", "RuntimeSessionUnknown"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/calls" "An agent's model calls, newest first" G [SIGNED, BUDGET_READ, &["AgentNotVisible", "RequestMalformed"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/calls/{call}" "One model call whole: its record and both bodies" G [SIGNED, BUDGET_READ, &["AgentNotVisible", "not_permitted", "CallUnknown", "CallKeptElsewhere", "CallRecordsUnavailable"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/usage" "Report a use an agent made" G [SIGNED_BODY, BUDGET_READ, &["AgentNotVisible", "RuntimeSessionUnknown", "RuntimeUnavailable", "ProvisioningUnavailable"]] scope("agent", "agent.usage.report", ["id"]);
    GET "/agents/{id}/policy" "An agent's tool-boundary policy" G [SIGNED, &["not_permitted"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/policy" "Set an agent's tool-boundary policy, from its next launch" G [SIGNED_BODY, &["not_permitted", "PolicyVersionConflict"], &["policy_invalid", "policy_rule_duplicate", "policy_target_ambiguous"]] scope("agent", "agent.policy.set", ["id"]);
    GET "/.well-known/openid-configuration" "The issuer's discovery document" P [];
    GET "/oauth/authorize" "Begin an authorization" P [&["RequestMalformed", "RedirectUnregistered", "ScopeUnknown", "ScopeNotGranted", "NoPerson", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"], &["credential_refused", "app_not_approved", "app_retired", "redirect_invalid", "apps_unavailable"]];
    POST "/oauth/token" "Exchange a code or a refresh token for a pass" P [&["RequestMalformed", "ClientUnknown", "CodeExpired", "CodeUnknown", "CodeUsed", "RedirectUnregistered", "VerifierWrong", "RefreshUnknown", "SessionEnded", "HolderRetired", "NotAdmitted", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"], UNANSWERED, &["credential_refused", "app_not_approved", "app_retired", "redirect_invalid", "apps_unavailable", "SecretsUnavailable"], &["grant_log_identity_unavailable", "grant_binding_unsupported", "grant_binding_revision_moved", "grant_binding_degraded"]];
    GET "/oauth/jwks" "The issuer's signing keys" P [&["ProviderUnavailable"]];
    GET "/.well-known/oauth-protected-resource" "Where the MCP door's authorization is found" P [];
    GET "/.well-known/oauth-protected-resource/mcp" "Where the MCP door's authorization is found, by its address" P [];
    GET "/.well-known/oauth-protected-resource/api/mcp" "Where the MCP door's authorization is found, by its screens address" P [];
    GET "/.well-known/oauth-authorization-server" "How a connected app is authorized" P [];
    POST "/oauth/mcp/register" "An app registers to connect to the MCP door" P [&["RequestMalformed", "RegistrationThrottled", "ConfigInvalid"]];
    GET "/oauth/mcp/authorize" "Ask the person to connect an app" P [&["RequestMalformed", "RedirectUnregistered"]];
    POST "/oauth/mcp/consent" "The person connects an app or refuses it" P [&["RequestMalformed", "RedirectUnregistered", "CodeUnknown", "NotSignedIn"]];
    POST "/oauth/mcp/token" "Exchange a connected app's code or refresh token for tokens" P [];
    GET "/oauth/userinfo" "The signed-in subject's claims" B [SIGNED, &["TokenUnknown", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"], &["apps_unavailable"]];
    GET "/apps/me/agents" "The access pass holder's own agents" B [SIGNED, &["TokenUnknown", "PassRefused", "ProductPassPersonRequired", "HolderRetired", "NoPerson", "RequestMalformed", "ProviderUnavailable", "SessionsUnavailable", "DirectoryUnavailable"]];
    POST "/sign-in" "Sign in with a password" P [&["SignInFailed", "NotAdmittedYet", "AccountDisabled", "SecondFactorUnsupported", "SignInRefused", "SignInNotTheirs", "IssuerChallengeExpired", "IssuerRefused", "SignInThrottled", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable"]];
    GET "/sign-in/providers" "The providers the sign-in page offers and the address people open Lys at" P [];
    GET "/sign-in/help" "The configured administrator's sign-in contact" P [&["ConfigInvalid", "DirectoryUnavailable", "SignInProvidersUnavailable", "SignInProvidersRefused", "AccountRefused"]];
    GET "/sign-in/providers/{id}" "Begin sign-in through a provider" P [&["IssuerChallengeExpired"]];
    POST "/setup/open" "Open first-run setup with its code" P [&["SignInFailed", "SignInThrottled", "SetupClosed", "SetupCodeRefused", "DirectoryUnavailable"]];
    POST "/setup/administrator" "Register the first administrator" P [&["SignInFailed", "SignInThrottled", "AccountRefused", "SetupClosed", "SetupCodeRefused", "SignInRefused", "SignInNotTheirs", "IssuerChallengeExpired", "IssuerRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "DirectoryUnavailable"]];
    POST "/setup/password" "Set the administrator's password without changing other sign-in factors" P [&["SignInFailed", "SignInThrottled", "SetupCodeRefused", "SetupUnavailable", "AccountRefused", "SecondFactorUnsupported", "SignInRefused", "SignInNotTheirs", "IssuerChallengeExpired", "IssuerRefused", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "DirectoryUnavailable"]];
    GET "/me/account" "The caller's sign-in account: its email, whether it may sign in, whether it has a password and the providers it signs in with" C [SIGNED, &["AccountRefused"]];
    POST "/me/account/email" "Mail codes to the new and held addresses, confirmed by the current password when held" C [SIGNED_BODY, &["AccountRefused", "EmailChangeNeedsMail", "EmailHeldByAnother", "mail_not_sent"]];
    POST "/me/account/email/confirm" "Confirm both mailbox codes and replace the held address" C [SIGNED_BODY, &["AccountRefused", "EmailChangeCodeRefused", "EmailHeldByAnother"]];
    POST "/me/account/password" "Set or change the caller's password, confirmed by the current one when the account has one" C [SIGNED_BODY, &["AccountRefused"]];
    GET "/directory/people/{id}/account" "A person's sign-in account" G [ADMIN, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "AccountRefused", "IdentityUnknown"]] scope("account", "read", ["id"]);
    POST "/directory/people/{id}/account/email" "Change a person's email" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "AccountRefused", "EmailHeldByAnother", "IdentityUnknown"]] scope("account", "person.email.set", ["id"]);
    POST "/directory/people/{id}/account/enabled" "Enable or disable a person's sign-in" G [ADMIN_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked", "HoldingNotHeld", "TeamsUnavailable", "NoPerson", "AccountRefused", "IdentityUnknown", "SignInProvidersUnavailable", "SignInProvidersRefused", "SessionsUnavailable", "ProviderUnavailable", "WithheldFromAgents"]] scope("account", "person.sign-in.set", ["id"]);
    POST "/directory/people/{id}/account/password" "Set a person's password" S [ADMIN_BODY, &["AccountRefused", "IdentityUnknown", "WithheldFromAgents"]] scope("account", "person.password.set", ["id"]);
    GET "/agents/{id}/goals" "An agent's goals" G [SIGNED, &["AgentNotVisible", "goals_unavailable"]] scope("agent", "read", ["id"]);
    GET "/alarms" "The caller's alarms, open first" G [AGENT, SIGNED, &["NoPerson", "AlarmsUnavailable", "AlarmMalformed"]] scope("agent", "read", []);
    GET "/security-events" "Recovery events visible to the administrator" G [SIGNED, &["NotAdmitted", "SetupUnavailable"]] scope("configuration", "read", []);
    GET "/alarms/{id}" "One of the caller's alarms" G [AGENT, SIGNED, &["NoPerson", "AlarmUnknown", "AlarmsUnavailable", "AlarmMalformed"]] scope("agent", "read", []);
    POST "/alarms/{id}/acknowledge" "A person acknowledges an alarm" GC [SIGNED_BODY, &["NoPerson", "AlarmUnknown", "AlarmsUnavailable", "AlarmMalformed"]] scope("alarm", "alarm.acknowledge", ["id"]);
    POST "/agents/{id}/goals" "Set a goal on an agent" G [SIGNED_BODY, &["AgentNotVisible", "evidence_missing", "goal_reused", "reminder_needs_deadline"]] scope("agent", "agent.goal.set", ["id"]);
    GET "/teams/{id}/goals" "A team's goals" G [SIGNED, &["NotAdmitted", "TeamUnknown"]] scope("team", "read", ["id"]);
    POST "/teams/{id}/goals" "Set a goal on a team" G [SIGNED_BODY, &["NotAdmitted", "TeamUnknown", "evidence_missing", "goal_reused", "reminder_needs_deadline"]] scope("team", "team.goal.set", ["id"]);
    POST "/goals/{goal}/mark" "Judge a goal" G [AGENT, SIGNED_BODY, &["evidence_missing", "goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement"]];
    POST "/goals/{goal}/active" "Suspend or resume a standing aim" G [AGENT, SIGNED_BODY, &["goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement", "goals_unavailable", "TeamsUnavailable", "TeamUnknown"]];
    POST "/goals/{goal}/words" "Reword a standing aim" G [AGENT, SIGNED_BODY, &["goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement", "goals_unavailable", "TeamsUnavailable", "TeamUnknown"]];
    GET "/words" "The words a seat is sent: every layer and template as held" G [SIGNED, &["words_unavailable"]] scope("words", "read", []);
    POST "/words/preview" "Render a slot for a session or an agent; nothing is sent" G [SIGNED_BODY, &["NotAdmitted", "AgentNotVisible", "SessionUnknown", "words_malformed"]] scope("words", "read", []);
    POST "/words/templates/{name}" "Save a named template" G [ADMIN_BODY, &["words_malformed", "words_stale", "words_unavailable"]] scope("words", "words.set", []);
    POST "/words/{slot}" "Set a slot at the workspace layer" G [ADMIN_BODY, &["words_malformed", "words_stale", "words_template_unknown", "words_unavailable"]] scope("words", "words.set", []);
    GET "/agents/{id}/words" "An agent's slots resolved, and its own layer" G [SIGNED, &["AgentNotVisible", "words_unavailable"]] scope("words", "read", ["id"]);
    POST "/agents/{id}/words/{slot}" "Set a slot at an agent's layer" G [SIGNED_BODY, &["AgentNotVisible", "words_malformed", "words_stale", "words_template_unknown", "words_unavailable"]] scope("words", "words.set", ["id"]);
    POST "/runtime/sessions/{id}/words/{slot}" "Set a slot at a session's layer" G [SIGNED_BODY, &["AgentNotVisible", "SessionUnknown", "words_malformed", "words_stale", "words_template_unknown", "words_unavailable"]] scope("words", "words.set", ["id"]);
    GET "/agents/{id}/variables" "An agent's variables" G [AGENT, SIGNED, &["AgentNotVisible", "not_permitted", "variables_unavailable"]];
    POST "/agents/{id}/variables" "Patch an agent's variables with the revision read" G [AGENT, SIGNED_BODY, &["AgentNotVisible", "not_permitted", "variables_malformed", "variables_stale", "variables_unavailable"]];
    GET "/runtime/sessions/{id}/variables" "A session's variables" G [AGENT, SIGNED, &["AgentNotVisible", "SessionUnknown", "not_permitted", "variables_unavailable"]];
    POST "/runtime/sessions/{id}/variables" "Patch a session's variables with the revision read" G [AGENT, SIGNED_BODY, &["AgentNotVisible", "SessionUnknown", "not_permitted", "variables_malformed", "variables_stale", "variables_unavailable"]];
    GET "/me/variables" "The asking run's agent variables, by its run pass" P [SIGNED, &["NotAdmitted", "AgentPassRefused", "variables_unavailable"]];
    POST "/me/variables" "Patch the asking run's agent variables, by its run pass" P [SIGNED_BODY, &["NotAdmitted", "AgentPassRefused", "variables_malformed", "variables_stale", "variables_unavailable"]];
    GET "/me/session/variables" "The asking run's session variables, by its run pass" P [SIGNED, &["NotAdmitted", "AgentPassRefused", "variables_unavailable"]];
    POST "/me/session/variables" "Patch the asking run's session variables, by its run pass" P [SIGNED_BODY, &["NotAdmitted", "AgentPassRefused", "variables_malformed", "variables_stale", "variables_unavailable"]];
    GET "/schedules" "The schedules the caller may see" G [SIGNED, &["schedules_unavailable"]] scope("schedule", "read", []);
    POST "/schedules" "Set a schedule" G [SIGNED_BODY, &["AgentNotVisible", "SessionUnknown", "schedule_malformed", "schedule_reused", "schedules_unavailable"]] scope("schedule", "schedule.set", []);
    GET "/schedules/{id}" "A schedule with every occurrence and delivery" G [SIGNED, &["schedule_unknown", "schedules_unavailable"]] scope("schedule", "read", ["id"]);
    POST "/schedules/{id}/change" "Pause, resume or edit a schedule, keeping its count" G [SIGNED_BODY, &["schedule_malformed", "schedule_reused", "schedule_stopped", "schedule_unknown", "schedules_unavailable"]] scope("schedule", "schedule.set", ["id"]);
    POST "/schedules/{id}/stop" "Stop a schedule" G [SIGNED_BODY, &["schedule_unknown", "schedules_unavailable"]] scope("schedule", "schedule.set", ["id"]);
    GET "/service-accounts" "The service accounts" G [SIGNED] scope("service-account", "read", []);
    POST "/service-accounts" "Create a service account" G [SIGNED_BODY, &["NotAdmitted"], &["ServiceAccountReused"]] scope("service-account", "service-account.create", []);
    POST "/service-accounts/{id}/retire" "Retire a service account" G [SIGNED_BODY, &["ServiceAccountRetired", "ServiceAccountUnknown"]] scope("service-account", "service-account.retire", ["id"]);
    GET "/tree" "The caller's owned and led teams and descendants" G [AGENT, SIGNED, &["TeamsUnavailable", "RolesUnavailable", "ProvisioningUnavailable", "RuntimeUnavailable", "BudgetsUnavailable", "ConfigurationUnavailable", "goals_unavailable"]] scope("team", "read", []);
    GET "/teams" "Every team of the caller's organisation" G [SIGNED, &["NoPerson"]] scope("team", "read", []);
    POST "/teams" "Create a team" G [SIGNED_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["NotAdmitted", "team_parent_cycle", "team_lead_not_member", "TeamsUnavailable", "HoldingNotHeld", "NotHeld", "Revoked", "TeamUnknown", "NoPerson"]] scope("team", "team.create", []);
    GET "/teams/{id}" "One team" G [SIGNED, &["NoPerson", "TeamUnknown"]] scope("team", "read", ["id"]);
    POST "/teams/{id}/members" "Add a team member" G [SIGNED_BODY, AGENT, GRANT_ASKED, UNANSWERED, &["AgentNotVisible", "NotAdmitted", "not_permitted", "HoldingNotHeld", "NotHeld", "Revoked", "TeamUnknown", "TeamsUnavailable", "NoPerson"]] scope("team", "team.member.add", ["id"]);
    POST "/teams/{id}/members/{member}/remove" "Remove a member" G [SIGNED_BODY] scope("team", "team.member.remove", ["id"]);
    POST "/teams/{id}/members/{member}/confirm" "Confirm a held membership" G [ADMIN_BODY, &["TeamsUnavailable"]] scope("team", "team.member.confirm", ["id"]);
    POST "/teams/{id}/nesting" "Replace a team parent and lead" G [SIGNED_BODY, &["NotAdmitted", "team_parent_cycle", "team_lead_not_member", "TeamsUnavailable"]] scope("team", "team.parent.set", ["id"]);
    POST "/teams/{id}/retire" "Retire a team" G [SIGNED_BODY] scope("team", "team.retire", ["id"]);
    GET "/resources" "The resources grants are on" G [SIGNED, &["NotAdmitted"]] scope("resource", "read", []);
    GET "/apps/{app}/schema" "A schema version, the current one unless `version` names one" G [VERSION] scope("app", "read", ["app"]);
    GET "/apps/{app}/placements" "An app's recorded placements: each child resource under its parent, in the order they were placed" G [SIGNED, &["NotAdmitted", "app_unknown"]] scope("resource", "read", []);
    GET "/secrets" "The secrets" G [SIGNED, &["NoPerson", "SecretsUnavailable"]] scope("secret", "read", []);
    GET "/secrets/grants" "The secrets' grants" G [SIGNED] scope("secret", "read", []);
    GET "/secrets/audit" "The secrets' audit" G [SIGNED] scope("secret", "read", []);
    GET "/secrets/revocation" "The secrets' revocations" G [SIGNED, &["RequestMalformed"]] scope("secret", "read", []);
    GET "/secrets/settings" "The secrets' settings" G [SIGNED, &["RequestMalformed"]] scope("secret", "read", []);
    GET "/secrets/handles" "The secret handles" G [SIGNED, &["RequestMalformed"]] scope("secret", "read", []);
    POST "/secrets/scope" "Scope a secret" G [SIGNED_BODY, &["LendingNotPermitted"]] scope("secret", "secret.scope", []);
    POST "/secrets/recipients" "A secret's recipients" G [SIGNED_BODY] scope("secret", "secret.recipients", []);
    POST "/secrets/drop" "Drop a secret handle" G [SIGNED_BODY] scope("secret", "secret.drop", []);
    POST "/secrets/accounts/hold" "Issue a held model account handle" G [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "account_handle_unheld", "account_provider_mismatch", "account_proxy_key_invalid"]] scope("secret", "secret.account.hold", []);
    POST "/secrets/add" "Add a secret with its value" G [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "SecretExists", "SecretRetired", "OperationReused", "RouteInvalid", "ValueEmpty"]] scope("secret", "secret.add", []);
    POST "/secrets/replace" "Change a secret's value" G [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "LendingNotPermitted", "ValueEmpty"]] scope("secret", "secret.replace", []);
    POST "/secrets/retire" "Retire a secret" G [SIGNED_BODY, &["NoPerson", "SecretsUnavailable", "LendingNotPermitted"]] scope("secret", "secret.retire", []);
    GET "/canvas" "The caller's canvas: their arrangement and saved layouts" S [SIGNED, &["NoPerson", "CanvasUnavailable"]];
    PUT "/canvas" "Keep the caller's arrangement" S [SIGNED_BODY, &["NoPerson", "CanvasRefused", "CanvasUnavailable"]];
    POST "/canvas/layouts" "Save an arrangement as a layout of the caller's, by name" S [SIGNED_BODY, &["NoPerson", "CanvasRefused", "CanvasUnavailable"]];
    POST "/canvas/layouts/remove" "Remove a layout of the caller's, by name" S [SIGNED_BODY, &["NoPerson", "CanvasRefused", "CanvasUnavailable"]];
    GET "/sessions" "The caller's sessions" S [SIGNED, &["NoPerson"]];
    POST "/sessions/{id}/end" "End one's own session" S [SIGNED, &["SessionUnknown"], &["NoPerson"]];
    GET "/directory/people/{id}/sessions" "A person's sessions" G [ADMIN, &["IdentityUnknown"]] scope("person", "read", ["id"]);
    POST "/directory/people/{id}/sessions/{session}/end" "End a session" G [ADMIN, &["SessionUnknown"], &["IdentityUnknown"]] scope("session", "person.session.end", ["session"]);
    GET "/configuration" "The service's configuration" G [ADMIN, &["ConfigurationUnavailable"]] scope("configuration", "read", []);
    PUT "/configuration" "Set the organisation zone at its current version" G [ADMIN_BODY, &["ConfigurationMalformed", "ConfigurationVersionConflict", "ConfigurationZoneRefused", "ConfigurationUnavailable", "NoPerson"]] scope("configuration", "configuration.zone.set", []);
    GET "/agents/{id}/memory" "An agent's memory" G [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    GET "/agents/{id}/certificates" "An agent's certificates" G [SIGNED, &["AgentNotVisible"]] scope("agent", "read", ["id"]);
    POST "/agents/{id}/certificates" "Issue an own agent's certificate over a request given as PEM or standard base64 DER, with a person's app pass or sign-in, administration requiring sign-in; or, by an agent holding agent.certificate.issue on the agent or on the person it answers to, any certificate; or, by the agent that registered it, its first certificate alone" G [AGENT, &["agent_not_yours", "agent_create_revoked", "certificate_not_first"], APP_PERSON, SIGNED_BODY, &["NotAdmitted", "AgentNotVisible", "CertificateReused"], &["CertificateReused"]] scope("agent", "agent.certificate.issue", ["id"]);
    POST "/agents/{id}/certificates/{serial}/withdrawal" "Withdraw one; never an agent's" G [AGENT, &["not_permitted"], ADMIN_BODY, &["CertificateUnknown", "CertificateWithdrawn", "CertificatesUnavailable"], &["CertificateWithdrawn"]] scope("agent", "agent.certificate.withdraw", ["id"]);
    POST "/agents/{id}/start" "Start an agent" G [AGENT, &["not_permitted"], SIGNED_BODY, START_BUDGET, &["AgentHasNoPolicy", "PolicyUnavailable", "everything_stopped", "LaunchUnrenderable", "account_handle_unheld", "account_provider_mismatch", "SecretsUnavailable"]] scope("agent", "agent.start", ["id"]);
    POST "/launch-records/{id}/start-again" "Start a launch again" G [SIGNED_BODY, &["AgentHasNoPolicy", "PolicyUnavailable", "AgentNotActive", "AgentNotVisible", "CertificatesUnavailable", "everything_stopped", "account_handle_unheld", "account_provider_mismatch", "SecretsUnavailable"]] scope("launch-record", "launch-record.start-again", ["id"]);
    POST "/launch-records/{id}/withdraw" "Withdraw a launch" G [SIGNED_BODY] scope("launch-record", "launch-record.withdraw", ["id"]);
    GET "/launch-records/{id}/state" "A launch's state" G [SIGNED] scope("launch-record", "read", ["id"]);
    GET "/changes" "Wait for the next change signal" G [SIGNED, &["RequestMalformed", "RuntimeUnavailable"]] scope("change", "read", []);
    GET "/mcp" "MCP server stream availability" G [SIGNED];
    POST "/mcp" "MCP calls through the admitted HTTP router" G [SIGNED, AGENT, &["GrantTokenCookieConflict", "TokenScopeUndeclared", "TokenHolderNotAgent", "TokenUnknown", "seat_unverifiable", "pass_unknown_or_ended"]];
    GET "/surface-contract" "The surface registration and computer admission contract" P [];
    GET "/openapi.json" "This document" P [];
    POST "/seats" "Add a seat: a name an agent runs under on one machine from one reviewed profile version" G [AGENT, &["not_permitted"], SIGNED_BODY, &["seat_name_invalid", "seat_name_taken", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed", "NotAdmitted", "AgentNotVisible", "ProfileVersionUnknown", "ProfileNotReviewed", "HarnessUndeclared", "MachineUnknown", "MachineRetired", "MachineWithoutRunner", "ProvisioningUnavailable", "NetworkUnavailable"]];
    POST "/seats/{name}/version" "Select a reviewed profile version for a stopped seat" G [AGENT, &["not_permitted"], SIGNED_BODY, &["seat_unknown", "seat_name_invalid", "seat_running", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed", "NotAdmitted", "AgentNotVisible", "ProfileVersionUnknown", "ProfileNotReviewed", "HarnessUndeclared", "ProvisioningUnavailable"]];
    POST "/seats/{name}/retire" "Retire a stopped seat and release its name without discarding its history" G [SIGNED_BODY, &["seat_unknown", "seat_name_invalid", "seat_running", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed", "NotAdmitted", "AgentNotVisible"]] scope("seat", "seat.retire", ["name"]);
    GET "/seats" "Every seat with its state from its runner's own knowledge, and whether the runners were read" G [SIGNED, &["seats_unavailable", "IdentifierMalformed"]] scope("seat", "read", []);
    GET "/seats/unregistered" "The sessions runners hold for no seat" G [SIGNED, &["seats_unavailable", "NetworkUnavailable", "RuntimeUnavailable"]] scope("seat", "read", []);
    GET "/seats/owned" "Every seat owner each runner started, live and unreachable as the runner proved them at the kernel" G [SIGNED, &["seats_unavailable", "authority_unavailable"]] scope("seat", "read", []);
    GET "/seats/{name}" "One seat with its state" G [SIGNED, &["seat_unknown", "seats_unavailable", "IdentifierMalformed"]] scope("seat", "read", ["name"]);
    POST "/seats/{name}/start" "Start a seat through its reviewed profile's selected transport" G [AGENT, SIGNED_BODY, GRANT_ASKED, UNANSWERED, START_RUNNER, &["runner_absent", "runner_unreachable", "runner_reply_malformed", "runner_protocol_mismatch", "runner_request_unsigned", "runner_request_replayed", "RuntimeSessionUnknown", "RuntimeUnavailable"], &["not_permitted", "seat_unknown", "seat_no_responsible", "seat_running", "import_incomplete", "ConfigInvalid", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed", "ProfileVersionUnknown", "ProfileNotReviewed", "everything_stopped", "seat_owner_start_failed", "seat_owner_held", "seat_owner_binding_invalid", "control_binding_invalid", "seat_owner_binding_mismatch"]] scope("seat", "seat.start", ["name"]);
    POST "/seats/{name}/stop" "Stop a seat between turns, or mid-turn with force" G [AGENT, SIGNED_BODY, GRANT_ASKED, UNANSWERED, &["runner_absent", "runner_unreachable", "runner_reply_malformed", "runner_protocol_mismatch", "runner_request_unsigned", "runner_request_replayed", "RuntimeSessionUnknown", "RuntimeUnavailable"], &["not_permitted", "seat_unknown", "seat_not_running", "seat_turn_in_progress", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed"]] scope("seat", "seat.stop", ["name"]);
    POST "/seats/{name}/restart" "Stop a seat and restart its reviewed profile's selected transport under a new session" G [AGENT, SIGNED_BODY, GRANT_ASKED, UNANSWERED, START_RUNNER, &["runner_absent", "runner_unreachable", "runner_reply_malformed", "runner_protocol_mismatch", "runner_request_unsigned", "runner_request_replayed", "RuntimeSessionUnknown", "RuntimeUnavailable"], &["not_permitted", "seat_unknown", "seat_no_responsible", "seat_running", "import_incomplete", "ConfigInvalid", "seat_turn_in_progress", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed", "ProfileVersionUnknown", "ProfileNotReviewed", "everything_stopped", "seat_owner_start_failed", "seat_owner_held"]] scope("seat", "seat.restart", ["name"]);
    POST "/seats/{name}/send" "Send a seat a message, delivered as a user turn" G [SIGNED_BODY, GRANT_ASKED, UNANSWERED, &["runner_absent", "runner_unreachable", "runner_reply_malformed", "runner_protocol_mismatch", "runner_request_unsigned", "runner_request_replayed", "RuntimeSessionUnknown", "RuntimeUnavailable"], &["not_permitted", "seat_unknown", "seat_not_running", "seat_text_empty", "seat_text_control", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed"]] scope("seat", "seat.send", ["name"]);
    POST "/seats/{name}/type" "A line typed in an attach, delivered as a user turn" G [SIGNED_BODY, GRANT_ASKED, UNANSWERED, &["runner_absent", "runner_unreachable", "runner_reply_malformed", "runner_protocol_mismatch", "runner_request_unsigned", "runner_request_replayed", "RuntimeSessionUnknown", "RuntimeUnavailable"], &["not_permitted", "seat_unknown", "seat_not_running", "seat_text_empty", "seat_text_control", "seat_operation_reused", "seats_unavailable", "IdentifierMalformed"]] scope("seat", "seat.send", ["name"]);
    POST "/seats/{name}/attach" "Read a seat's live session as lines, following it" G [SIGNED_BODY, &["runner_absent", "runner_unreachable", "runner_reply_malformed", "runner_protocol_mismatch", "runner_request_unsigned", "runner_request_replayed", "RuntimeSessionUnknown", "RuntimeUnavailable"], &["seat_unknown", "seat_not_running", "seat_attach_refused", "attach_pty_use_read_bytes", "seats_unavailable", "IdentifierMalformed", "AgentNotVisible"]] scope("seat", "read", ["name"]);
    GET "/seats/{name}/import" "A seat's imports, its selected import and its latest dry run" G [SIGNED, &["seat_unknown", "seat_name_invalid", "seats_unavailable", "import_bulk_refused", "seat_imports_unavailable"]] scope("seat", "read", ["name"]);
    POST "/seats/{name}/import/dry-run" "Read one seat's declared sources at one instant and show the whole import plan; nothing is sent, started or changed" G [SIGNED_BODY, &["seat_unknown", "seat_name_invalid", "seats_unavailable", "import_bulk_refused", "import_seat_mismatch", "import_confirmer_not_person", "import_no_responsible", "not_permitted", "AgentNotVisible", "IdentifierMalformed", "seat_imports_unavailable"]] scope("seat", "read", ["name"]);
    POST "/seats/{name}/import/confirm" "Confirm one seat's exact dry-run plan and apply it, resumably, under one operation" G [SIGNED_BODY, GRANT_ASKED, UNANSWERED, &["not_permitted", "seat_unknown", "seat_name_invalid", "seats_unavailable", "IdentifierMalformed", "AgentNotVisible", "import_bulk_refused", "import_confirmer_not_person", "import_no_responsible", "import_plan_unknown", "import_plan_stale", "import_plan_refused", "import_source_changed", "import_destination_moved", "import_seat_running", "import_operation_reused", "import_in_progress", "import_stopped", "import_destination_unconfigured", "import_destination_unsupported", "import_destination_malformed", "import_schedule_active", "import_upgrade_pending", "import_upgrade_intent_unreadable", "seat_imports_unavailable"], &["words_unavailable", "variables_unavailable", "schedules_unavailable", "BudgetsUnavailable", "ProvisioningUnavailable"]] scope("seat", "seat.import", ["name"]);
    POST "/agents/{id}/pass" "An agent's pass to an approved app, carrying its own grants, asked with its run pass or a grant credential for a grant it holds" P [&["agent_pass_unproven", "agent_retired", "audience_not_approved", "agent_not_holder", "AgentPassRefused", "CertificatesUnavailable", "RequestMalformed", "IdentifierMalformed", "HolderRetired", "ProviderUnavailable", "authority_unavailable", "apps_unavailable", "DirectoryUnavailable"], UNANSWERED];
    POST "/agents/{id}/pass/revoke" "Revoke an agent's retained audience passes with its own run pass and seat proof; offline copies remain valid until expiry" P [&["agent_pass_unproven", "agent_retired", "audience_not_approved", "agent_not_holder", "AgentPassRefused", "CertificatesUnavailable", "RequestMalformed", "IdentifierMalformed", "HolderRetired", "ProviderUnavailable", "authority_unavailable", "apps_unavailable", "DirectoryUnavailable"], UNANSWERED];
};
