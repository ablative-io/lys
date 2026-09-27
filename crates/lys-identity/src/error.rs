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
}
