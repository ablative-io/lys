//! [`RoleError`], the role records' error type.
//!
//! Each variant names the rule a role act, a record or a role event broke,
//! in its message's first word, so a caller can say which rule and act on
//! it. No variant carries a default: a refusal is never turned into a
//! permission by leaving something out, and every refusal commits nothing.

use std::fmt;

use super::check::Check;
use crate::error::IdentityError;
use crate::grants::{GrantError, Relation, Resource};

/// Why a template's grant could not be made for the acting person, as
/// conformance row 2.4 names the reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateReason {
    /// The grant the actor holds for it is use-only.
    UseOnly,
    /// It asks for more than any grant the actor holds lets it pass on.
    AboveHeld,
    /// The grant the actor holds for it was lent to the actor, use-only.
    LentToYou,
    /// It would pass on a sign-in identity. No grant a template copies is a
    /// sign-in identity, so the role acts never answer this reason; it is
    /// named so the four reasons of row 2.4 are one closed set.
    SignInIdentity,
}

impl TemplateReason {
    /// The reason's name, as row 2.4 spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UseOnly => "use_only",
            Self::AboveHeld => "above_held",
            Self::LentToYou => "lent_to_you",
            Self::SignInIdentity => "sign_in_identity",
        }
    }
}

impl fmt::Display for TemplateReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One template whose grant the actor could not make, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateRefusal {
    /// The template's relation.
    pub relation: Relation,
    /// The template's resource.
    pub resource: Resource,
    /// The reason, as row 2.4 names it.
    pub reason: TemplateReason,
    /// The grant admission's own refusal, when one was made.
    pub detail: String,
}

impl fmt::Display for TemplateRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} on {}: {} ({})",
            self.relation, self.resource, self.reason, self.detail
        )
    }
}

fn listed(list: &[TemplateRefusal]) -> String {
    list.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Errors returned by the role records, their events and the role acts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleError {
    /// A role or holding id's text form is not the form this module writes.
    #[error("IdMalformed: `{text}` is not a {kind} identifier")]
    IdMalformed {
        /// What kind of id was expected.
        kind: &'static str,
        /// The text read.
        text: String,
    },
    /// A version skips a number.
    #[error("version_gap: role {role} is at version {last}, so version {version} is not its next")]
    VersionGap {
        /// The role.
        role: String,
        /// Its last version, 0 before its first.
        last: u64,
        /// The version the event makes.
        version: u64,
    },
    /// An event would replace a committed version.
    #[error("version_immutable: version {version} of role {role} is committed and never changes")]
    VersionImmutable {
        /// The role.
        role: String,
        /// The committed version.
        version: u64,
    },
    /// A holding record carries no move policy.
    #[error("missing_policy: a holding names its move policy; absence is never a default")]
    MissingPolicy,
    /// A move policy is neither of the two this module defines.
    #[error("unknown_policy: `{text}` is not move_at_next_renewal or deliberate_only")]
    UnknownPolicy {
        /// The text read.
        text: String,
    },
    /// A capacity is neither of the two this module defines.
    #[error("unknown_capacity: `{text}` is not responsible_person or project_owner")]
    UnknownCapacity {
        /// The text read.
        text: String,
    },
    /// A timing is neither of the two this module defines.
    #[error("unknown_timing: `{text}` is not next_start or now")]
    UnknownTiming {
        /// The text read.
        text: String,
    },
    /// A holding sits in a project other than its role's.
    #[error(
        "project_mismatch: {what} names project {named}, and role {role} is defined in {project}"
    )]
    ProjectMismatch {
        /// What named the project.
        what: &'static str,
        /// The project it names.
        named: String,
        /// The role.
        role: String,
        /// The project the role is defined in.
        project: String,
    },
    /// A template carries a holder.
    #[error("template_has_holder: a grant template has no holder")]
    TemplateHasHolder,
    /// A template carries a source.
    #[error("template_has_source: a grant template has no source")]
    TemplateHasSource,
    /// A template's window has an end of its own.
    #[error(
        "template_has_end: a template's window has no end of its own; a holding's end date is written on its grants"
    )]
    TemplateHasEnd,
    /// A template or version breaks the template rules.
    #[error("template_invalid: {reason}")]
    TemplateInvalid {
        /// The rule broken.
        reason: &'static str,
    },
    /// A title is empty or too long.
    #[error("title_invalid: a role's title is 1 to 100 characters, not blank")]
    TitleInvalid,
    /// The actor holds neither capacity the check admits.
    #[error("not_permitted: {actor} may not {check}")]
    NotPermitted {
        /// The actor.
        actor: String,
        /// The check refused.
        check: Check,
    },
    /// The seam could not answer, so nothing is permitted.
    #[error("check_unavailable: the {check} check could not be answered: {reason}")]
    CheckUnavailable {
        /// The check that could not be answered.
        check: Check,
        /// Why.
        reason: String,
    },
    /// The grant admission refused one or more templates' grants.
    #[error("templates_refused: {}", listed(.refusals))]
    TemplatesRefused {
        /// Each template refused, with its reason.
        refusals: Vec<TemplateRefusal>,
    },
    /// A move names a version that is not newer than the held one.
    #[error("not_newer: version {target} is not newer than the held version {held}")]
    NotNewer {
        /// The held version.
        held: u64,
        /// The version asked for.
        target: u64,
    },
    /// A holding has no end date, so it has no renewal.
    #[error("no_end_date: holding {holding} has no end date")]
    NoEndDate {
        /// The holding.
        holding: String,
    },
    /// A holding's end date has passed.
    #[error("holding_lapsed: holding {holding} lapsed at {ended_at}")]
    HoldingLapsed {
        /// The holding.
        holding: String,
        /// Its end date.
        ended_at: u64,
    },
    /// The renewing actor holds neither capacity a renewal admits.
    #[error("ROLE_RENEW_REFUSED: {actor} may not renew holding {holding}")]
    RenewRefused {
        /// The actor.
        actor: String,
        /// The holding.
        holding: String,
    },
    /// A move now asked for by anyone but the configured administrator.
    #[error(
        "now_not_operator: only the configured administrator takes a move now; {actor} may take it at the next start"
    )]
    NowNotOperator {
        /// The actor.
        actor: String,
    },
    /// A move now, whose session stop is not yet on the main branch.
    #[error(
        "now_unavailable: a move now stops every open session first, and the session stop it asks for is not built"
    )]
    NowUnavailable,
    /// A confirmed preview no longer matches the holding.
    #[error("preview_stale: the preview confirmed is not the move the holding would take now")]
    PreviewStale,
    /// A new end date does not lie after the time it is given at.
    #[error("end_date_invalid: {reason}")]
    EndDateInvalid {
        /// The rule broken.
        reason: &'static str,
    },
    /// A resource named as a project is not one.
    #[error("not_a_project: {resource} is not a project")]
    NotAProject {
        /// The resource named.
        resource: String,
    },
    /// No role has this id.
    #[error("role_unknown: no role {role}")]
    RoleUnknown {
        /// The role named.
        role: String,
    },
    /// No holding has this id.
    #[error("holding_unknown: no holding {holding}")]
    HoldingUnknown {
        /// The holding named.
        holding: String,
    },
    /// A role has no version of this number.
    #[error("version_unknown: role {role} has no version {version}")]
    VersionUnknown {
        /// The role.
        role: String,
        /// The version named.
        version: u64,
    },
    /// A holding id is already taken.
    #[error("holding_exists: holding {holding} is already granted")]
    HoldingExists {
        /// The holding.
        holding: String,
    },
    /// A holding's grants do not match its version's templates.
    #[error("grants_mismatch: {reason}")]
    GrantsMismatch {
        /// The rule broken.
        reason: &'static str,
    },
    /// A role act was asked for by an identity that is not a person.
    #[error("actor_not_person: {actor} is not a person; only a person acts on a role")]
    ActorNotPerson {
        /// The actor.
        actor: String,
    },
    /// An operation id already names a committed role event.
    #[error("operation_reused: operation {operation} already names a committed role event")]
    OperationReused {
        /// The operation.
        operation: String,
    },
    /// A role event or record is not the exact shape this module writes.
    #[error("event_malformed: {reason}")]
    EventMalformed {
        /// The rule broken.
        reason: &'static str,
    },
    /// Bytes decode, but are not the one canonical encoding of what they carry.
    #[error("event_not_canonical: the bytes are not the canonical encoding")]
    EventNotCanonical,
    /// A role event is larger than any this module reads.
    #[error("event_too_large: {len} bytes exceeds the {limit}-byte limit")]
    EventTooLarge {
        /// The event's length.
        len: usize,
        /// The limit.
        limit: usize,
    },
    /// A message names a key other than the service key checked against.
    #[error("signer_mismatch: the message names another signing key")]
    SignerMismatch,
    /// A message's signature does not verify.
    #[error("signature_invalid: the signature does not verify")]
    SignatureInvalid,
    /// The grants refused a grant change a role act made.
    #[error(transparent)]
    Grant(#[from] GrantError),
    /// The directory refused.
    #[error(transparent)]
    Identity(#[from] IdentityError),
}
