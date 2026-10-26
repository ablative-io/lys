//! Directory, authentication, certificate and configuration refusal declarations.
macro_rules! directory_refusals {
    ($($service:tt)*) => {
        /// Everything the service refuses.
        #[derive(Debug, thiserror::Error)]
        pub enum ServerError {

    /// A team refusal, preserving its status and response words.
    #[error(transparent)]
    Team(#[from] crate::error_team::TeamError),
    /// The agents cluster's refusals (budgets, goals, words, variables, schedules, seats and seat imports), each keeping its own status and name; one variant so this file stays under the length gate.
    #[error(transparent)]
    Agents(#[from] crate::error_agents::AgentsError),
    /// A refusal of the calls routes.
    #[error(transparent)]
    Call(#[from] crate::error_call::CallError),
    /// A giver's holding does not cover the requested gift.
    #[error(transparent)]
    Holding(#[from] crate::error_holding::HoldingError),
    /// A catalogue description could not be read before serving.
    #[error("harness_catalogue_unreadable: {file}: {reason}")]
    HarnessCatalogueUnreadable {
        /// The description that could not be read.
        file: String,
        /// What made it unreadable.
        reason: String,
    },
    /// A directory refusal.
    #[error(transparent)]
    Identity(#[from] lys_identity::IdentityError),
    /// A grant refusal, from the grants' one authority owner.
    #[error(transparent)]
    Grant(#[from] lys_identity::grants::GrantError),
    /// An apps refusal: a registration, a schema, a kind or an app's credential.
    #[error(transparent)]
    App(#[from] crate::apps_error::AppError),
    /// The caller's directory identity is not Active for the requested act.
    #[error("inactive: {identity} is {state} and may not act")]
    Inactive {
        /// The person or agent refused.
        identity: String,
        /// The current recorded lifecycle state.
        state: lys_identity::LifecycleState,
    },
    /// The caller has no live session.
    #[error("NotSignedIn: sign in through the configured issuer first")]
    NotSignedIn,
    /// The caller is signed in but not admitted to this act.
    #[error("NotAdmitted: {reason}")]
    NotAdmitted {
        /// Why the caller is not admitted.
        reason: &'static str,
    },
    /// The body names an organisation that is not the caller's own
    /// (ACCESS-009 R1): a person is admitted into the caller's.
    #[error(
        "organisation_not_own: {organisation} is not the caller's organisation; a person is admitted into the caller's own"
    )]
    OrganisationNotOwn {
        /// The organisation the body named.
        organisation: String,
    },
    /// A product asked to admit a person with no first grant (DIRECTORY-113
    /// R1): a product admits only with the person's first grant on its own
    /// kind.
    #[error(
        "app_admit_grant_required: a product admits a person only with their first grant on its own kind"
    )]
    AppAdmitGrantRequired,
    /// An email already opens a record the caller did not name.
    #[error("EmailHeldByAnother: {email} already signs in as {display_name} ({person})")]
    EmailHeldByAnother {
        /// The person the email signs in as.
        person: String,
        /// Their stored display name.
        display_name: String,
        /// The address held by that account.
        email: String,
    },
    /// A browser already holds another person's session.
    #[error("SignedInAsAnother: You are signed in as {signed_in}. This link is for {intended}. Ask the person who sent it.")]
    SignedInAsAnother {
        /// The current person's display name.
        signed_in: String,
        /// The link's person's display name.
        intended: String,
    },
    /// An own email change needs a letter to prove the new address.
    #[error("EmailChangeNeedsMail: Lys has no mail settings. Ask an administrator to change the email from your Account page.")]
    EmailChangeNeedsMail,
    /// A code is wrong, replaced, spent, or belongs to another account.
    #[error("EmailChangeCodeRefused: This email code is not waiting for you, or was replaced or used. Ask for a new code.")]
    EmailChangeCodeRefused,
    /// The issuer account does not agree with the person's held address.
    #[error("ProviderEmailMismatch: {provider} returned the sign-in account {account}, but Lys holds {held} for this person. Ask an administrator to check your Account page.")]
    ProviderEmailMismatch {
        /// The provider of this login.
        provider: String,
        /// The address Lys holds for this person, masked.
        held: String,
        /// The issuer account's address, masked.
        account: String,
    },
    /// The source no longer holds the chosen login.
    #[error("LoginNotBound: This login is not bound to the source person.")]
    LoginNotBound,
    /// The source and target are the person already holding this login.
    #[error("SameTarget: This sign-in is already bound to that person.")]
    SameTarget,
    /// Both people must belong to the administrator's organisation.
    #[error("NotSameOrganisation: The source and target must belong to the same organisation.")]
    NotSameOrganisation,
    /// No unspent admission link is waiting for the person.
    #[error("NoUnspentAdmission: no unspent admission link is waiting for this person")]
    NoUnspentAdmission,
    /// A proving link that Lys did not issue, or one already used
    /// (DIRECTORY-111 R2): one name and one text for both, so the link's
    /// standing is never read off the refusal.
    #[error(
        "proof_refused: This link is not one Lys issued, or it was already used. Found again from the sign-in page."
    )]
    ProofRefused,
    /// The address being proven already signs in to Lys as an active
    /// person who is not the founder (DIRECTORY-111 R2).
    #[error(
        "already_admitted: That address already signs in to Lys. Sign in there; nothing was founded."
    )]
    AlreadyAdmitted,
    /// A letter did not go; the act it reports is recorded (DIRECTORY-110 R3).
    #[error(
        "mail_not_sent: the {letter} letter to {to} was not sent: {reply}; the act is recorded, ask again with the same operation to send it"
    )]
    MailNotSent {
        /// The letter: `added`, `access_given` or `founded`.
        letter: String,
        /// The address it was for.
        to: String,
        /// The mail server's reply, or why the letter could not be built.
        reply: String,
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
    /// The challenge expired before the issuer could accept it.
    #[error(
        "IssuerChallengeExpired: the sign-in challenge expired; press sign-in again to start fresh"
    )]
    IssuerChallengeExpired,
    /// An issuer refusal carries only its status and error word.
    #[error("IssuerRefused: the sign-in issuer answered {status} {error}")]
    IssuerRefused {
        /// The HTTP status the issuer answered.
        status: u16,
        /// Only the issuer's error word is retained; unreadable bodies name no private data.
        error: String,
    },
    /// Too many sign-ins failed from the person's address for now.
    #[error(
        "SignInThrottled: too many sign-ins failed from this address; wait a little, then try again"
    )]
    SignInThrottled,
    /// Too many apps registered to connect in the last minute.
    #[error(
        "RegistrationThrottled: too many apps registered to connect in the last minute; wait a minute, then register again"
    )]
    RegistrationThrottled,
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
    /// Recorded bootstrap authority requires an explicit administrator recovery.
    #[error("BootstrapInterrupted: {reason}")]
    BootstrapInterrupted {
        /// The interruption and exact recovery requests.
        reason: String,
    },
    /// A request body or path is not one the route takes.
    #[error("RequestMalformed: {reason}")]
    RequestMalformed {
        /// What is wrong with it.
        reason: String,
    },
    /// The owned clock could not supply a representable instant.
    #[error("ClockUnavailable: {reason}")]
    ClockUnavailable {
        /// The clock read or conversion failure.
        reason: String,
    },
    /// A body longer than is read before its caller is verified:
    /// `signed_first::UNVERIFIED_BODY_LIMIT` bytes.
    #[error("BodyTooLarge: the body is longer than is read before its caller is verified")]
    BodyTooLarge,
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
        status: axum::http::StatusCode,
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
    /// Requests for declared MCP servers cannot be read or recorded.
    #[error("McpRequestsUnavailable: {reason}")]
    McpRequestsUnavailable {
        /// What failed.
        reason: String,
    },
    /// No reviewed provisioning version declares the named server.
    #[error("mcp_server_unknown: no reviewed profile declares MCP server `{server}`")]
    McpServerUnknown {
        /// The server requested.
        server: String,
    },
    /// The requesting agent's most recent reviewed version already carries it.
    #[error("mcp_server_held: the latest reviewed profile already carries MCP server `{server}`")]
    McpServerHeld {
        /// The server requested.
        server: String,
    },
    /// The caller cannot pass this server to the requesting agent.
    #[error(
        "mcp_beyond_remit: approver `{approver}` cannot approve MCP server `{server}` for agent `{agent}`"
    )]
    McpBeyondRemit {
        /// The approver's identity.
        approver: String,
        /// The requesting agent.
        agent: String,
        /// The server requested.
        server: String,
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
    /// A machine act's operation id already names a different act.
    #[error(transparent)]
    Machine(#[from] crate::error_machine::MachineError),
    /// A refusal of the master off switch, or of a start it holds back.
    #[error(transparent)]
    Cord(#[from] crate::error_cord::CordError),
    /// A refusal of a person's canvas.
    #[error(transparent)]
    Canvas(#[from] crate::error_canvas::CanvasError),
    /// A refusal of a product draft route (ACCESS-001 R3).
    #[error(transparent)]
    ProductDraft(#[from] crate::error_product_draft::ProductDraftError),
    /// The signed-in sessions could not be kept or read back.
    #[error("SessionsUnavailable: {reason}")]
    SessionsUnavailable {
        /// What failed.
        reason: String,
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
    /// A recorded version was replaced and cannot be approved for a new start.
    #[error("version {version} has been replaced by version {latest}; approve the latest")]
    ProfileVersionReplaced {
        /// The recorded version whose review was requested.
        version: u32,
        /// The latest recorded version, which must be reviewed instead.
        latest: u32,
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
            $($service)*
        }
    };
}
pub(super) use directory_refusals;
