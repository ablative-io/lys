//! The role event: one role act and its audit record at once.
//!
//! An event names the caller's operation id, the authenticated actor, the
//! capacity it acted in, when the service recorded it, and the change, one
//! of seven kinds: `1` role version made, `2` role title changed, `3` role
//! default policy changed, `4` holding granted, `5` holding moved, `6`
//! holding renewed and `7` holding policy changed. The exact encoding is
//! drafted in docs/design/identity/IDENTITY-EVENTS.md, in the section on
//! role events, for the envelope's joint review.
//!
//! A signed event is a `COSE_Sign1` message (RFC 9052, tag 18) in the shape
//! of `lys/identity-event/v1`, with its own content type, [`ROLE_ENVELOPE`],
//! so a role event is never read as an identity or grant event nor they as
//! it. Every closed value (capacity, policy, timing) is written as its name,
//! and a name outside the set is refused by name. Reading is strict: bytes
//! that are not the exact canonical encoding are refused. No kind is durably
//! signed before the envelope review accepts it: the role events are held in
//! memory by [`crate::roles::holding::Roles`] until then.

use ciborium::Value;
use lys_core::Ed25519Identity;

use super::error::RoleError;
use super::types::{Capacity, Holding, HoldingId, MovePolicy, RoleId, Template, Timing};
use crate::encoding::{
    MAJOR_ARRAY, MAJOR_NEGATIVE, MAJOR_TAG, bytes, head, map, payload_commitment, text, uint,
};
use crate::grants::codec::write_identity;
use crate::grants::{GrantId, Resource};
use crate::id::{IdentityId, PersonId};
use crate::operation::OperationId;

#[path = "codec.rs"]
mod codec;

pub use codec::{decode_holding, decode_template, encode_holding, encode_template};

/// The content type a signed role event's protected header names.
pub const ROLE_ENVELOPE: &str = "application/vnd.lys.role-event.v1+cbor";

/// The role event version this crate writes and reads.
pub const ROLE_EVENT_VERSION: u64 = 1;

/// The largest role event written or read, in bytes.
pub const MAX_ROLE_EVENT_BYTES: usize = 64 * 1024;

const COSE_SIGN1_TAG: u64 = 18;
const SIGNATURE_LEN: usize = 64;
const KEY_LEN: usize = 32;

/// What version 1 of a role opens it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The role's title.
    pub title: String,
    /// The role's default move policy.
    pub default_policy: MovePolicy,
}

/// One role act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleChange {
    /// A version of a role is made; version 1 makes the role.
    VersionMade {
        /// The role.
        role: RoleId,
        /// The project it is defined in.
        project: Resource,
        /// The version's number.
        version: u64,
        /// The version's grant templates.
        templates: Vec<Template>,
        /// The title and default version 1 opens the role with; none after it.
        opening: Option<Opening>,
    },
    /// A role's title is changed; no version is made.
    TitleChanged {
        /// The role.
        role: RoleId,
        /// The title before.
        before: String,
        /// The title after.
        after: String,
    },
    /// A role's default move policy is changed; no holding changes.
    DefaultPolicyChanged {
        /// The role.
        role: RoleId,
        /// The default before.
        before: MovePolicy,
        /// The default after.
        after: MovePolicy,
    },
    /// A holding is granted.
    HoldingGranted(Box<Holding>),
    /// A holding is moved to a newer version.
    HoldingMoved {
        /// The holding.
        holding: HoldingId,
        /// The version it held.
        from: u64,
        /// The version it moves to.
        to: u64,
        /// When the move takes effect.
        timing: Timing,
        /// The grants the move made, in the order of the new templates they copy.
        added: Vec<GrantId>,
        /// The grants the move withdrew.
        removed: Vec<GrantId>,
    },
    /// A holding is renewed as new grants with a new end date.
    HoldingRenewed {
        /// The holding.
        holding: HoldingId,
        /// The version it lands on.
        version: u64,
        /// Its new end date.
        ends_at: u64,
        /// The grants made, one for each template of that version, in order.
        grants: Vec<GrantId>,
        /// The grants they replace, withdrawn.
        replaced: Vec<GrantId>,
    },
    /// One holding's move policy is changed.
    HoldingPolicyChanged {
        /// The holding.
        holding: HoldingId,
        /// The policy before.
        before: MovePolicy,
        /// The policy after.
        after: MovePolicy,
    },
}

impl RoleChange {
    /// The change kind's wire code.
    pub fn kind(&self) -> u64 {
        match self {
            Self::VersionMade { .. } => 1,
            Self::TitleChanged { .. } => 2,
            Self::DefaultPolicyChanged { .. } => 3,
            Self::HoldingGranted(_) => 4,
            Self::HoldingMoved { .. } => 5,
            Self::HoldingRenewed { .. } => 6,
            Self::HoldingPolicyChanged { .. } => 7,
        }
    }
}

/// A role act, before it is signed or after it is verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleEvent {
    operation: OperationId,
    actor: PersonId,
    capacity: Capacity,
    recorded_at: u64,
    change: RoleChange,
}

impl RoleEvent {
    /// The event recording `change`, made by the authenticated `actor` in
    /// `capacity` under `operation`. Only a person acts on a role.
    pub fn new(
        operation: OperationId,
        actor: IdentityId,
        capacity: Capacity,
        recorded_at: u64,
        change: RoleChange,
    ) -> Result<Self, RoleError> {
        let IdentityId::Person(actor) = actor else {
            return Err(RoleError::ActorNotPerson {
                actor: actor.to_string(),
            });
        };
        Ok(Self {
            operation,
            actor,
            capacity,
            recorded_at,
            change,
        })
    }

    /// The operation id.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The authenticated actor.
    pub fn actor(&self) -> PersonId {
        self.actor
    }

    /// The capacity the actor acted in.
    pub fn capacity(&self) -> Capacity {
        self.capacity
    }

    /// When the service recorded it.
    pub fn recorded_at(&self) -> u64 {
        self.recorded_at
    }

    /// The change.
    pub fn change(&self) -> &RoleChange {
        &self.change
    }
}

pub(super) fn write_resource(out: &mut Vec<u8>, resource: &Resource) {
    map(out, 2);
    uint(out, 1);
    text(out, resource.kind());
    uint(out, 2);
    text(out, resource.id());
}

fn write_change(out: &mut Vec<u8>, change: &RoleChange) {
    match change {
        RoleChange::VersionMade {
            role,
            project,
            version,
            templates,
            opening,
        } => {
            map(out, if opening.is_some() { 6 } else { 4 });
            uint(out, 1);
            bytes(out, role.as_bytes());
            uint(out, 2);
            write_resource(out, project);
            uint(out, 3);
            uint(out, *version);
            uint(out, 4);
            head(out, MAJOR_ARRAY, templates.len() as u64);
            for template in templates {
                out.extend_from_slice(&encode_template(template));
            }
            if let Some(opening) = opening {
                uint(out, 5);
                text(out, &opening.title);
                uint(out, 6);
                text(out, opening.default_policy.as_str());
            }
        }
        RoleChange::TitleChanged {
            role,
            before,
            after,
        } => {
            map(out, 3);
            uint(out, 1);
            bytes(out, role.as_bytes());
            uint(out, 2);
            text(out, before);
            uint(out, 3);
            text(out, after);
        }
        RoleChange::DefaultPolicyChanged {
            role,
            before,
            after,
        } => {
            map(out, 3);
            uint(out, 1);
            bytes(out, role.as_bytes());
            uint(out, 2);
            text(out, before.as_str());
            uint(out, 3);
            text(out, after.as_str());
        }
        RoleChange::HoldingGranted(holding) => out.extend_from_slice(&encode_holding(holding)),
        RoleChange::HoldingMoved {
            holding,
            from,
            to,
            timing,
            added,
            removed,
        } => {
            map(out, 6);
            uint(out, 1);
            bytes(out, holding.as_bytes());
            uint(out, 2);
            uint(out, *from);
            uint(out, 3);
            uint(out, *to);
            uint(out, 4);
            text(out, timing.as_str());
            uint(out, 5);
            codec::write_ids(out, added);
            uint(out, 6);
            codec::write_ids(out, removed);
        }
        RoleChange::HoldingRenewed {
            holding,
            version,
            ends_at,
            grants,
            replaced,
        } => {
            map(out, 5);
            uint(out, 1);
            bytes(out, holding.as_bytes());
            uint(out, 2);
            uint(out, *version);
            uint(out, 3);
            uint(out, *ends_at);
            uint(out, 4);
            codec::write_ids(out, grants);
            uint(out, 5);
            codec::write_ids(out, replaced);
        }
        RoleChange::HoldingPolicyChanged {
            holding,
            before,
            after,
        } => {
            map(out, 3);
            uint(out, 1);
            bytes(out, holding.as_bytes());
            uint(out, 2);
            text(out, before.as_str());
            uint(out, 3);
            text(out, after.as_str());
        }
    }
}

/// The canonical body of `event`.
pub fn encode_event_body(event: &RoleEvent) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 7);
    uint(&mut out, 1);
    uint(&mut out, ROLE_EVENT_VERSION);
    uint(&mut out, 2);
    bytes(&mut out, event.operation.as_bytes());
    uint(&mut out, 3);
    write_identity(&mut out, IdentityId::Person(event.actor));
    uint(&mut out, 4);
    text(&mut out, event.capacity.as_str());
    uint(&mut out, 5);
    uint(&mut out, event.recorded_at);
    uint(&mut out, 6);
    uint(&mut out, event.change.kind());
    uint(&mut out, 7);
    write_change(&mut out, &event.change);
    out
}

/// The event `body` carries, refused unless it is the exact canonical body.
pub fn decode_event_body(body: &[u8]) -> Result<RoleEvent, RoleError> {
    let event = codec::read_body(codec::parse(body, "a role event body is one CBOR map")?)?;
    if encode_event_body(&event) != body {
        return Err(RoleError::EventNotCanonical);
    }
    Ok(event)
}

/// A role event with the exact bytes that were signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRoleEvent {
    bytes: Vec<u8>,
    event: RoleEvent,
    commitment: [u8; 32],
}

impl SignedRoleEvent {
    /// The whole `COSE_Sign1` message.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The event the message carries.
    pub fn event(&self) -> &RoleEvent {
        &self.event
    }

    /// SHA-256 over the body bytes: the payload commitment.
    pub fn payload_commitment(&self) -> [u8; 32] {
        self.commitment
    }
}

fn protected_header(kid: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 3);
    uint(&mut out, 1);
    head(&mut out, MAJOR_NEGATIVE, 7);
    uint(&mut out, 3);
    text(&mut out, ROLE_ENVELOPE);
    uint(&mut out, 4);
    bytes(&mut out, kid);
    out
}

fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_ARRAY, 4);
    text(&mut out, "Signature1");
    bytes(&mut out, protected);
    bytes(&mut out, &[]);
    bytes(&mut out, payload);
    out
}

fn cose_sign1(protected: &[u8], payload: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    head(&mut out, MAJOR_ARRAY, 4);
    bytes(&mut out, protected);
    map(&mut out, 0);
    bytes(&mut out, payload);
    bytes(&mut out, signature);
    out
}

/// Sign `event` with the service's key.
pub fn sign_role_event(
    event: RoleEvent,
    service_key: &Ed25519Identity,
) -> Result<SignedRoleEvent, RoleError> {
    let body = encode_event_body(&event);
    let protected = protected_header(&service_key.public_key_bytes());
    let signature = service_key.sign(&sig_structure(&protected, &body));
    let message = cose_sign1(&protected, &body, &signature);
    if message.len() > MAX_ROLE_EVENT_BYTES {
        return Err(RoleError::EventTooLarge {
            len: message.len(),
            limit: MAX_ROLE_EVENT_BYTES,
        });
    }
    Ok(SignedRoleEvent {
        bytes: message,
        commitment: payload_commitment(&body),
        event,
    })
}

/// Verify `message` against the service's public key and return the event it carries.
pub fn verify_role_event(
    message: &[u8],
    service_key: &[u8; KEY_LEN],
) -> Result<SignedRoleEvent, RoleError> {
    const SHAPE: &str = "a signed role event is a tagged COSE_Sign1 of four parts";
    if message.len() > MAX_ROLE_EVENT_BYTES {
        return Err(RoleError::EventTooLarge {
            len: message.len(),
            limit: MAX_ROLE_EVENT_BYTES,
        });
    }
    let Value::Tag(COSE_SIGN1_TAG, inner) = codec::parse(message, SHAPE)? else {
        return Err(RoleError::EventMalformed { reason: SHAPE });
    };
    let Value::Array(items) = *inner else {
        return Err(RoleError::EventMalformed { reason: SHAPE });
    };
    let Ok([protected, unprotected, payload, signature]) = <[Value; 4]>::try_from(items) else {
        return Err(RoleError::EventMalformed { reason: SHAPE });
    };
    if unprotected != Value::Map(Vec::new()) {
        return Err(RoleError::EventMalformed {
            reason: "the unprotected header is not empty",
        });
    }
    let (Value::Bytes(protected), Value::Bytes(payload), Value::Bytes(signature)) =
        (protected, payload, signature)
    else {
        return Err(RoleError::EventMalformed { reason: SHAPE });
    };
    if protected != protected_header(service_key) {
        return Err(if codec::names_envelope(&protected, ROLE_ENVELOPE) {
            RoleError::SignerMismatch
        } else {
            RoleError::EventMalformed {
                reason: "the protected header is not the role-event header",
            }
        });
    }
    if signature.len() != SIGNATURE_LEN
        || Ed25519Identity::verify(
            service_key,
            &sig_structure(&protected, &payload),
            &signature,
        )
        .is_err()
    {
        return Err(RoleError::SignatureInvalid);
    }
    let event = decode_event_body(&payload)?;
    if cose_sign1(&protected, &payload, &signature) != message {
        return Err(RoleError::EventNotCanonical);
    }
    Ok(SignedRoleEvent {
        bytes: message.to_vec(),
        commitment: payload_commitment(&payload),
        event,
    })
}
