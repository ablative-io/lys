//! [`GrantError`], the grant contract's error type.
//!
//! Each variant names the rule a grant, a request or a stored grant event
//! broke, in its message's first word, so a caller can say which rule and act
//! on it. No variant carries a default: a refusal is never turned into a
//! permission by leaving something out.

use super::types::RecipientKind;
use crate::error::IdentityError;
use crate::lifecycle::LifecycleState;

/// Errors returned by the grant contract, its model and its admission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GrantError {
    /// A grant id's text form is not the form this contract writes.
    #[error("GrantIdMalformed: `{text}` is not a grant identifier")]
    GrantIdMalformed {
        /// The text that was given.
        text: String,
    },
    /// An action, relation or resource name is not a token this contract records.
    #[error(
        "TokenInvalid: the {kind} `{text}` is not a token of 1 to {max} bytes of a-z, 0-9, `_`, `-` and `.` (a resource id also A-Z)"
    )]
    TokenInvalid {
        /// What the token names.
        kind: &'static str,
        /// The text that was given.
        text: String,
        /// The most bytes the permission engine takes for it.
        max: usize,
    },
    /// A model is not one a decision can be made against.
    #[error("ModelInvalid: {reason}")]
    ModelInvalid {
        /// The rule the model broke.
        reason: &'static str,
    },
    /// A relation is not defined by the model the request is judged against.
    #[error("RelationUnknown: model version {model_version} defines no relation `{relation}`")]
    RelationUnknown {
        /// The relation named.
        relation: String,
        /// The model version it was looked up in.
        model_version: u64,
    },
    /// A requested relation resolves to actions outside the authority it is judged against.
    #[error(
        "ActionsOutside: `{relation}` in model version {model_version} carries {outside}, outside the authority held"
    )]
    ActionsOutside {
        /// The relation requested.
        relation: String,
        /// The actions it carries that the authority does not.
        outside: String,
        /// The model version the relation was resolved in.
        model_version: u64,
    },
    /// A grant or request leaves out authority it must state affirmatively.
    #[error("AuthorityAbsent: the {member} is empty, and an empty authority is never a permission")]
    AuthorityAbsent {
        /// The member that was empty.
        member: &'static str,
    },
    /// A grant's pass-on authority is wider than what it may exercise.
    #[error("PassOnOutside: a grant may pass on only actions it may itself exercise")]
    PassOnOutside,
    /// A grant's time window is not one this contract records.
    #[error("WindowInvalid: {reason}")]
    WindowInvalid {
        /// The rule the window broke.
        reason: &'static str,
    },
    /// A grant's lineage is not a lineage this contract records.
    #[error("LineageMalformed: {reason}")]
    LineageMalformed {
        /// The rule the lineage broke.
        reason: &'static str,
    },
    /// An encoded grant carries a member this contract does not define.
    #[error("MemberUnknown: key {key} is not a member of a grant")]
    MemberUnknown {
        /// The key found.
        key: u64,
    },
    /// An encoded grant leaves out a member this contract requires.
    #[error("MemberMissing: the grant leaves out its {member}, and nothing stands in for it")]
    MemberMissing {
        /// The member left out.
        member: &'static str,
    },
    /// An encoded grant names a recipient kind this contract does not define.
    #[error("RecipientKindUnknown: code {code} is not a recipient kind")]
    RecipientKindUnknown {
        /// The code found.
        code: u64,
    },
    /// An encoded grant does not decode to the contract's shape.
    #[error("GrantMalformed: {reason}")]
    GrantMalformed {
        /// The part of the shape that was wrong.
        reason: &'static str,
    },
    /// An encoded grant decodes, but its bytes are not the canonical encoding of what it decodes to.
    #[error(
        "GrantNotCanonical: the grant's bytes are not the canonical encoding of what they decode to"
    )]
    GrantNotCanonical,
    /// A grant or request names a source grant the book does not hold.
    #[error("SourceUnknown: no grant {grant} is held, so nothing derives from it")]
    SourceUnknown {
        /// The grant named.
        grant: String,
    },
    /// An ancestry returns to a grant already on it, or runs past the longest ancestry walked.
    #[error("LineageCycle: the ancestry returns to {grant}, and a cycle reaches no person")]
    LineageCycle {
        /// The grant met twice.
        grant: String,
    },
    /// A grant names an issuer other than the holder of its source.
    #[error(
        "IssuerNotHolder: {grant} was issued by someone other than the holder of {source_grant}"
    )]
    IssuerNotHolder {
        /// The grant.
        grant: String,
        /// Its source.
        source_grant: String,
    },
    /// A caller asked to pass on a grant it does not hold.
    #[error("NotHolder: {caller} does not hold {grant}")]
    NotHolder {
        /// The caller.
        caller: String,
        /// The grant named.
        grant: String,
    },
    /// A request or grant is on a resource other than its source's.
    #[error("ResourceOutside: {requested} is not the resource of {source_grant}")]
    ResourceOutside {
        /// The resource requested.
        requested: String,
        /// The source grant.
        source_grant: String,
    },
    /// A grant that may be exercised but not passed on was asked to be passed on.
    #[error("UseOnly: {grant} may be exercised and not passed on")]
    UseOnly {
        /// The use-only grant.
        grant: String,
    },
    /// A grant may not be passed on to this kind of recipient.
    #[error("RecipientRefused: {grant} may not be passed on to a {kind}")]
    RecipientRefused {
        /// The recipient's kind.
        kind: RecipientKind,
        /// The source grant.
        grant: String,
    },
    /// An agent was to be given acts no agent may hold.
    #[error(
        "WithheldFromAgents: `{relation}` carries {withheld}, which no agent may be given; a person may hold it"
    )]
    WithheldFromAgents {
        /// The relation asked for.
        relation: String,
        /// The withheld acts it carries, comma-separated.
        withheld: String,
    },
    /// A request asks to let the recipient pass on more than the source lets be passed on.
    #[error(
        "PassOnBeyondSource: the requested pass-on is wider than {source_grant} lets be passed on"
    )]
    PassOnBeyondSource {
        /// The source grant.
        source_grant: String,
    },
    /// A requested end is later than its source grant's end. It is refused, never clamped.
    #[error(
        "ExpiryBeyondSource: the requested end ({requested}) is later than {source_grant}, which ends at {source_ends}"
    )]
    ExpiryBeyondSource {
        /// The end requested.
        requested: String,
        /// The source grant.
        source_grant: String,
        /// When the source ends.
        source_ends: u64,
    },
    /// A request names a responsible person the directory does not record for the recipient.
    #[error("ResponsibleMismatch: {identity} is answered for by {recorded}, not {named}")]
    ResponsibleMismatch {
        /// The identity.
        identity: String,
        /// The person the request named.
        named: String,
        /// The person the directory records.
        recorded: String,
    },
    /// An identity on the path is not active, so no grant it holds is effective.
    #[error(
        "IdentityNotActive: {identity} is {state}, and only an active identity's grants are effective"
    )]
    IdentityNotActive {
        /// The identity.
        identity: String,
        /// Its state.
        state: LifecycleState,
    },
    /// A machine was to issue, pass on or approve, or to be given a
    /// responsibility a person keeps (ACCESS-005 R1). A machine exercises
    /// what it is given and never gives it on.
    #[error("MachineRefused: {machine}: a machine never {act}")]
    MachineRefused {
        /// The machine, or the grant whose recipients name the machine kind.
        machine: String,
        /// What it may never do, in words.
        act: String,
    },
    /// A grant on the path was revoked.
    #[error("Revoked: {grant} was revoked, and nothing derived from it is effective")]
    Revoked {
        /// The revoked grant.
        grant: String,
    },
    /// A grant on the path has ended.
    #[error("Expired: {grant} ended at {ended_at}")]
    Expired {
        /// The grant whose end was reached.
        grant: String,
        /// When it ended.
        ended_at: u64,
    },
    /// A grant on the path has not started.
    #[error("NotStarted: {grant} starts at {starts_at}")]
    NotStarted {
        /// The grant.
        grant: String,
        /// When it starts.
        starts_at: u64,
    },
    /// A root grant was asked for by someone other than the configured root authority.
    #[error(
        "RootAuthorityRefused: {caller} is not the root authority, and only it issues a root grant"
    )]
    RootAuthorityRefused {
        /// The caller.
        caller: String,
    },
    /// A revocation was asked for by someone with no authority over the grant.
    #[error("RevokeRefused: {caller} neither issued {grant} nor holds a grant it derives from")]
    RevokeRefused {
        /// The caller.
        caller: String,
        /// The grant.
        grant: String,
    },
    /// A grant was already revoked.
    #[error("AlreadyRevoked: {grant} is already revoked")]
    AlreadyRevoked {
        /// The grant.
        grant: String,
    },
    /// A grant id is already held by the book.
    #[error("GrantExists: {grant} is already issued")]
    GrantExists {
        /// The grant.
        grant: String,
    },
    /// The caller holds no grant carrying the action on the resource.
    #[error("NotHeld: {identity} holds no grant of {action} on {resource}")]
    NotHeld {
        /// The caller.
        identity: String,
        /// The resource.
        resource: String,
        /// The action.
        action: String,
    },
    /// An operation id was already used for a different request.
    #[error("OperationReused: {operation} already names a different request")]
    OperationReused {
        /// The operation id.
        operation: String,
    },
    /// A grant event's parts do not fit together.
    #[error("EventMismatch: {reason}")]
    EventMismatch {
        /// The rule the event broke.
        reason: &'static str,
    },
    /// A request names a grant the book does not hold.
    #[error("GrantUnknown: no grant {grant} is held")]
    GrantUnknown {
        /// The grant named.
        grant: String,
    },
    /// A signed message names an envelope other than the grant envelope.
    #[error("EnvelopeMismatch: the message is not a grant event: {reason}")]
    EnvelopeMismatch {
        /// What was found instead.
        reason: String,
    },
    /// A grant event's bytes do not decode to the envelope's shape.
    #[error("EventMalformed: {reason}")]
    EventMalformed {
        /// The part of the shape that was wrong.
        reason: &'static str,
    },
    /// A grant event decodes, but its bytes are not the canonical encoding of what it decodes to.
    #[error(
        "EventNotCanonical: the grant event's bytes are not the canonical encoding of what they decode to"
    )]
    EventNotCanonical,
    /// A grant event names a version the grants do not read.
    #[error("VersionUnsupported: grant event version {version} is not one the grants read")]
    VersionUnsupported {
        /// The version named.
        version: u64,
    },
    /// A grant event names a service key other than the one it is verified against.
    #[error(
        "SignerMismatch: the grant event names a service key other than the one it is verified against"
    )]
    SignerMismatch,
    /// A grant event's signature does not verify.
    #[error("SignatureInvalid: the grant event's signature does not verify")]
    SignatureInvalid,
    /// A receipt does not match the signed grant event and log position it is checked against.
    #[error("ReceiptInvalid: {reason}")]
    ReceiptInvalid {
        /// The first thing that did not match.
        reason: &'static str,
    },
    /// The grant log could not be opened, read or written.
    #[error("LogUnavailable: {reason}")]
    LogUnavailable {
        /// What the log store reported.
        reason: String,
    },
    /// A leaf of the grant log is not a grant event this service signed.
    #[error("LeafNotAnEvent: leaf {index} is not a grant event this service signed: {reason}")]
    LeafNotAnEvent {
        /// The leaf's index.
        index: u64,
        /// Why it was refused.
        reason: String,
    },
    /// An append failed and its leaf is known not to be in the log: nothing was recorded.
    #[error("AppendRefused: the grant change was not recorded: {reason}")]
    AppendRefused {
        /// What the log store reported.
        reason: String,
    },
    /// Whether an operation's event was committed is not yet known.
    #[error(
        "OperationUnresolved: whether {operation} on {grant} was committed is not yet known, and nothing depending on it is answered until it is"
    )]
    OperationUnresolved {
        /// The operation.
        operation: String,
        /// The grant it changes.
        grant: String,
    },
    /// An operation's event is committed and not yet in the permission relationships.
    #[error(
        "ProjectionPending: {operation} on {grant} is committed at index {index} and not yet projected into the permission relationships"
    )]
    ProjectionPending {
        /// The operation.
        operation: String,
        /// The grant it changes.
        grant: String,
        /// The index of its event.
        index: u64,
    },
    /// The permission relationships are older than the revision the decision needs.
    #[error(
        "StaleDecision: the decision needs revision {required} and the permission relationships stand at {projected}"
    )]
    StaleDecision {
        /// The revision required.
        required: u64,
        /// The revision projected.
        projected: u64,
    },
    /// The permission engine could not be read or written.
    #[error("PermissionEngineUnavailable: {reason}")]
    PermissionEngineUnavailable {
        /// What the engine reported.
        reason: String,
    },
    /// A grant names a resource kind the permission engine cannot hold; it is
    /// refused before anything is committed.
    #[error("ResourceKindUnheld: resource kind {kind} cannot be held: {reason}")]
    ResourceKindUnheld {
        /// The resource kind named.
        kind: String,
        /// Why the engine cannot hold it and who can change that.
        reason: String,
    },
    /// The permission relationships hold no live relationship for a grant the book holds.
    #[error("PermissionAbsent: the permission relationships hold no live relationship for {grant}")]
    PermissionAbsent {
        /// The grant.
        grant: String,
    },
    /// The grant is held by draft or by two: its act is taken through an
    /// approved draft, never exercised at once (ACCESS-001 R1).
    #[error("ModeHeld: {grant} is held {mode}, so its act is taken through an approved draft")]
    ModeHeld {
        /// The grant.
        grant: String,
        /// Its mode: `by_draft` or `by_two`.
        mode: &'static str,
    },
    /// A directory refusal met while judging a grant.
    #[error(transparent)]
    Identity(#[from] IdentityError),
}
