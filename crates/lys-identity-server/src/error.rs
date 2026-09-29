//! Everything the service refuses, each by name; `error_status` maps each
//! to its HTTP answer.

use axum::http::StatusCode;
use lys_identity::IdentityError;
use lys_identity::grants::GrantError;

/// Everything the service refuses.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// A directory refusal.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// A grant refusal, from the grants' one authority owner.
    #[error(transparent)]
    Grant(#[from] GrantError),
    /// An apps refusal: a registration, a schema, a kind or an app's credential.
    #[error(transparent)]
    App(#[from] crate::apps_error::AppError),
    /// The caller has no live session.
    #[error("NotSignedIn: sign in through the configured issuer first")]
    NotSignedIn,
    /// The caller is signed in but not admitted to this act.
    #[error("NotAdmitted: {reason}")]
    NotAdmitted {
        /// Why the caller is not admitted.
        reason: &'static str,
    },
    /// The caller is signed in through a login bound to no person, so no personal view is theirs.
    #[error("NoPerson: the signed-in login is bound to no person in the directory")]
    NoPerson,
    /// The configured administrator has not completed their first-run identity setup.
    #[error("SetupRequired: finish setting up your identity to continue")]
    SetupRequired,
    /// The agent is not one the caller may see: the directory does not hold it, or it answers to another person.
    #[error("AgentNotVisible: no agent by that id is visible to the signed-in caller")]
    AgentNotVisible,
    /// The grant is not one the caller may see: the grants do not hold it, or it is another's.
    #[error("GrantNotVisible: no grant by that id is visible to the signed-in caller")]
    GrantNotVisible,
    /// A grant refusal whose record names a grant or identity the caller may not see.
    #[error("{refusal}: the refusal names a grant or identity the caller may not inspect")]
    Withheld {
        /// The refusal's name, as the whole refusal carries it.
        refusal: String,
    },
    /// The session is not one the caller may end: it is not live, or it belongs to another person.
    #[error("SessionUnknown: no live session by that id is visible to the signed-in caller")]
    SessionUnknown,
    /// A sign-in answer names a state this service did not issue, or one already used.
    #[error("SignInStateUnknown: the sign-in answer does not match a sign-in this service began")]
    SignInStateUnknown,
    /// The email or the password is not right. Which one is never said.
    #[error("SignInRefused: the email or password is not right")]
    SignInRefused,
    /// Too many sign-ins failed from the person's address for now.
    #[error(
        "SignInThrottled: too many sign-ins failed from this address; wait a little, then try again"
    )]
    SignInThrottled,
    /// The account asks for a second factor, which Lys's sign-in does not take.
    #[error(
        "SecondFactorUnsupported: this account asks for a second factor, such as a passkey, which Lys's sign-in does not take yet (act: ask your administrator to reset the account to a password)"
    )]
    SecondFactorUnsupported,
    /// First-run setup is over: an administrator exists.
    #[error(
        "SetupClosed: Lys already has an administrator, so first-run setup is closed (act: sign in, or run lys identity setup-code to set a new password)"
    )]
    SetupClosed,
    /// The setup code is not the one written for this machine, or was used.
    #[error(
        "SetupCodeRefused: this setup link is not valid or was already used (act: run lys identity setup-code for a fresh one)"
    )]
    SetupCodeRefused,
    /// First-run setup is not configured, or its files could not be read or written.
    #[error("SetupUnavailable: {reason}")]
    SetupUnavailable {
        /// What failed.
        reason: String,
    },
    /// The account could not be made or changed as asked.
    #[error("AccountRefused: {reason}")]
    AccountRefused {
        /// What was refused, in Lys's words.
        reason: String,
    },
    /// The issuer could not be reached or its answer did not validate.
    #[error("SignInFailed: {reason}")]
    SignInFailed {
        /// What failed.
        reason: String,
    },
    /// The configuration is not one the service runs under.
    #[error("ConfigInvalid: {reason}")]
    ConfigInvalid {
        /// What is wrong with it.
        reason: String,
    },
    /// A request body or path is not one the route takes.
    #[error("RequestMalformed: {reason}")]
    RequestMalformed {
        /// What is wrong with it.
        reason: String,
    },
    /// The secrets broker is not configured, or could not be reached or read.
    #[error("SecretsUnavailable: {reason}")]
    SecretsUnavailable {
        /// What failed.
        reason: String,
    },
    /// The secrets broker refused the request, by name.
    #[error("{refusal}: {reason}")]
    SecretsRefused {
        /// The status the broker answered with.
        status: StatusCode,
        /// The broker's name for the refusal.
        refusal: String,
        /// The broker's words.
        reason: String,
    },
    /// The access requests are not configured, or their file could not be read or written.
    #[error("RequestsUnavailable: {reason}")]
    RequestsUnavailable {
        /// What failed.
        reason: String,
    },
    /// The request is not one the caller may see: none is kept by that id, or it is another's.
    #[error("RequestUnknown: no access request by that id is visible to the signed-in caller")]
    RequestUnknown,
    /// The request was already decided, another way.
    #[error("RequestDecided: access request `{request}` is already decided")]
    RequestDecided {
        /// The request.
        request: String,
    },
    /// An approval of the request is being settled, and this is not that approval.
    #[error(
        "RequestHeld: an approval of access request `{request}` by {by} is being settled, and until it is only that approval is taken"
    )]
    RequestHeld {
        /// The request.
        request: String,
        /// The person whose approval is being settled.
        by: String,
    },
    /// The operation id already names a request asked in other words.
    #[error(
        "RequestReused: operation `{request}` already names an access request asked in other words"
    )]
    RequestReused {
        /// The operation id.
        request: String,
    },
    /// The machines cannot be read or written.
    #[error("NetworkUnavailable: {reason}")]
    NetworkUnavailable {
        /// What failed.
        reason: String,
    },
    /// No person in the directory holds the login that was asked about.
    #[error(
        "LoginUnbound: no person in the directory holds that login (act: bind the login to its person in the directory, then ask again)"
    )]
    LoginUnbound,
    /// No machine is kept by that id.
    #[error("MachineUnknown: no machine is kept by that id")]
    MachineUnknown,
    /// The operation id already names a machine named in other words.
    #[error("MachineReused: operation `{machine}` already names a machine named in other words")]
    MachineReused {
        /// The operation id.
        machine: String,
    },
    /// The memory of an agent cannot be read.
    #[error("MemoryUnavailable: {reason}")]
    MemoryUnavailable {
        /// What failed.
        reason: String,
    },
    /// The provisioning profiles cannot be read or written.
    #[error("ProvisioningUnavailable: {reason}")]
    ProvisioningUnavailable {
        /// What failed.
        reason: String,
    },
    /// The profile is no longer at the version the change was set over.
    #[error(
        "ProvisioningChanged: the profile is at version {latest}, not the version this change was set over: read the profile again and set the change over version {latest}"
    )]
    ProvisioningChanged {
        /// The latest version kept.
        latest: u32,
    },
    /// The operation id already names a profile version set in other words.
    #[error(
        "ProvisioningReused: operation `{operation}` already names a profile version set in other words: set this change under a new operation id"
    )]
    ProvisioningReused {
        /// The operation id.
        operation: String,
    },
    /// The agent's profile holds no version by that number.
    #[error("ProfileVersionUnknown: the agent's profile holds no version {version}")]
    ProfileVersionUnknown {
        /// The version asked for.
        version: u32,
    },
    /// The agent's latest profile version is not reviewed, so it is not started.
    #[error(
        "ProfileNotReviewed: version {version} of the agent's profile is not reviewed; review it before the agent is started"
    )]
    ProfileNotReviewed {
        /// The version.
        version: u32,
    },
    /// The certificate log cannot be read or written.
    #[error("CertificatesUnavailable: {reason}")]
    CertificatesUnavailable {
        /// What failed.
        reason: String,
    },
    /// No certificate is kept by that serial.
    #[error("CertificateUnknown: no certificate is kept by serial `{serial}`")]
    CertificateUnknown {
        /// The serial asked for.
        serial: String,
    },
    /// The serial already names a certificate issued with other bytes.
    #[error(
        "CertificateReused: serial `{serial}` already names a certificate with other bytes: issue this certificate under a new serial"
    )]
    CertificateReused {
        /// The serial.
        serial: String,
    },
    /// The certificate was already withdrawn by someone else, or at another time.
    #[error("CertificateWithdrawn: certificate `{serial}` was already withdrawn by {by}")]
    CertificateWithdrawn {
        /// The serial.
        serial: String,
        /// Who withdrew it.
        by: String,
    },
    /// The roles cannot be read or written.
    #[error("RolesUnavailable: {reason}")]
    RolesUnavailable {
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
    /// A model the declared harness cannot carry.
    #[error("ModelUnrepresentable: the harness `{harness}` cannot carry model `{model}`: {reason}")]
    ModelUnrepresentable {
        /// The declared harness kind.
        harness: String,
        /// The model.
        model: String,
        /// Why it cannot.
        reason: String,
    },
    /// The profile declares no harness build to start.
    #[error(
        "HarnessUndeclared: profile version {version} declares no harness build to start; declare the build's kind, program and package in the profile"
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
    /// The teams are not configured, or their log could not be read or written.
    #[error("TeamsUnavailable: {reason}")]
    TeamsUnavailable {
        /// Why.
        reason: String,
    },
    /// No team by that id was ever created.
    #[error("TeamUnknown: no team by that id was ever created")]
    TeamUnknown,
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
    /// The operation id already names a team act sent in other words.
    #[error(
        "TeamReused: operation `{operation}` already names a team act in other words: send this act under a new operation id"
    )]
    TeamReused {
        /// The operation id.
        operation: String,
    },
    /// The team is retired and takes no more changes.
    #[error("TeamRetired: team `{team}` is retired and takes no more changes")]
    TeamRetired {
        /// The team.
        team: String,
    },
    /// The member is already in the team.
    #[error("TeamMemberHeld: that member is already in the team")]
    TeamMemberHeld,
    /// The member is not in the team.
    #[error("TeamMemberAbsent: that member is not in the team")]
    TeamMemberAbsent,
    /// The member named is not a person or agent the directory holds, or is retired.
    #[error(
        "TeamMemberUnknown: a member is a person or agent the directory holds and has not retired"
    )]
    TeamMemberUnknown,
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
    /// The issuer's administration API is not configured, could not be
    /// reached, or answered what could not be read.
    #[error("SignInProvidersUnavailable: {reason}")]
    SignInProvidersUnavailable {
        /// What failed.
        reason: String,
    },
    /// Lys's `OpenID` provider is not configured, or cannot answer now.
    #[error("ProviderUnavailable: {reason}")]
    ProviderUnavailable {
        /// What failed.
        reason: String,
    },
    /// No product is registered with that client id, or its secret is wrong.
    #[error("ClientUnknown: no product is registered with that client id and secret")]
    ClientUnknown,
    /// The redirect address is not one registered for the product.
    #[error("RedirectUnregistered: that redirect address is not registered for this product")]
    RedirectUnregistered,
    /// The code is not one Lys answered this product with.
    #[error("CodeUnknown: that code is not one Lys gave this product")]
    CodeUnknown,
    /// The code was already exchanged.
    #[error("CodeUsed: that code was already used")]
    CodeUsed,
    /// The code is past its instant.
    #[error("CodeExpired: that code is past the instant it was good until")]
    CodeExpired,
    /// The PKCE verifier does not match the code's challenge.
    #[error("VerifierWrong: the PKCE verifier does not match the code's challenge")]
    VerifierWrong,
    /// The access token is not one Lys issued, or is past its instant.
    #[error("TokenUnknown: that access token is not one Lys issued, or it has ended")]
    TokenUnknown,
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
    /// A runner refused the act, or could not be reached, by its own name.
    #[error("{refusal}: {words}")]
    Runner {
        /// The refusal's name.
        refusal: String,
        /// Why, in words.
        words: String,
    },
    /// A dial request was not signed by the machine's key, or names a
    /// machine with no dialled runner.
    #[error("runner_dial_refused: {reason}")]
    DialRefused {
        /// Why.
        reason: String,
    },
    /// A dial was signed under an epoch not this server's: it was made
    /// before the server last started, or captured and sent again after.
    #[error("runner_dial_stale: {reason}")]
    DialStale {
        /// Why.
        reason: String,
    },
}
