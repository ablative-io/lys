//! The grant event: one grant change and its audit record at once.
//!
//! An event names the caller's operation id, the identity that made the
//! request, when the service recorded it, and the change: a grant issued, a
//! grant revoked, or a grant's use observed where it was exercised. A grant change is recorded only as one of these events,
//! and both the grant book and the permission relationships are derived from
//! it, never written beside it.
//!
//! A signed event is a `COSE_Sign1` message (RFC 9052, tag 18) in the shape of
//! `lys/identity-event/v1`, with its own content type, [`GRANT_ENVELOPE`], in
//! the protected header; an event naming a service account is version 2 under
//! [`SERVICE_ACCOUNT_ENVELOPE`], and one naming a connector version 3 under
//! [`CONNECTOR_ENVELOPE`]; one issuing a held grant is version 4 under
//! [`HELD_MODE_ENVELOPE`], one issuing a grant that names a role version
//! 5 under [`ROLE_ENVELOPE`], and one naming a machine version 6 under
//! [`MACHINE_ENVELOPE`] (ACCESS-005 R1), and one naming an organisation,
//! as caller or issuer, carrying a grant marked with one, or stamping one
//! onto a grant, version 7 under [`ORGANISATION_ENVELOPE`] (DIRECTORY-109
//! R3), and a use naming the act it exercised version 8 under
//! [`EXERCISED_ENVELOPE`] (AGENTS-019 R5), so no earlier event's bytes
//! change. The body is a canonical CBOR map: `1` version, `2`
//! operation id, `3` caller, `4` recorded-at, `5` change kind (`1` issue, `2`
//! revoke, `3` use, `4` stamp) and `6` the change: the grant's own map; for a revocation
//! `1` grant id and `2` reason; for a use `1` grant id and `2` route (`1`
//! browser, `2` api, `3` tool), and, when it names what it exercised, `3`
//! the action and `4` the resource; for a stamp `1` grant id and `2` the
//! organisation (16 bytes). A use's caller is the holder who exercised the
//! grant and its recorded-at is when. A stamp's caller is the organisation
//! itself. Reading is strict: a message naming another envelope is refused
//! `EnvelopeMismatch`, and bytes that are not the exact canonical message are
//! refused.

use ciborium::Value;

use super::admission::Route;
use super::codec::{
    GRANT_ENVELOPE, as_id, as_text, as_uint, fields, read_grant, read_identity, write_grant,
    write_identity,
};
use super::error::GrantError;
use super::types::{Grant, GrantId};
use crate::encoding::{bytes, map, text, uint};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// The grant event version this crate writes and reads.
pub const GRANT_EVENT_VERSION: u64 = 1;
/// Separate signed envelope for service-account grant events.
pub const SERVICE_ACCOUNT_ENVELOPE: &str = "application/vnd.lys.grant-event.v2+cbor";
/// Separate signed envelope for grant events that name a connector.
pub const CONNECTOR_ENVELOPE: &str = "application/vnd.lys.grant-event.v3+cbor";
/// Separate signed envelope for events issuing a grant held by draft or by two.
pub const HELD_MODE_ENVELOPE: &str = "application/vnd.lys.grant-event.v4+cbor";
/// Separate signed envelope for events issuing a grant that names a role
/// (ACCESS-004 R1).
pub const ROLE_ENVELOPE: &str = "application/vnd.lys.grant-event.v5+cbor";
/// Separate signed envelope for grant events that name a machine, as
/// caller, holder, issuer or a kind a grant may be passed on to.
pub const MACHINE_ENVELOPE: &str = "application/vnd.lys.grant-event.v6+cbor";
/// Separate signed envelope for grant events that name an organisation:
/// as caller or issuer, in a grant's key 16, or as a stamp (DIRECTORY-109 R3).
pub const ORGANISATION_ENVELOPE: &str = "application/vnd.lys.grant-event.v7+cbor";

const COSE_SIGN1_TAG: u64 = 18;
const SIGNATURE_LEN: usize = 64;
const KEY_LEN: usize = 32;

/// The first grant event version able to name `kind`, and the content type
/// that signs it.
fn kind_envelope(kind: super::types::RecipientKind) -> (u64, &'static str) {
    use super::types::RecipientKind;
    match kind {
        RecipientKind::Person | RecipientKind::Agent => (GRANT_EVENT_VERSION, GRANT_ENVELOPE),
        RecipientKind::ServiceAccount => (2, SERVICE_ACCOUNT_ENVELOPE),
        RecipientKind::Connector => (3, CONNECTOR_ENVELOPE),
        RecipientKind::Machine => (6, MACHINE_ENVELOPE),
        RecipientKind::Organisation => (7, ORGANISATION_ENVELOPE),
    }
}

/// Whether the grants read events of `version`.
fn version_read(version: u64) -> bool {
    matches!(version, GRANT_EVENT_VERSION | 2 | 3 | 4 | 5 | 6 | 7 | 8)
}

/// One change to the grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantChange {
    /// A grant is issued, as a root or derived from another grant.
    Issue(Box<Grant>),
    /// A grant is revoked, and with it everything derived from it.
    Revoke {
        /// The grant revoked.
        grant: GrantId,
        /// Why it was revoked.
        reason: String,
    },
    /// A grant was exercised by its holder, as the check that permitted it observed.
    Use {
        /// The grant exercised.
        grant: GrantId,
        /// How the exercise arrived.
        route: Route,
    },
    /// A grant written before organisations is marked with the one it
    /// belongs to, once, at the first boot under DIRECTORY-109 R2. The
    /// caller is the organisation itself.
    Stamp {
        /// The grant stamped.
        grant: GrantId,
        /// The organisation it belongs to.
        organisation: crate::OrganisationId,
    },
}

/// A grant change, before it is signed or after it is verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantEvent {
    operation: OperationId,
    caller: IdentityId,
    recorded_at: u64,
    change: GrantChange,
    exercised: Option<Exercised>,
}

impl GrantEvent {
    /// Version two adds service-account principals. Events using only the
    /// original principal and recipient kinds retain their original bytes.
    /// Version three adds the connector, four a held mode, five a grant
    /// naming a role, six the machine, seven the organisation and eight a
    /// use naming what it exercised. Each event takes the version of the
    /// newest kind, mode or mark it names.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.newest().0
    }

    fn content_type(&self) -> &'static str {
        self.newest().1
    }

    /// The version and content type of the newest kind the event names.
    fn newest(&self) -> (u64, &'static str) {
        use super::types::{PassOn, RecipientKind};
        let mut newest = kind_envelope(RecipientKind::of(self.caller));
        if let GrantChange::Issue(grant) = &self.change {
            let mut kinds = vec![RecipientKind::of(grant.holder())];
            if let PassOn::To { recipients, .. } = grant.pass_on() {
                kinds.extend(recipients.iter().copied());
            }
            let mut named: Vec<_> = kinds.into_iter().map(kind_envelope).collect();
            if grant.mode().is_held() {
                named.push((4, HELD_MODE_ENVELOPE));
            }
            if grant.names_role() {
                named.push((5, ROLE_ENVELOPE));
            }
            if grant.organisation().is_some() {
                named.push((7, ORGANISATION_ENVELOPE));
            }
            for named in named {
                if named.0 > newest.0 {
                    newest = named;
                }
            }
        }
        if let GrantChange::Stamp { .. } = &self.change {
            newest = (7, ORGANISATION_ENVELOPE);
        }
        if self.exercised.is_some() {
            newest = (8, EXERCISED_ENVELOPE);
        }
        newest
    }

    /// The event recording `change`, requested by `caller` under `operation`.
    ///
    /// An issued grant names this event's operation as the one that
    /// authorised it, and its issuer is the caller; a revocation names a
    /// reason that is not blank.
    ///
    /// # Errors
    /// - `GrantError::EventMismatch` if an issued grant names another operation or issuer, or a revocation has a blank reason.
    pub fn new(
        operation: OperationId,
        caller: IdentityId,
        recorded_at: u64,
        change: GrantChange,
    ) -> Result<Self, GrantError> {
        match &change {
            GrantChange::Issue(grant) => {
                if grant.parts().operation != operation {
                    return Err(GrantError::EventMismatch {
                        reason: "an issued grant names the operation of the event that issues it",
                    });
                }
                if grant.parts().issuer != caller {
                    return Err(GrantError::EventMismatch {
                        reason: "an issued grant names the caller as its issuer",
                    });
                }
            }
            GrantChange::Revoke { reason, .. } => {
                if reason.trim().is_empty() {
                    return Err(GrantError::EventMismatch {
                        reason: "a revocation names its reason",
                    });
                }
            }
            GrantChange::Use { .. } => {}
            GrantChange::Stamp { organisation, .. } => {
                if caller != IdentityId::Organisation(*organisation) {
                    return Err(GrantError::EventMismatch {
                        reason: "a stamp is written by the organisation it names",
                    });
                }
            }
        }
        Ok(Self {
            operation,
            caller,
            recorded_at,
            change,
            exercised: None,
        })
    }

    /// This use, naming the act it exercised and the resource it was asked
    /// on (AGENTS-019 R5).
    ///
    /// # Errors
    /// - `GrantError::EventMismatch` if the event is not a use.
    pub fn with_exercised(mut self, exercised: Exercised) -> Result<Self, GrantError> {
        if !matches!(self.change, GrantChange::Use { .. }) {
            return Err(GrantError::EventMismatch {
                reason: "only a use names what it exercised",
            });
        }
        self.exercised = Some(exercised);
        Ok(self)
    }

    /// What a use exercised, when it names it: a use written before
    /// AGENTS-019 R5 names only its grant and route.
    #[must_use]
    pub fn exercised(&self) -> Option<&Exercised> {
        self.exercised.as_ref()
    }

    /// The operation id the caller gave the change.
    #[must_use]
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The identity that made the request.
    #[must_use]
    pub fn caller(&self) -> IdentityId {
        self.caller
    }

    /// When the service recorded the change, in seconds since the Unix epoch.
    #[must_use]
    pub fn recorded_at(&self) -> u64 {
        self.recorded_at
    }

    /// The change.
    #[must_use]
    pub fn change(&self) -> &GrantChange {
        &self.change
    }

    /// The grant the change is about.
    #[must_use]
    pub fn grant(&self) -> GrantId {
        match &self.change {
            GrantChange::Issue(grant) => grant.id(),
            GrantChange::Revoke { grant, .. }
            | GrantChange::Use { grant, .. }
            | GrantChange::Stamp { grant, .. } => *grant,
        }
    }
}

/// The change kind code of an issue.
pub(crate) const ISSUE: u64 = 1;
/// The change kind code of a revocation.
pub(crate) const REVOKE: u64 = 2;
/// The change kind code of a use.
pub(crate) const USE: u64 = 3;
/// The change kind code of a stamp.
pub(crate) const STAMP: u64 = 4;

/// The change kind's wire code: `1` issue, `2` revoke, `3` use, `4` stamp.
#[must_use]
pub fn change_kind(change: &GrantChange) -> u64 {
    match change {
        GrantChange::Issue(_) => ISSUE,
        GrantChange::Revoke { .. } => REVOKE,
        GrantChange::Use { .. } => USE,
        GrantChange::Stamp { .. } => STAMP,
    }
}

/// The canonical body bytes of `event`.
#[must_use]
pub fn encode_event_body(event: &GrantEvent) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 6);
    uint(&mut out, 1);
    uint(&mut out, event.version());
    uint(&mut out, 2);
    bytes(&mut out, event.operation.as_bytes());
    uint(&mut out, 3);
    write_identity(&mut out, event.caller);
    uint(&mut out, 4);
    uint(&mut out, event.recorded_at);
    uint(&mut out, 5);
    uint(&mut out, change_kind(&event.change));
    uint(&mut out, 6);
    match &event.change {
        GrantChange::Issue(grant) => write_grant(&mut out, grant),
        GrantChange::Revoke { grant, reason } => {
            map(&mut out, 2);
            uint(&mut out, 1);
            bytes(&mut out, grant.as_bytes());
            uint(&mut out, 2);
            text(&mut out, reason);
        }
        GrantChange::Use { grant, route } => {
            map(&mut out, if event.exercised.is_some() { 4 } else { 2 });
            uint(&mut out, 1);
            bytes(&mut out, grant.as_bytes());
            uint(&mut out, 2);
            uint(&mut out, route_code(*route));
            if let Some(exercised) = &event.exercised {
                exercise::write(&mut out, exercised);
            }
        }
        GrantChange::Stamp {
            grant,
            organisation,
        } => {
            map(&mut out, 2);
            uint(&mut out, 1);
            bytes(&mut out, grant.as_bytes());
            uint(&mut out, 2);
            bytes(&mut out, organisation.as_bytes());
        }
    }
    out
}

/// A route's wire code.
pub(crate) fn route_code(route: Route) -> u64 {
    match route {
        Route::Browser => 1,
        Route::Api => 2,
        Route::Tool => 3,
    }
}

/// The route a wire code names.
pub(crate) fn route_from(code: u64) -> Option<Route> {
    match code {
        1 => Some(Route::Browser),
        2 => Some(Route::Api),
        3 => Some(Route::Tool),
        _ => None,
    }
}

fn malformed(reason: &'static str) -> GrantError {
    GrantError::EventMalformed { reason }
}

fn cbor(encoded: &[u8], reason: &'static str) -> Result<Value, GrantError> {
    ciborium::from_reader(encoded)
        .ok()
        .ok_or_else(|| malformed(reason))
}

/// The event a body's bytes name, refused unless the bytes are its canonical encoding.
///
/// # Errors
/// - `GrantError::EventMalformed` if the event CBOR or wire codes have the wrong shape.
/// - `GrantError::VersionUnsupported` if the event version is unsupported.
/// - `GrantError::GrantMalformed` if a grant member, identity or optional mark has the wrong shape.
/// - `GrantError::MemberUnknown` if an encoded grant names an unsupported member.
/// - `GrantError::MemberMissing` if an encoded grant omits a required member.
/// - `GrantError::RecipientKindUnknown` if a pass-on recipient code is unsupported.
/// - `GrantError::TokenInvalid` if an action, relation or resource token is invalid.
/// - `GrantError::AuthorityAbsent` if exercise or pass-on actions or recipients are empty.
/// - `GrantError::PassOnOutside` if pass-on authority exceeds exercise authority.
/// - `GrantError::WindowInvalid` if the grant ends at or before its start.
/// - `GrantError::ModelInvalid` if the model version is zero.
/// - `GrantError::LineageMalformed` if the source or root holder is invalid.
/// - `GrantError::UseOnly` if a one-time grant allows pass-on.
/// - `GrantError::EventMismatch` if the issued grant disagrees with its event or a revocation reason is blank.
/// - `GrantError::EventNotCanonical` if re-encoding does not reproduce the input bytes.
pub fn decode_event_body(body: &[u8]) -> Result<GrantEvent, GrantError> {
    const SHAPE: &str = "a grant event body is a map of keys 1 to 6";
    let value = cbor(body, SHAPE)?;
    if let Value::Map(pairs) = &value
        && let Some((_, version)) = pairs.first()
    {
        let version = as_uint(version, SHAPE)?;
        if !version_read(version) {
            return Err(GrantError::VersionUnsupported { version });
        }
    }
    let [_, operation, caller, recorded_at, kind, change] = fields::<6>(value, SHAPE)?;
    let mut exercised = None;
    let change = match as_uint(&kind, "a change kind is a code")? {
        ISSUE => GrantChange::Issue(Box::new(read_grant(change)?)),
        REVOKE => {
            let [grant, reason] = fields::<2>(change, "a revocation is a map of keys 1 and 2")?;
            GrantChange::Revoke {
                grant: GrantId::from_bytes(as_id(grant, "a grant id is 16 bytes")?),
                reason: as_text(reason, "a reason is text")?,
            }
        }
        USE => {
            let named = matches!(&change, Value::Map(pairs) if pairs.len() == 4);
            let [grant, route] = if named {
                let [grant, route, action, resource] =
                    fields::<4>(change, "a use naming what it exercised is a map of keys 1 to 4")?;
                exercised = Some(exercise::read(action, resource)?);
                [grant, route]
            } else {
                fields::<2>(change, "a use is a map of keys 1 and 2")?
            };
            GrantChange::Use {
                grant: GrantId::from_bytes(as_id(grant, "a grant id is 16 bytes")?),
                route: route_from(as_uint(&route, "a route is a code")?)
                    .ok_or_else(|| malformed("a route code is 1 to 3"))?,
            }
        }
        STAMP => {
            let [grant, organisation] = fields::<2>(change, "a stamp is a map of keys 1 and 2")?;
            GrantChange::Stamp {
                grant: GrantId::from_bytes(as_id(grant, "a grant id is 16 bytes")?),
                organisation: crate::OrganisationId::from_bytes(as_id(
                    organisation,
                    "an organisation id is 16 bytes",
                )?),
            }
        }
        _ => return Err(malformed("a grant change kind is 1 to 4")),
    };
    let mut event = GrantEvent::new(
        OperationId::from_bytes(as_id(operation, "an operation id is 16 bytes")?),
        read_identity(caller)?,
        as_uint(&recorded_at, "a recorded time is seconds")?,
        change,
    )?;
    if let Some(exercised) = exercised {
        event = event.with_exercised(exercised)?;
    }
    if encode_event_body(&event) != body {
        return Err(GrantError::EventNotCanonical);
    }
    Ok(event)
}

#[path = "events_exercised.rs"]
mod exercise;
#[path = "events_signing.rs"]
mod signing;

pub use exercise::{EXERCISED_ENVELOPE, Exercised};

pub use signing::{SignedGrantEvent, sign_grant_event, verify_grant_event};
