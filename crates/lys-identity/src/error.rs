//! [`IdentityError`], the directory's error type.
//!
//! Each variant names the rule a request or a stored event broke, so a caller
//! can always say which rule and act on it. The one place detail is withheld
//! is signature verification: a forged or altered event is reported as
//! [`IdentityError::SignatureInvalid`] and nothing more.

use crate::lifecycle::{LifecycleState, Transition};

/// Errors returned by the directory's records and its event encoding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    /// The approval does not bind the immutable creation payload.
    #[error("DraftHashMismatch: approval names another draft payload")]
    DraftHashMismatch,
    /// The named draft has not been recorded.
    #[error("DraftNotFound: no draft {draft} is recorded")]
    DraftNotFound {
        /// The requested draft identifier.
        draft: String,
    },
    /// A terminal decision was already recorded for the draft.
    #[error("DraftNotPending: draft {draft} already has a decision")]
    DraftNotPending {
        /// The draft identifier.
        draft: String,
    },
    /// The prepared action or request evidence is invalid.
    #[error("DraftChangeInvalid: {reason}")]
    DraftChangeInvalid {
        /// The failed boundary rule.
        reason: &'static str,
    },
    /// A draft leaf was requested as an identity event.
    #[error("DraftEntry: the leaf records a draft, not an identity change")]
    DraftEntry,
    /// The directory snapshot must be upgraded before ordinary opening.
    #[error(
        "DirectorySnapshotUnmigrated: snapshot version {found} requires migration to {expected}"
    )]
    DirectorySnapshotUnmigrated {
        /// The stored version.
        found: u64,
        /// The version required by this reader.
        expected: u64,
    },
    /// A reporting target is not held in the directory.
    #[error("AnswersToUnknown: the directory holds no reporting target {identity}")]
    AnswersToUnknown {
        /// The unknown target.
        identity: String,
    },
    /// The selected target cannot accept a new reporting edge.
    #[error("AnswersToInactive: {identity} is {state}")]
    AnswersToInactive {
        /// The selected target.
        identity: String,
        /// Its recorded state.
        state: LifecycleState,
    },
    /// A reporting edge would make a cycle.
    #[error("AnswersToCycle: the reporting chain cycles through {chain:?}")]
    AnswersToCycle {
        /// The traversed identities including the repeated identity.
        chain: Vec<String>,
    },
    /// A reporting chain cannot reach an active person.
    #[error("NoAccountablePerson: the reporting chain has no active person: {chain:?}")]
    NoAccountablePerson {
        /// The traversed identities through the gap.
        chain: Vec<String>,
    },
    /// The login already belongs to a person; setup cannot replace that identity.
    #[error("AlreadyBootstrapped: this sign-in is already bound to person `{person}`")]
    AlreadyBootstrapped {
        /// The person already bound to the login.
        person: String,
    },
    /// The secure random source could not produce an identifier.
    #[error(
        "RandomSourceUnavailable: the secure random source could not produce an identifier: {reason}"
    )]
    RandomSourceUnavailable {
        /// What the random source reported.
        reason: String,
    },
    /// An identifier's text form is not the form this directory writes.
    #[error("IdentifierMalformed: `{text}` is not a {kind} identifier")]
    IdentifierMalformed {
        /// The kind of identifier that was expected.
        kind: &'static str,
        /// The text that was given.
        text: String,
    },
    /// An issuer and subject pair is not a login binding this directory records.
    #[error("BindingMalformed: {reason}")]
    BindingMalformed {
        /// The rule the pair broke.
        reason: &'static str,
    },
    /// A display profile is not one this directory records.
    #[error("ProfileInvalid: {reason}")]
    ProfileInvalid {
        /// The rule the profile broke.
        reason: &'static str,
    },
    /// A lifecycle transition that must name its reason named none.
    #[error("ReasonRequired: {transition} names the reason it was made")]
    ReasonRequired {
        /// The transition that needed a reason.
        transition: Transition,
    },
    /// A lifecycle transition outside the table was asked for.
    #[error("TransitionRefused: {transition} is not allowed from {from}")]
    TransitionRefused {
        /// The transition that was asked for.
        transition: Transition,
        /// The state it was asked for from.
        from: LifecycleState,
    },
    /// An event's change does not fit the identity it names.
    #[error("ChangeMismatch: {reason}")]
    ChangeMismatch {
        /// The rule the pairing broke.
        reason: &'static str,
    },
    /// An event's bytes do not decode to the envelope's shape.
    #[error("EventMalformed: {reason}")]
    EventMalformed {
        /// The part of the shape that was wrong.
        reason: &'static str,
    },
    /// An event decodes, but its bytes are not the canonical encoding of what it decodes to.
    #[error(
        "EventNotCanonical: the event's bytes are not the canonical encoding of what they decode to"
    )]
    EventNotCanonical,
    /// A leaf records a change to the install as a whole where a change to
    /// one identity was asked for.
    #[error(
        "InstallEntry: the leaf records a change to the install as a whole, not a change to an identity"
    )]
    InstallEntry,
    /// An event is larger than any event this directory writes.
    #[error("EventTooLarge: {len} bytes, over the limit of {limit}")]
    EventTooLarge {
        /// The event's length in bytes.
        len: usize,
        /// The largest event this directory reads.
        limit: usize,
    },
    /// An event names an envelope version this directory does not read.
    #[error("VersionUnsupported: event version {version} is not one this directory reads")]
    VersionUnsupported {
        /// The version the event names.
        version: u64,
    },
    /// An event names a service key other than the one it is verified against.
    #[error(
        "SignerMismatch: the event names a service key other than the one it is verified against"
    )]
    SignerMismatch,
    /// An event's signature does not verify.
    #[error("SignatureInvalid: the event's signature does not verify")]
    SignatureInvalid,
    /// The log could not be opened, read or written.
    #[error("LogUnavailable: {reason}")]
    LogUnavailable {
        /// What the log store reported.
        reason: String,
    },
    /// A leaf of the log is not an event signed by this directory's service key.
    #[error(
        "LeafNotAnEvent: leaf {index} is not an identity event this directory signed: {reason}"
    )]
    LeafNotAnEvent {
        /// The leaf's index.
        index: u64,
        /// Why it was refused.
        reason: String,
    },
    /// An append failed and whether its leaf reached the log is not yet known.
    #[error(
        "AppendUncertain: whether leaf {index} was committed is not yet known, and nothing is answered as current until it is"
    )]
    AppendUncertain {
        /// The index the uncertain leaf was written at.
        index: u64,
    },
    /// An append failed and its leaf is known not to be in the log.
    #[error("AppendRefused: the change was not recorded: {reason}")]
    AppendRefused {
        /// What the log store reported.
        reason: String,
    },
    /// A change names an identity the directory does not hold.
    #[error("IdentityUnknown: the directory holds no identity {identity}")]
    IdentityUnknown {
        /// The identity named.
        identity: String,
    },
    /// A registration names an identity the directory already holds.
    #[error("AlreadyRegistered: {identity} is already registered")]
    AlreadyRegistered {
        /// The identity named.
        identity: String,
    },
    /// A login is already bound to a person.
    #[error("BindingTaken: the login {subject} at {issuer} is already bound to {person}")]
    BindingTaken {
        /// The login's issuer.
        issuer: String,
        /// The login's subject.
        subject: String,
        /// The person it is bound to.
        person: String,
    },
    /// A transition names a from-state the identity is not in.
    #[error("StateMismatch: {identity} is {recorded}, not {from}")]
    StateMismatch {
        /// The identity named.
        identity: String,
        /// The state the directory records.
        recorded: LifecycleState,
        /// The state the transition named.
        from: LifecycleState,
    },
    /// An operation id was already used for a different change.
    #[error("OperationReused: {operation} already names a different change")]
    OperationReused {
        /// The operation id.
        operation: String,
    },
    /// A link-audit source operation id was already accepted.
    #[error("LinkSourceSeen: source operation {source_operation_id} was already accepted")]
    LinkSourceSeen {
        /// The source's operation id.
        source_operation_id: String,
    },
    /// A receipt does not match the signed event and log position it is checked against.
    #[error("ReceiptInvalid: {reason}")]
    ReceiptInvalid {
        /// The first thing that did not match.
        reason: &'static str,
    },
    /// The service key file could not be loaded.
    #[error("KeyUnavailable: {reason}")]
    KeyUnavailable {
        /// What loading the key reported.
        reason: String,
    },
}
