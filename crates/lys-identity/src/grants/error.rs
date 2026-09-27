//! [`GrantError`], the grant contract's error type.
//!
//! Each variant names the rule a grant, a request or a stored grant event
//! broke, in its message's first word, so a caller can say which rule and act
//! on it. No variant carries a default: a refusal is never turned into a
//! permission by leaving something out.

use crate::error::IdentityError;

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
        "TokenInvalid: the {kind} `{text}` is not a token of 1 to 128 bytes of a-z, 0-9, `_`, `-` and `.`"
    )]
    TokenInvalid {
        /// What the token names.
        kind: &'static str,
        /// The text that was given.
        text: String,
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
    /// A directory refusal met while judging a grant.
    #[error(transparent)]
    Identity(#[from] IdentityError),
}
