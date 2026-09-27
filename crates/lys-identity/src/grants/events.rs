//! The grant event: one grant change and its audit record at once.
//!
//! An event names the caller's operation id, the identity that made the
//! request, when the service recorded it, and the change: a grant issued, or
//! a grant revoked. A grant change is recorded only as one of these events,
//! and both the grant book and the permission relationships are derived from
//! it, never written beside it.
//!
//! A signed event is a `COSE_Sign1` message (RFC 9052, tag 18) in the shape of
//! `lys/identity-event/v1`, with its own content type, [`GRANT_ENVELOPE`], in
//! the protected header. The body is a canonical CBOR map: `1` version, `2`
//! operation id, `3` caller, `4` recorded-at, `5` change kind (`1` issue, `2`
//! revoke) and `6` the change: the grant's own map, or `1` grant id and `2`
//! reason. Reading is strict: a message naming another envelope is refused
//! `EnvelopeMismatch`, and bytes that are not the exact canonical message are
//! refused.

use ciborium::Value;
use lys_core::Ed25519Identity;

use super::codec::{
    GRANT_ENVELOPE, as_id, as_text, as_uint, fields, read_grant, read_identity, write_grant,
    write_identity,
};
use super::error::GrantError;
use super::types::{Grant, GrantId};
use crate::encoding::{
    MAJOR_ARRAY, MAJOR_NEGATIVE, MAJOR_TAG, bytes, head, map, payload_commitment, text, uint,
};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// The grant event version this crate writes and reads.
pub const GRANT_EVENT_VERSION: u64 = 1;

/// The largest grant event written or read, in bytes.
pub const MAX_GRANT_EVENT_BYTES: usize = 64 * 1024;

const COSE_SIGN1_TAG: u64 = 18;
const SIGNATURE_LEN: usize = 64;
const KEY_LEN: usize = 32;

/// The longest reason a revocation may give, in bytes.
pub const REVOKE_REASON_MAX_BYTES: usize = 1024;

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
}

/// A grant change, before it is signed or after it is verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantEvent {
    operation: OperationId,
    caller: IdentityId,
    recorded_at: u64,
    change: GrantChange,
}

impl GrantEvent {
    /// The event recording `change`, requested by `caller` under `operation`.
    ///
    /// An issued grant names this event's operation as the one that
    /// authorised it, and its issuer is the caller; a revocation names a
    /// reason of 1 to 1024 bytes.
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
                if reason.trim().is_empty() || reason.len() > REVOKE_REASON_MAX_BYTES {
                    return Err(GrantError::EventMismatch {
                        reason: "a revocation names its reason in 1 to 1024 bytes",
                    });
                }
            }
        }
        Ok(Self {
            operation,
            caller,
            recorded_at,
            change,
        })
    }

    /// The operation id the caller gave the change.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The identity that made the request.
    pub fn caller(&self) -> IdentityId {
        self.caller
    }

    /// When the service recorded the change, in seconds since the Unix epoch.
    pub fn recorded_at(&self) -> u64 {
        self.recorded_at
    }

    /// The change.
    pub fn change(&self) -> &GrantChange {
        &self.change
    }

    /// The grant the change is about.
    pub fn grant(&self) -> GrantId {
        match &self.change {
            GrantChange::Issue(grant) => grant.id(),
            GrantChange::Revoke { grant, .. } => *grant,
        }
    }
}

/// The change kind's wire code: `1` issue, `2` revoke.
pub fn change_kind(change: &GrantChange) -> u64 {
    match change {
        GrantChange::Issue(_) => 1,
        GrantChange::Revoke { .. } => 2,
    }
}

/// The canonical body bytes of `event`.
pub fn encode_event_body(event: &GrantEvent) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 6);
    uint(&mut out, 1);
    uint(&mut out, GRANT_EVENT_VERSION);
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
    }
    out
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
pub fn decode_event_body(body: &[u8]) -> Result<GrantEvent, GrantError> {
    const SHAPE: &str = "a grant event body is a map of keys 1 to 6";
    let value = cbor(body, SHAPE)?;
    if let Value::Map(pairs) = &value
        && let Some((_, version)) = pairs.first()
    {
        let version = as_uint(version, SHAPE)?;
        if version != GRANT_EVENT_VERSION {
            return Err(GrantError::VersionUnsupported { version });
        }
    }
    let [_, operation, caller, recorded_at, kind, change] = fields::<6>(value, SHAPE)?;
    let change = match as_uint(&kind, "a change kind is a code")? {
        1 => GrantChange::Issue(Box::new(read_grant(change)?)),
        2 => {
            let [grant, reason] = fields::<2>(change, "a revocation is a map of keys 1 and 2")?;
            GrantChange::Revoke {
                grant: GrantId::from_bytes(as_id(grant, "a grant id is 16 bytes")?),
                reason: as_text(reason, "a reason is text")?,
            }
        }
        _ => return Err(malformed("a grant change kind is 1 or 2")),
    };
    let event = GrantEvent::new(
        OperationId::from_bytes(as_id(operation, "an operation id is 16 bytes")?),
        read_identity(caller)?,
        as_uint(&recorded_at, "a recorded time is seconds")?,
        change,
    )?;
    if encode_event_body(&event) != body {
        return Err(GrantError::EventNotCanonical);
    }
    Ok(event)
}

fn protected_header(kid: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 3);
    uint(&mut out, 1);
    head(&mut out, MAJOR_NEGATIVE, 7);
    uint(&mut out, 3);
    text(&mut out, GRANT_ENVELOPE);
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

/// A grant event with the exact bytes that were signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedGrantEvent {
    bytes: Vec<u8>,
    event: GrantEvent,
    commitment: [u8; 32],
}

impl SignedGrantEvent {
    /// The whole `COSE_Sign1` message: the log leaf.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The event the message carries.
    pub fn event(&self) -> &GrantEvent {
        &self.event
    }

    /// SHA-256 over the body bytes.
    pub fn payload_commitment(&self) -> [u8; 32] {
        self.commitment
    }
}

/// Sign `event` with the service key, refusing one larger than is read back.
pub fn sign_grant_event(
    event: GrantEvent,
    service_key: &Ed25519Identity,
) -> Result<SignedGrantEvent, GrantError> {
    let body = encode_event_body(&event);
    let protected = protected_header(&service_key.public_key_bytes());
    let signature = service_key.sign(&sig_structure(&protected, &body));
    let bytes = cose_sign1(&protected, &body, &signature);
    if bytes.len() > MAX_GRANT_EVENT_BYTES {
        return Err(GrantError::EventTooLarge {
            len: bytes.len(),
            limit: MAX_GRANT_EVENT_BYTES,
        });
    }
    Ok(SignedGrantEvent {
        bytes,
        commitment: payload_commitment(&body),
        event,
    })
}

fn as_bytes(value: Value, reason: &'static str) -> Result<Vec<u8>, GrantError> {
    match value {
        Value::Bytes(raw) => Ok(raw),
        _ => Err(malformed(reason)),
    }
}

/// The protected header's content type and key id.
fn header(protected: &[u8]) -> Result<(String, Vec<u8>), GrantError> {
    const SHAPE: &str = "the protected header is not a map naming a content type and a key id";
    let Value::Map(pairs) = cbor(protected, SHAPE)? else {
        return Err(malformed(SHAPE));
    };
    let mut content_type = None;
    let mut kid = None;
    for (key, value) in pairs {
        match (as_uint(&key, SHAPE), value) {
            (Ok(3), Value::Text(found)) => content_type = Some(found),
            (Ok(4), Value::Bytes(found)) => kid = Some(found),
            _ => {}
        }
    }
    Ok((
        content_type.ok_or_else(|| malformed(SHAPE))?,
        kid.ok_or_else(|| malformed(SHAPE))?,
    ))
}

/// Verify `message` against the service key and return the grant event it carries.
pub fn verify_grant_event(
    message: &[u8],
    service_key: &[u8; KEY_LEN],
) -> Result<SignedGrantEvent, GrantError> {
    const SHAPE: &str = "the message is not a tagged COSE_Sign1 of four parts";
    if message.len() > MAX_GRANT_EVENT_BYTES {
        return Err(GrantError::EventTooLarge {
            len: message.len(),
            limit: MAX_GRANT_EVENT_BYTES,
        });
    }
    let Value::Tag(COSE_SIGN1_TAG, inner) = cbor(message, SHAPE)? else {
        return Err(malformed(SHAPE));
    };
    let Value::Array(items) = *inner else {
        return Err(malformed(SHAPE));
    };
    let [protected, _, payload, signature] = <[Value; 4]>::try_from(items)
        .ok()
        .ok_or_else(|| malformed(SHAPE))?;
    let (protected, payload, signature) = (
        as_bytes(protected, SHAPE)?,
        as_bytes(payload, SHAPE)?,
        as_bytes(signature, SHAPE)?,
    );
    let (content_type, kid) = header(&protected)?;
    if content_type != GRANT_ENVELOPE {
        return Err(GrantError::EnvelopeMismatch {
            reason: format!("it names {content_type}"),
        });
    }
    let kid = <[u8; KEY_LEN]>::try_from(kid.as_slice())
        .ok()
        .ok_or_else(|| malformed("the key id is not a 32-byte Ed25519 public key"))?;
    if protected != protected_header(&kid) {
        return Err(malformed(
            "the protected header is not the grant-event header",
        ));
    }
    if &kid != service_key {
        return Err(GrantError::SignerMismatch);
    }
    if signature.len() != SIGNATURE_LEN
        || Ed25519Identity::verify(
            service_key,
            &sig_structure(&protected, &payload),
            &signature,
        )
        .is_err()
    {
        return Err(GrantError::SignatureInvalid);
    }
    let event = decode_event_body(&payload)?;
    if cose_sign1(&protected, &payload, &signature) != message {
        return Err(GrantError::EventNotCanonical);
    }
    Ok(SignedGrantEvent {
        bytes: message.to_vec(),
        commitment: payload_commitment(&payload),
        event,
    })
}
