//! Named service refusals preserve the public enum and its derived conversions.
mod directory;
directory::directory_refusals! {
    /// In-flight work records could not be read or changed.
    #[error("waits_unavailable: {reason}")]
    WaitsUnavailable {
        /// What failed.
        reason: String,
    },
    /// The call already answered or the identity was never held.
    #[error("wait_unknown: no outbound call is waiting by that id")]
    WaitUnknown,
    /// A product's own-agent view requires a person's access pass.
    #[error("ProductPassPersonRequired: this route requires a person's access pass")]
    ProductPassPersonRequired,
    /// The roles cannot be read or written.
    #[error("RolesUnavailable: {reason}")]
    RolesUnavailable {
        /// What failed.
        reason: String,
    },
    /// The membership capability's own state cannot be read.
    #[error("MembershipUnavailable: {reason}")]
    MembershipUnavailable {
        /// What failed.
        reason: String,
    },
    /// No role is kept by that id.
    #[error("RoleUnknown: no role is kept by that id")]
    RoleUnknown,
    /// The role has no version by that number.
    #[error("RoleVersionUnknown: the role has no version by that number")]
    RoleVersionUnknown,
    /// The identity holds no holding of the role, or the directory does not know it.
    #[error(
        "HolderUnknown: no such holder: the directory does not know the identity, or it has no holding of the role"
    )]
    HolderUnknown,
    /// The operation id already names a role, a version or a holding made in other words.
    #[error(
        "RoleReused: operation `{operation}` already names a role, a version or a holding made in other words"
    )]
    RoleReused {
        /// The operation id.
        operation: String,
    },
    /// The holder already holds the role.
    #[error("RoleHeld: `{holder}` already holds the role: end that holding or let it lapse first")]
    RoleHeld {
        /// The holder.
        holder: String,
    },
    /// The holding is over, so it is not moved.
    #[error(
        "HoldingOver: the holding is {state} and is never renewed or moved: assign the role again"
    )]
    HoldingOver {
        /// `lapsed` or `ended`.
        state: &'static str,
    },
    /// The holding is not the one the change was asked of, or not where it
    /// was when the change was asked.
    #[error(
        "HoldingChanged: the holding is not as it was when this change was asked: read the role again and ask the change of the holding as it stands"
    )]
    HoldingChanged,
    /// The agent has no provisioning profile, so there is nothing to start it from.
    #[error(
        "LaunchRecordMissing: the agent has no provisioning profile to start it from: set its profile first"
    )]
    LaunchRecordMissing,
    /// A request carrying the operator token was refused, naming the check.
    #[error("OperatorRefused: {reason}")]
    OperatorRefused {
        /// Which check refused it.
        reason: &'static str,
    },
    /// An agent's signed request was refused, naming the check that refused it.
    #[error("AgentSignatureRefused: {reason}")]
    AgentSignatureRefused {
        /// Which check refused it.
        reason: &'static str,
    },
    /// An unknown, ended or mixed run pass was refused.
    #[error("AgentPassRefused: {reason}")]
    AgentPassRefused {
        /// The check that refused it, without credential material.
        reason: String,
    },
    /// The agent is not active, so it is not started.
    #[error("AgentNotActive: the agent is {state} and only an active agent is started")]
    AgentNotActive {
        /// The agent's lifecycle state.
        state: String,
    },
    /// The machine may not reach a host the agent's profile needs.
    #[error("MachineCannotReach: the machine may not reach `{host}`, which the profile needs")]
    MachineCannotReach {
        /// The host.
        host: String,
    },
    /// The machine is retired, so nothing is started on it.
    #[error("MachineRetired: the machine is retired and nothing is started on it")]
    MachineRetired,
    /// The machine does not list the agent among those that may run on it.
    #[error(
        "MachineNotForAgent: the machine does not list this agent among those that may run on it"
    )]
    MachineNotForAgent,
    /// The machine has no runtime, so nothing is started on it.
    #[error("MachineWithoutRuntime: the machine has no runtime to start the agent with")]
    MachineWithoutRuntime,
    /// The launch names no working folder and the agent's profile has no
    /// default of its own, so there is nowhere to start the harness: the
    /// runner's home is never used in its place.
    #[error(
        "WorkingFolderUnnamed: the launch names no working folder and the agent's profile has no working_folder; name one on the launch or set the agent's default"
    )]
    WorkingFolderUnnamed,
    /// The machine cannot execute a start without a recorded runner.
    #[error("MachineWithoutRunner: the machine has no recorded runner to start the agent with")]
    MachineWithoutRunner,
    /// A skill Lys keeps no text for.
    #[error("SkillUnknown: Lys keeps no skill named `{name}`; keep its text under /skills first")]
    SkillUnknown {
        /// The skill.
        name: String,
    },
    /// A permission rule, mode or directory the settings file cannot express.
    #[error("PolicyUnrepresentable: the settings file cannot express `{rule}`: {reason}")]
    PolicyUnrepresentable {
        /// The rule, mode or directory, or the policy rule's id.
        rule: String,
        /// Why it cannot.
        reason: String,
    },
    /// A model the declared harness cannot carry.
    #[error("ModelUnrepresentable: the harness `{harness}` cannot carry model `{model}`: {reason}")]
    ModelUnrepresentable {
        /// The declared harness name.
        harness: String,
        /// The model.
        model: String,
        /// Why it cannot.
        reason: String,
    },
    /// The profile declares no harness build to start.
    #[error(
        "HarnessUndeclared: profile version {version} declares no harness build to start; declare the build's name, description, program and package in the profile"
    )]
    HarnessUndeclared {
        /// The profile version.
        version: u32,
    },
    /// The launch template the profile renders to is not one the home takes.
    #[error("LaunchUnrenderable: {reason}")]
    LaunchUnrenderable {
        /// What the home refused.
        reason: String,
    },
    /// An MCP server's program, argument, setting or address carries a
    /// credential where only a handle may; the value is never repeated.
    #[error(
        "McpCredentialInline: MCP server `{server}` carries a credential in {member}; give the secret as a handle"
    )]
    McpCredentialInline {
        /// The server.
        server: String,
        /// Where in it.
        member: String,
    },
    /// An MCP server's setting is not one the launch can carry.
    #[error("McpSettingUnrepresentable: MCP server `{server}` {member}: {reason}")]
    McpSettingUnrepresentable {
        /// The server.
        server: String,
        /// Which setting.
        member: String,
        /// Why it cannot be carried.
        reason: String,
    },
    /// An MCP server's setting names a secret the agent holds no handle on,
    /// so nothing could resolve it at start.
    #[error(
        "McpHandleUnsupported: MCP server `{server}` {member} names secret `{secret}`, on which the agent holds no handle for the secrets broker to resolve"
    )]
    McpHandleUnsupported {
        /// The server.
        server: String,
        /// Which setting.
        member: String,
        /// The secret it names.
        secret: String,
    },
    /// The runtime reports are not configured, or their log could not be read or written.
    #[error("RuntimeUnavailable: {reason}")]
    RuntimeUnavailable {
        /// What failed.
        reason: String,
    },
    /// No session by that id is visible to the caller, or the report does not begin one.
    #[error(
        "RuntimeSessionUnknown: no session by that id is visible to the caller; an agent's session begins with a starting report and a found session with a running report"
    )]
    RuntimeSessionUnknown,
    /// The session was already started.
    #[error("RuntimeSessionStarted: session `{session}` was already started")]
    RuntimeSessionStarted {
        /// The session.
        session: String,
    },
    /// The runtime confirmed the session stopped, so it takes no further report.
    #[error(
        "RuntimeSessionStopped: session `{session}` was confirmed stopped and takes no further report"
    )]
    RuntimeSessionStopped {
        /// The session.
        session: String,
    },
    /// The operation id already names a report sent in other words.
    #[error(
        "RuntimeReportReused: operation `{operation}` already names a report sent in other words"
    )]
    RuntimeReportReused {
        /// The operation id.
        operation: String,
    },
    /// The service accounts are not configured, or their log could not be read or written.
    #[error("ServiceAccountsUnavailable: {reason}")]
    ServiceAccountsUnavailable {
        /// What failed.
        reason: String,
    },
    /// No service account by that id is visible to the caller: none is kept, or it is another's.
    #[error(
        "ServiceAccountUnknown: no service account by that id is visible to the signed-in caller"
    )]
    ServiceAccountUnknown,
    /// The operation id already names a service account act in other words.
    #[error(
        "ServiceAccountReused: operation `{operation}` already names a service account act in other words: send this act under a new operation id"
    )]
    ServiceAccountReused {
        /// The operation id.
        operation: String,
    },
    /// The service account was already retired, under another operation.
    #[error(
        "ServiceAccountRetired: service account `{account}` was already retired under another operation"
    )]
    ServiceAccountRetired {
        /// The service account.
        account: String,
    },
    /// The person named as owner is retired, so no service account is created for them.
    #[error(
        "ServiceAccountOwnerRetired: person `{owner}` is retired and owns no new service account"
    )]
    ServiceAccountOwnerRetired {
        /// The person named.
        owner: String,
    },
    /// The agent policies cannot be kept or read.
    #[error("PolicyUnavailable: {reason}")]
    PolicyUnavailable {
        /// Why.
        reason: String,
    },
    /// A launch requires an explicitly recorded policy.
    #[error("AgentHasNoPolicy: agent {agent} has no policy; record its policy before starting it")]
    AgentHasNoPolicy {
        /// The agent whose launch was refused.
        agent: String,
    },
    /// Another change to the policy came between the caller's read and its
    /// change.
    #[error(
        "PolicyVersionConflict: the policy is at version {held}, not {expected}: read it again and send the change on the version read"
    )]
    PolicyVersionConflict {
        /// The version held.
        held: u64,
        /// The version the caller read.
        expected: u64,
    },
    /// A policy of the wrong shape, refused by name.
    #[error("{refusal}: {words}")]
    PolicyRefused {
        /// The refusal's name.
        refusal: &'static str,
        /// Why, in words.
        words: String,
    },
    /// The emergency stops cannot be kept or read.
    #[error("StopsUnavailable: {reason}")]
    StopsUnavailable {
        /// Why.
        reason: String,
    },
    /// The operation id already names an emergency stop sent in other words.
    #[error(
        "StopReused: operation `{operation}` already names an emergency stop in other words: send this stop under a new operation id"
    )]
    StopReused {
        /// The operation id.
        operation: String,
    },
    /// The review decisions are not configured, or their log could not be read or written.
    #[error("ReviewsUnavailable: {reason}")]
    ReviewsUnavailable {
        /// What failed.
        reason: String,
    },
    /// The caller may see the grant but is not the one who reviews it.
    #[error(
        "ReviewerOnly: only the person the holding agent answers to, or the root authority, keeps this grant"
    )]
    ReviewerOnly,
    /// The grant is not due for review, so there is nothing to keep.
    #[error("GrantNotDue: grant `{grant}` is not due for review: {why}")]
    GrantNotDue {
        /// The grant.
        grant: String,
        /// Why it is not due.
        why: &'static str,
    },
    /// A batch named a grant held by another identity.
    #[error("AgentMismatch: grant `{grant}` is not held by agent `{agent}`")]
    AgentMismatch {
        /// The mismatched grant.
        grant: String,
        /// The agent whose batch was asked for.
        agent: String,
    },
    /// The operation id already names a review decision made in other words.
    #[error(
        "ReviewReused: operation `{operation}` already names a review decision made in other words: keep this grant under a new operation id"
    )]
    ReviewReused {
        /// The operation id.
        operation: String,
    },
    /// The directory's worker could not be reached.
    #[error("DirectoryUnavailable: {reason}")]
    DirectoryUnavailable {
        /// What failed.
        reason: String,
    },
    /// The store could not be brought under the install's organisation at
    /// boot (DIRECTORY-109 R2): nothing is served until it is.
    #[error("OrganisationBootRefused: {reason}")]
    OrganisationBootRefused {
        /// What was not committed, and why.
        reason: String,
    },
    /// An act is held while the upgrade is still reversible; the copy of
    /// the stores before organisations, when the upgrade kept one, is the
    /// put-back (DIRECTORY-109 R2).
    #[error("UpgradePending: the upgrade is still reversible{}", kept_words(.previous_organisation.as_ref()))]
    UpgradePending {
        /// Where the directory and grant logs before organisations are kept.
        previous_organisation: Option<String>,
    },
    /// The issuer's administration API is not configured, could not be
    /// reached, or answered what could not be read.
    #[error("SignInProvidersUnavailable: {reason}")]
    SignInProvidersUnavailable {
        /// What failed.
        reason: String,
    },
    /// A refusal of Lys's `OpenID` provider.
    #[error("{0}")]
    Provider(#[from] crate::error_provider::ProviderError),
    /// A refusal of the grant change stream or a pass binding (DIRECTORY-089).
    #[error("{0}")]
    GrantStream(#[from] crate::error_grant_stream::GrantStreamError),
    /// A sign-in provider refused the client id it was proved with.
    #[error("ProviderRefused: {provider} did not accept this client id ({status}): {reason}")]
    ProviderRefused {
        /// The provider.
        provider: &'static str,
        /// The status the provider answered, zero when it was never asked.
        status: u16,
        /// The provider's own words.
        reason: String,
    },
    /// The issuer's administration API refused the act.
    #[error("SignInProvidersRefused: the issuer answered {status}: {reason}")]
    SignInProvidersRefused {
        /// The status the issuer answered.
        status: u16,
        /// The issuer's message.
        reason: String,
    },
    /// The caller does not hold the operate relation on the agent.
    #[error("not_permitted: {reason}")]
    NotPermitted {
        /// What was refused.
        reason: String,
    },
    /// The agent has no live session a message can be typed into.
    #[error("no_live_session: agent `{agent}` has no live session to wake")]
    NoLiveSession {
        /// The agent.
        agent: String,
    },
    /// The session's machine names no runner, so Lys does not drive it.
    #[error("runner_absent: machine `{machine}` names no runner")]
    RunnerAbsent {
        /// The machine.
        machine: String,
    },
    /// The authority an act needs is not there to admit it (AGENTS-004):
    /// the act is held, never admitted on a guess.
    #[error("authority_unavailable: {reason}")]
    AuthorityUnavailable {
        /// What was missing.
        reason: String,
    },
    /// A runner refused the act, or could not be reached, by its own name.
    #[error("{refusal}: {words}")]
    Runner {
        /// The refusal's name.
        refusal: String,
        /// Why, in words.
        words: String,
    },
}

/// The words that name the kept copy on an `UpgradePending` refusal.
fn kept_words(previous_organisation: Option<&String>) -> String {
    previous_organisation.map_or_else(String::new, |copy| {
        format!("; the directory and grant logs before organisations are kept at {copy}")
    })
}

#[cfg(test)]
mod repair_tests {
    use super::ServerError;

    #[test]
    fn provider_conversion_preserves_refusal_words_and_status() {
        let original = crate::error_provider::ProviderError::ClientUnknown;
        let words = original.to_string();
        let error = ServerError::from(original);
        assert_eq!(error.to_string(), words);
        assert_eq!(error.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn grant_stream_conversion_preserves_refusal_words_and_status() {
        let original = crate::error_grant_stream::GrantStreamError::BindingDegraded {
            refusal: "projection unavailable".to_owned(),
        };
        let words = original.to_string();
        let error = ServerError::from(original);
        assert_eq!(error.to_string(), words);
        assert_eq!(error.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }
}
