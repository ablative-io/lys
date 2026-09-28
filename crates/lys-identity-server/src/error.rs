//! The mapping of every refusal to an HTTP answer, each by name.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
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
    /// No machine is kept by that id.
    #[error("MachineUnknown: no machine is kept by that id")]
    MachineUnknown,
    /// The operation id already names a machine named in other words.
    #[error("MachineReused: operation `{machine}` already names a machine named in other words")]
    MachineReused {
        /// The operation id.
        machine: String,
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
    /// The launch template the profile renders to is not one the home takes.
    #[error("LaunchUnrenderable: {reason}")]
    LaunchUnrenderable {
        /// What the home refused.
        reason: String,
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
    /// The directory's worker could not be reached.
    #[error("DirectoryUnavailable: {reason}")]
    DirectoryUnavailable {
        /// What failed.
        reason: String,
    },
}

impl ServerError {
    /// The refusal's name: the first word of its message.
    pub fn name(&self) -> String {
        let text = self.to_string();
        text.split(':').next().unwrap_or_default().to_owned()
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotSignedIn => StatusCode::UNAUTHORIZED,
            Self::NotAdmitted { .. }
            | Self::NoPerson
            | Self::SetupRequired
            | Self::Withheld { .. }
            | Self::MachineNotForAgent => StatusCode::FORBIDDEN,
            Self::AgentNotVisible
            | Self::GrantNotVisible
            | Self::SessionUnknown
            | Self::RequestUnknown
            | Self::MachineUnknown
            | Self::LaunchRecordMissing
            | Self::RuntimeSessionUnknown
            | Self::RoleUnknown
            | Self::RoleVersionUnknown
            | Self::HolderUnknown
            | Self::ServiceAccountUnknown
            | Self::CertificateUnknown { .. } => StatusCode::NOT_FOUND,
            Self::RequestDecided { .. }
            | Self::RequestHeld { .. }
            | Self::RequestReused { .. }
            | Self::MachineReused { .. }
            | Self::RoleReused { .. }
            | Self::RoleHeld { .. }
            | Self::HoldingOver { .. }
            | Self::HoldingChanged
            | Self::ProvisioningChanged { .. }
            | Self::ProvisioningReused { .. }
            | Self::CertificateReused { .. }
            | Self::CertificateWithdrawn { .. }
            | Self::MachineRetired
            | Self::AgentNotActive { .. }
            | Self::MachineCannotReach { .. }
            | Self::MachineWithoutRuntime
            | Self::LaunchUnrenderable { .. }
            | Self::RuntimeSessionStarted { .. }
            | Self::RuntimeSessionStopped { .. }
            | Self::RuntimeReportReused { .. }
            | Self::ServiceAccountReused { .. }
            | Self::ServiceAccountRetired { .. }
            | Self::ServiceAccountOwnerRetired { .. } => StatusCode::CONFLICT,
            Self::SignInStateUnknown | Self::RequestMalformed { .. } => StatusCode::BAD_REQUEST,
            Self::SignInFailed { .. } | Self::SecretsUnavailable { .. } => StatusCode::BAD_GATEWAY,
            Self::ConfigInvalid { .. }
            | Self::DirectoryUnavailable { .. }
            | Self::RequestsUnavailable { .. }
            | Self::NetworkUnavailable { .. }
            | Self::RolesUnavailable { .. }
            | Self::ProvisioningUnavailable { .. }
            | Self::CertificatesUnavailable { .. }
            | Self::RuntimeUnavailable { .. }
            | Self::ServiceAccountsUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::SecretsRefused { status, .. } => *status,
            Self::Identity(error) => identity_status(error),
            Self::Grant(error) => grant_status(error),
        }
    }
}

fn identity_status(error: &IdentityError) -> StatusCode {
    match error {
        IdentityError::IdentityUnknown { .. } => StatusCode::NOT_FOUND,
        IdentityError::AppendUncertain { .. }
        | IdentityError::LogUnavailable { .. }
        | IdentityError::AppendRefused { .. }
        | IdentityError::RandomSourceUnavailable { .. }
        | IdentityError::KeyUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::CONFLICT,
    }
}

fn grant_status(error: &GrantError) -> StatusCode {
    match error {
        GrantError::Identity(error) => identity_status(error),
        GrantError::GrantIdMalformed { .. }
        | GrantError::TokenInvalid { .. }
        | GrantError::AuthorityAbsent { .. }
        | GrantError::PassOnOutside
        | GrantError::WindowInvalid { .. }
        | GrantError::LineageMalformed { .. }
        | GrantError::MemberUnknown { .. }
        | GrantError::MemberMissing { .. }
        | GrantError::RecipientKindUnknown { .. }
        | GrantError::GrantMalformed { .. }
        | GrantError::GrantNotCanonical
        | GrantError::EventMalformed { .. }
        | GrantError::EventNotCanonical
        | GrantError::EventTooLarge { .. } => StatusCode::BAD_REQUEST,
        GrantError::SourceUnknown { .. } | GrantError::GrantUnknown { .. } => StatusCode::NOT_FOUND,
        GrantError::ModelInvalid { .. }
        | GrantError::EventMismatch { .. }
        | GrantError::VersionUnsupported { .. }
        | GrantError::SignerMismatch
        | GrantError::SignatureInvalid
        | GrantError::ReceiptInvalid { .. }
        | GrantError::LogUnavailable { .. }
        | GrantError::LeafNotAnEvent { .. }
        | GrantError::AppendRefused { .. }
        | GrantError::OperationUnresolved { .. }
        | GrantError::ProjectionPending { .. }
        | GrantError::StaleDecision { .. }
        | GrantError::PermissionEngineUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        GrantError::OperationReused { .. }
        | GrantError::GrantExists { .. }
        | GrantError::AlreadyRevoked { .. } => StatusCode::CONFLICT,
        _ => StatusCode::FORBIDDEN,
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "refusal": self.name(), "reason": self.to_string() });
        (self.status(), Json(body)).into_response()
    }
}
