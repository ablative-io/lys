//! The route table's untyped entries and the authentication every entry of
//! the table is written with; the refusal sets are `openapi_refusals.rs`'s.
//! `openapi.rs` turns them into the document; nothing here is read by
//! anything else.

use lys_openapi::{Auth, Method};

use crate::openapi_refusals::{
    ADMIN, ADMIN_BODY, AGENT, GRANT_ASKED, GRANT_MADE, GRANT_READ, PERSON, RECORDED, SIGNED,
    SIGNED_BODY, UNANSWERED,
};

pub(crate) const GET: Method = Method::Get;
pub(crate) const POST: Method = Method::Post;
pub(crate) const PUT: Method = Method::Put;

/// Anyone.
pub(crate) const P: &[Auth] = &[Auth::Public];
/// A signed-in person.
pub(crate) const S: &[Auth] = &[Auth::Session];
/// A signed-in person, or an app or registrar through its credential.
pub(crate) const A: &[Auth] = &[Auth::Session, Auth::Bearer];
/// An agent's signed request, or a signed-in person.
pub(crate) const G: &[Auth] = &[Auth::AgentSignature, Auth::Session];

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
    GET "/login" "Begin a sign-in at the issuer" P [];
    GET "/callback" "Finish a sign-in and begin a session" P [&["SignInStateUnknown"]];
    POST "/setup" "Finish the administrator's first-run setup" S [SIGNED_BODY, &["AlreadyBootstrapped", "NotAdmitted", "OperationReused", "ProfileInvalid"]];
    POST "/people" "Register a person" S [ADMIN_BODY, &["OperationReused"]];
    POST "/agents" "Register an agent" A [ADMIN_BODY, &["credential_refused", "ServiceAccountUnknown", "NotHeld"]];
    GET "/identities" "Every identity the directory holds" S [ADMIN];
    GET "/identities/{id}" "One identity" S [ADMIN];
    POST "/identities/{id}/profile" "Change an identity's profile" S [ADMIN_BODY];
    POST "/identities/{id}/transitions" "Move an identity's state" S [ADMIN_BODY];
    POST "/people/{id}/logins" "Bind a login to a person" S [ADMIN_BODY];
    GET "/me" "The signed-in caller" S [SIGNED, &["NoPerson", "SetupRequired"]];
    GET "/people" "The people the caller may see" S [SIGNED, &["NoPerson"]];
    GET "/agents/{id}" "An agent the caller answers for" S [PERSON, &["AgentNotVisible"]];
    GET "/directory/people" "Every person, for the administrator" S [ADMIN];
    GET "/directory/agents/{id}" "Any agent, for the administrator" S [ADMIN, &["AgentNotVisible"]];
    GET "/grants" "The grants the caller may see" S [SIGNED, &["NotAdmitted"]];
    POST "/grants" "Pass on part of a grant" S [GRANT_MADE, RECORDED, &["NoPerson", "ExpiryBeyondSource"], &["UseOnly"]];
    GET "/grants/model" "Lys's own permission model" S [SIGNED];
    POST "/grants/roots" "Issue a root grant" S [GRANT_MADE, RECORDED, &["RelationUnknown"], &["RootAuthorityRefused"]];
    POST "/grants/check" "Check, and record, an exercise" S [GRANT_ASKED, UNANSWERED, &["NotHeld", "Revoked"]];
    POST "/grants/why" "Why the caller may act" S [GRANT_ASKED, UNANSWERED, &["NotHeld"]];
    POST "/grants/who" "Who may act on a resource" S [GRANT_ASKED, UNANSWERED];
    POST "/grants/reach" "Who may act on each of many resources" S [GRANT_ASKED, UNANSWERED];
    GET "/grants/cannot-give" "What the caller cannot pass on" S [GRANT_READ, &["IdentityUnknown"]];
    GET "/grants/{id}" "One grant the caller may see" S [GRANT_READ, &["GrantIdMalformed"]];
    POST "/grants/{id}/revoke" "Revoke a grant and all it derives" S [GRANT_READ, RECORDED, &["GrantUnknown", "RevokeRefused"]];
    GET "/receipts/{index}" "A directory receipt, publicly" P [&["RequestMalformed"]];
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
    GET "/requests" "The access requests" S [SIGNED, &["NoPerson", "NotAdmitted"]];
    POST "/requests" "Ask for access" S [SIGNED_BODY, &["RelationUnknown"], &["NoPerson", "NotAdmitted", "RequestReused"]];
    POST "/requests/{id}/approve" "Approve an access request" S [SIGNED_BODY, &["NotAdmitted", "RequestDecided"], &["SourceUnknown"]];
    POST "/requests/{id}/decline" "Decline an access request" S [SIGNED_BODY, &["RequestDecided", "RequestUnknown"], &["NotAdmitted"]];
    POST "/requests/{id}/reconcile" "Settle an approval" S [SIGNED_BODY, &["NoPerson", "NotAdmitted"]];
    GET "/connections" "What the service is connected to" S [ADMIN];
    GET "/sign-in-providers" "The sign-in providers" S [ADMIN, &["SignInProvidersUnavailable"]];
    POST "/sign-in-providers" "Set a sign-in provider" S [ADMIN_BODY];
    POST "/link-audit" "Deliver a link-audit record" G [AGENT, &["NotAdmitted", "NotSignedIn", "RequestMalformed"]];
    POST "/link-audit/person" "Look up a link-audit holder" G [AGENT, &[ "LoginUnbound", "NotAdmitted", "NotSignedIn", "RequestMalformed", ]];
    GET "/network" "The machines" S [SIGNED];
    POST "/network/machines" "Name a machine" S [ADMIN_BODY, &["MachineReused"], &["IdentifierMalformed"]];
    POST "/network/machines/{id}/retire" "Retire a machine" S [ADMIN_BODY, &["MachineUnknown"]];
    GET "/agents/{id}/provisioning" "An agent's profile" S [SIGNED, &["AgentNotVisible"]];
    POST "/agents/{id}/provisioning" "Set an agent's profile" S [ADMIN_BODY, &["ProvisioningChanged"], &["AgentNotVisible", "ProvisioningReused"], &["McpCredentialInline", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown"]];
    GET "/skills" "The skills Lys keeps" S [SIGNED];
    POST "/skills" "Keep a skill's text" S [ADMIN_BODY];
    POST "/agents/{id}/provisioning/{version}/review" "Review a profile" S [ADMIN_BODY];
    POST "/agents/{id}/start-command" "An agent's start command" S [SIGNED_BODY, &["AgentNotVisible", "MachineCannotReach"], &["LaunchRecordMissing", "MachineNotForAgent", "MachineRetired", "MachineUnknown", "MachineWithoutRuntime", "NotAdmitted"], &["HarnessUndeclared", "LaunchUnrenderable", "McpHandleUnsupported", "McpSettingUnrepresentable", "ModelUnrepresentable", "PolicyUnrepresentable", "SkillUnknown"], &["runner_protocol_mismatch"]];
    POST "/agents/{id}/runtime/sessions/{session}/reports" "A runtime report" G [AGENT, &["NotAdmitted"], &["AgentNotVisible", "RequestMalformed"]];
    GET "/agents/{id}/runtime/sessions" "An agent's runtime sessions" S [SIGNED, &["AgentNotVisible"]];
    GET "/runtime/sessions" "Every runtime session" S [ADMIN];
    GET "/runtime/message-edges" "Caller-visible Cambium message addresses mapped to Lys identities" S [PERSON, &["MessageEdgesUnavailable"]];
    POST "/runtime/found/{session}/reports" "Report a found session" S [ADMIN_BODY, &["MachineUnknown"], &["RuntimeSessionStopped", "RuntimeSessionUnknown"]];
    GET "/runtime/found" "The sessions found" S [ADMIN];
    POST "/agents/{id}/stop" "Stop an agent" S [SIGNED_BODY, &["AgentNotVisible", "StopReused"]];
    GET "/agents/{id}/stops" "An agent's stops" S [SIGNED, &["AgentNotVisible"]];
    GET "/budgets/{kind}/{id}" "A holder's budgets" S [SIGNED, &["not_permitted"]];
    PUT "/budgets/{kind}/{id}" "Set a holder's budget" S [SIGNED_BODY, &["not_permitted", "zone_missing", "zone_unknown", "period_missing", "budget_invalid"], &["BudgetVersionConflict"]];
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
    GET "/runtime/live" "The sessions still running" S [SIGNED];
    GET "/network/machines/{id}/runner" "A machine's runner" S [SIGNED, &["MachineUnknown"]];
    POST "/network/machines/{id}/runner" "Name a machine's runner" S [ADMIN_BODY, &["MachineUnknown"]];
    GET "/runner/protocol" "The runner protocol" P [];
    POST "/runner/dial/{machine}/next" "A dialled runner's next request" P [&["runner_dial_refused", "runner_dial_stale"]];
    POST "/runner/dial/{machine}/replies/{ticket}" "A dialled runner's reply" P [&["runner_dial_refused", "runner_dial_stale"]];
    GET "/runner-receipts/{index}" "A runner act's receipt" P [&["RequestMalformed"]];
    GET "/agents/{id}/usage" "An agent's budget crossings and what came of each" S [SIGNED, &["AgentNotVisible", "not_permitted"]];
    POST "/agents/{id}/usage" "Report a use an agent made" S [SIGNED_BODY, &["AgentNotVisible", "not_permitted"]];
    GET "/agents/{id}/policy" "An agent's tool-boundary policy" S [SIGNED, &["not_permitted"]];
    POST "/agents/{id}/policy" "Set an agent's tool-boundary policy, from its next launch" S [SIGNED_BODY, &["not_permitted", "PolicyVersionConflict"], &["policy_invalid", "policy_rule_duplicate", "policy_target_ambiguous"]];
    GET "/.well-known/openid-configuration" "The issuer's discovery document" P [];
    GET "/oauth/authorize" "Begin an authorization" P [];
    POST "/oauth/token" "Exchange a code for tokens" P [];
    GET "/oauth/jwks" "The issuer's signing keys" P [];
    GET "/oauth/userinfo" "The signed-in subject's claims" A [SIGNED];
    POST "/sign-in" "Sign in with a password" P [];
    GET "/sign-in/providers" "The providers the sign-in page offers" P [];
    GET "/sign-in/providers/{id}" "Begin sign-in through a provider" P [];
    POST "/setup/open" "Open first-run setup with its code" P [];
    POST "/setup/administrator" "Register the first administrator" P [];
    POST "/setup/password" "Set the first administrator's password" P [];
    GET "/me/account" "The caller's sign-in account" S [SIGNED];
    POST "/me/account/email" "Change the caller's email" S [SIGNED_BODY];
    POST "/me/account/password" "Change the caller's password" S [SIGNED_BODY];
    GET "/directory/people/{id}/account" "A person's sign-in account" S [ADMIN];
    POST "/directory/people/{id}/account/email" "Change a person's email" S [ADMIN_BODY];
    POST "/directory/people/{id}/account/enabled" "Enable or disable a person's sign-in" S [ADMIN_BODY];
    POST "/directory/people/{id}/account/password" "Set a person's password" S [ADMIN_BODY];
    GET "/agents/{id}/goals" "An agent's goals" S [SIGNED, &["AgentNotVisible", "goals_unavailable"]];
    POST "/agents/{id}/goals" "Set a goal on an agent" S [SIGNED_BODY, &["AgentNotVisible", "evidence_missing", "goal_reused"]];
    GET "/teams/{id}/goals" "A team's goals" S [SIGNED, &["NotAdmitted", "TeamUnknown"]];
    POST "/teams/{id}/goals" "Set a goal on a team" S [SIGNED_BODY, &["NotAdmitted", "TeamUnknown"]];
    POST "/goals/{goal}/mark" "Judge a goal" G [AGENT, SIGNED_BODY, &["evidence_missing", "goal_closed", "goal_reused", "goal_unknown", "not_permitted", "not_your_judgement"]];
    GET "/service-accounts" "The service accounts" S [SIGNED];
    POST "/service-accounts" "Create a service account" S [SIGNED_BODY, &["NotAdmitted"], &["ServiceAccountReused"]];
    POST "/service-accounts/{id}/retire" "Retire a service account" S [SIGNED_BODY, &["ServiceAccountRetired", "ServiceAccountUnknown"]];
    GET "/teams" "Every team" S [SIGNED];
    POST "/teams" "Create a team" S [SIGNED_BODY];
    GET "/teams/{id}" "One team" S [SIGNED];
    POST "/teams/{id}/members" "Add a team member" S [SIGNED_BODY];
    POST "/teams/{id}/members/{member}/remove" "Remove a member" S [SIGNED_BODY];
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
    GET "/configuration" "The service's configuration" S [ADMIN];
    GET "/agents/{id}/memory" "An agent's memory" S [SIGNED, &["AgentNotVisible"]];
    GET "/agents/{id}/certificates" "An agent's certificates" S [SIGNED, &["AgentNotVisible"]];
    POST "/agents/{id}/certificates" "Issue a certificate" S [ADMIN_BODY, &["AgentNotVisible", "CertificateReused"], &["CertificateReused"]];
    POST "/agents/{id}/certificates/{serial}/withdrawal" "Withdraw one" S [ADMIN_BODY, &["CertificateUnknown", "CertificateWithdrawn"], &["CertificateWithdrawn"]];
    POST "/agents/{id}/start" "Start an agent" S [SIGNED_BODY];
    POST "/launch-records/{id}/start-again" "Start a launch again" S [SIGNED_BODY];
    POST "/launch-records/{id}/withdraw" "Withdraw a launch" S [SIGNED_BODY];
    GET "/launch-records/{id}/state" "A launch's state" S [SIGNED];
    GET "/openapi.json" "This document" P [];
};
