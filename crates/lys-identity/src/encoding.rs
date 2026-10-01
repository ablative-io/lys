//! The canonical encoding of an identity event's body, and its commitment.
//!
//! The body is a canonical CBOR map (RFC 8949 section 4.2: shortest heads,
//! definite lengths, keys in ascending order, no floats) with integer keys 1
//! to 7: version, operation id, actor, identity, recorded-at, change kind and
//! change. docs/design/identity/IDENTITY-EVENTS.md gives every key and code.
//! [`crate::signer`] wraps the body in the signed envelope.
//!
//! Decoding is strict. Bytes are refused unless they decode to the exact
//! shape, the event they name passes [`IdentityEvent::new`], and re-encoding
//! that event gives back the very bytes that were read. So there is one byte
//! string per event, and a non-canonical or padded body never decodes.
//!
//! Two SHA-256 hashes touch an event, and they are named apart. The payload
//! commitment a receipt carries is SHA-256 over the body bytes, named by
//! [`PAYLOAD_COMMITMENT_HASH`]. The log's leaf hash is RFC 6962's SHA-256 over
//! the whole signed message. Neither is a BLAKE3 content address. The head
//! writer is this crate's own because lys-core's is private to lys-core, and
//! lys-core is not changed by this row.

use ciborium::Value;
use sha2::{Digest, Sha256};

use crate::binding::LoginBinding;
use crate::error::IdentityError;
use crate::event::{Change, EVENT_VERSION, IdentityEvent, LinkObservation, wire};
use crate::id::{AgentId, ID_LEN, IdentityId, PersonId};
use crate::lifecycle::LifecycleState;
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::provenance::{Actor, Provenance};

#[path = "encoding_reporting.rs"]
mod reporting;

/// The hash a receipt's payload commitment is made with, over the body bytes.
pub const PAYLOAD_COMMITMENT_HASH: &str = "sha-256";

const MAJOR_UNSIGNED: u8 = 0;
pub(crate) const MAJOR_NEGATIVE: u8 = 1;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
pub(crate) const MAJOR_ARRAY: u8 = 4;
const MAJOR_MAP: u8 = 5;
pub(crate) const MAJOR_TAG: u8 = 6;

/// Write a CBOR head in its shortest form.
pub(crate) fn head(out: &mut Vec<u8>, major: u8, value: u64) {
    let initial = major << 5;
    let be = value.to_be_bytes();
    if value < 24 {
        out.push(initial | be[7]);
    } else if value <= 0xff {
        out.push(initial | 0x18);
        out.push(be[7]);
    } else if value <= 0xffff {
        out.push(initial | 0x19);
        out.extend_from_slice(&be[6..]);
    } else if value <= 0xffff_ffff {
        out.push(initial | 0x1a);
        out.extend_from_slice(&be[4..]);
    } else {
        out.push(initial | 0x1b);
        out.extend_from_slice(&be);
    }
}

pub(crate) fn uint(out: &mut Vec<u8>, value: u64) {
    head(out, MAJOR_UNSIGNED, value);
}

pub(crate) fn text(out: &mut Vec<u8>, value: &str) {
    head(out, MAJOR_TEXT, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

pub(crate) fn bytes(out: &mut Vec<u8>, value: &[u8]) {
    head(out, MAJOR_BYTES, value.len() as u64);
    out.extend_from_slice(value);
}

pub(crate) fn map(out: &mut Vec<u8>, entries: u64) {
    head(out, MAJOR_MAP, entries);
}

fn profile(out: &mut Vec<u8>, value: &Profile) {
    map(out, 1);
    uint(out, 1);
    text(out, value.display_name());
}

fn change(out: &mut Vec<u8>, value: &Change) {
    match value {
        Change::SetupPerson { profile: shown }
        | Change::RegisterPerson { profile: shown }
        | Change::ChangeProfile { profile: shown } => {
            map(out, 1);
            uint(out, 1);
            profile(out, shown);
        }
        Change::RegisterAgent {
            responsible,
            profile: shown,
        } => {
            map(out, 2);
            uint(out, 1);
            bytes(out, responsible.as_bytes());
            uint(out, 2);
            profile(out, shown);
        }
        Change::ReportingRegistration {
            responsible,
            profile: shown,
            reports_to,
        } => reporting::registration(out, *responsible, shown, *reports_to),
        Change::ReportsToChanged {
            from,
            to,
            responsible_from,
            responsible_to,
        } => reporting::changed(out, *from, *to, *responsible_from, *responsible_to),
        Change::BindLogin { binding } => {
            map(out, 2);
            uint(out, 1);
            text(out, binding.issuer());
            uint(out, 2);
            text(out, binding.subject());
        }
        Change::Transition {
            transition,
            from,
            to,
            reason,
        } => {
            map(out, 4);
            uint(out, 1);
            uint(out, wire::transition(*transition));
            uint(out, 2);
            uint(out, wire::state(*from));
            uint(out, 3);
            uint(out, wire::state(*to));
            uint(out, 4);
            text(out, reason);
        }
        Change::LinkAudit(seen) => {
            map(out, 6);
            uint(out, 1);
            text(out, seen.source_operation_id());
            uint(out, 2);
            uint(out, wire::link(seen.change()));
            uint(out, 3);
            text(out, seen.binding().issuer());
            uint(out, 4);
            text(out, seen.binding().subject());
            uint(out, 5);
            text(out, seen.observer());
            uint(out, 6);
            uint(out, seen.observed_at());
        }
    }
}

/// The actor's map: keys 1 to 4, and the agent's id under key 5 when an agent
/// signed the request. An OIDC actor is written as it always was.
pub(crate) fn actor(out: &mut Vec<u8>, value: &Actor) {
    let provenance = value.provenance();
    let principal = provenance
        .agent()
        .map(|id| *id.as_bytes())
        .or_else(|| provenance.service_account().map(|id| *id.as_bytes()));
    map(out, if principal.is_some() { 5 } else { 4 });
    uint(out, 1);
    text(out, value.binding().issuer());
    uint(out, 2);
    text(out, value.binding().subject());
    uint(out, 3);
    uint(out, wire::method(provenance.method()));
    uint(out, 4);
    uint(out, provenance.authenticated_at());
    if let Some(principal) = principal {
        uint(out, 5);
        bytes(out, &principal);
    }
}

/// The canonical body bytes of `event`: the COSE payload, and what the payload commitment is taken over.
pub fn encode_body(event: &IdentityEvent) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 7);
    uint(&mut out, 1);
    uint(&mut out, event.version());
    uint(&mut out, 2);
    bytes(&mut out, event.operation().as_bytes());
    uint(&mut out, 3);
    actor(&mut out, event.actor());
    uint(&mut out, 4);
    map(&mut out, 2);
    uint(&mut out, 1);
    let (kind, id) = match event.identity() {
        IdentityId::Person(id) => (wire::PERSON, *id.as_bytes()),
        IdentityId::Agent(id) => (wire::AGENT, *id.as_bytes()),
        IdentityId::ServiceAccount(id) => (wire::SERVICE_ACCOUNT, *id.as_bytes()),
    };
    uint(&mut out, kind);
    uint(&mut out, 2);
    bytes(&mut out, &id);
    uint(&mut out, 5);
    uint(&mut out, event.recorded_at());
    uint(&mut out, 6);
    uint(&mut out, wire::change(event.change()));
    uint(&mut out, 7);
    change(&mut out, event.change());
    out
}

/// SHA-256 over an event's body bytes: the payload commitment a receipt carries.
pub fn payload_commitment(body: &[u8]) -> [u8; 32] {
    Sha256::digest(body).into()
}

pub(crate) fn malformed(reason: &'static str) -> IdentityError {
    IdentityError::EventMalformed { reason }
}

/// The values of a map whose keys are exactly 1 to `N`, in order.
pub(crate) fn fields<const N: usize>(
    value: Value,
    reason: &'static str,
) -> Result<[Value; N], IdentityError> {
    let Value::Map(pairs) = value else {
        return Err(malformed(reason));
    };
    if pairs.len() != N {
        return Err(malformed(reason));
    }
    let mut values = Vec::with_capacity(N);
    for (expected, (key, value)) in (1..).zip(pairs) {
        if as_uint(&key, reason)? != expected {
            return Err(malformed(reason));
        }
        values.push(value);
    }
    <[Value; N]>::try_from(values)
        .ok()
        .ok_or_else(|| malformed(reason))
}

pub(crate) fn as_uint(value: &Value, reason: &'static str) -> Result<u64, IdentityError> {
    match value {
        Value::Integer(integer) => u64::try_from(*integer)
            .ok()
            .ok_or_else(|| malformed(reason)),
        _ => Err(malformed(reason)),
    }
}

pub(crate) fn as_text(value: Value, reason: &'static str) -> Result<String, IdentityError> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(malformed(reason)),
    }
}

pub(crate) fn as_bytes(value: Value, reason: &'static str) -> Result<Vec<u8>, IdentityError> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        _ => Err(malformed(reason)),
    }
}

fn as_id(value: Value, reason: &'static str) -> Result<[u8; ID_LEN], IdentityError> {
    <[u8; ID_LEN]>::try_from(as_bytes(value, reason)?)
        .ok()
        .ok_or_else(|| malformed(reason))
}

pub(crate) fn cbor(bytes: &[u8], reason: &'static str) -> Result<Value, IdentityError> {
    ciborium::from_reader(bytes)
        .ok()
        .ok_or_else(|| malformed(reason))
}

fn decode_profile(value: Value) -> Result<Profile, IdentityError> {
    let [name] = fields::<1>(value, "a profile is a map of key 1")?;
    Profile::new(&as_text(name, "a display name is text")?)
}

fn state(value: &Value) -> Result<LifecycleState, IdentityError> {
    wire::state_from(as_uint(value, "a lifecycle state is a code")?)
        .ok_or_else(|| malformed("a lifecycle state code is 1 to 4"))
}

fn decode_change(kind: u64, value: Value) -> Result<Change, IdentityError> {
    const TEXT: &str = "a change field is text";
    match kind {
        wire::SETUP_PERSON => {
            let [shown] = fields::<1>(value, "a setup is a map of key 1")?;
            Ok(Change::SetupPerson {
                profile: decode_profile(shown)?,
            })
        }
        wire::REGISTER_PERSON => {
            let [shown] = fields::<1>(value, "a person registration is a map of key 1")?;
            Ok(Change::RegisterPerson {
                profile: decode_profile(shown)?,
            })
        }
        wire::REGISTER_AGENT => {
            let [responsible, shown] =
                fields::<2>(value, "an agent registration is a map of keys 1 and 2")?;
            Ok(Change::RegisterAgent {
                responsible: PersonId::from_bytes(as_id(
                    responsible,
                    "a responsible person is 16 bytes",
                )?),
                profile: decode_profile(shown)?,
            })
        }
        wire::CHANGE_PROFILE => {
            let [shown] = fields::<1>(value, "a profile change is a map of key 1")?;
            Ok(Change::ChangeProfile {
                profile: decode_profile(shown)?,
            })
        }
        wire::BIND_LOGIN => {
            let [issuer, subject] = fields::<2>(value, "a login binding is a map of keys 1 and 2")?;
            Ok(Change::BindLogin {
                binding: LoginBinding::new(&as_text(issuer, TEXT)?, &as_text(subject, TEXT)?)?,
            })
        }
        wire::TRANSITION => {
            let [transition, from, to, reason] =
                fields::<4>(value, "a transition is a map of keys 1 to 4")?;
            let transition = wire::transition_from(as_uint(&transition, "a transition is a code")?)
                .ok_or_else(|| malformed("a transition code is 1 to 4"))?;
            Ok(Change::Transition {
                transition,
                from: state(&from)?,
                to: state(&to)?,
                reason: as_text(reason, TEXT)?,
            })
        }
        wire::LINK_AUDIT => {
            let [source, change, issuer, subject, observer, observed_at] =
                fields::<6>(value, "a link observation is a map of keys 1 to 6")?;
            let change = wire::link_from(as_uint(&change, "a link change is a code")?)
                .ok_or_else(|| malformed("a link change code is 1 or 2"))?;
            let binding = LoginBinding::new(&as_text(issuer, TEXT)?, &as_text(subject, TEXT)?)?;
            Ok(Change::LinkAudit(LinkObservation::new(
                &as_text(source, TEXT)?,
                change,
                binding,
                &as_text(observer, TEXT)?,
                as_uint(&observed_at, "an observation time is seconds")?,
            )?))
        }
        wire::REPORTING_REGISTRATION | wire::REPORTS_TO_CHANGED => reporting::decode(kind, value),
        _ => Err(malformed("a change kind is 1 to 9")),
    }
}

/// The actor an actor's map names: keys 1 to 4, and the agent's id under key
/// 5 for an agent signature or service-account bearer. Code 3 without key 5
/// is the operator; code 3 with a 16-byte key 5 is a service account.
pub(crate) fn decode_actor(value: Value, version: u64) -> Result<Actor, IdentityError> {
    const SHAPE: &str = "an actor is a map of keys 1 to 4, or 1 to 5 when it names a principal";
    let named = matches!(&value, Value::Map(pairs) if pairs.len() == 5);
    let ([issuer, subject, method, authenticated_at], agent) = if named {
        let [issuer, subject, method, authenticated_at, agent] = fields::<5>(value, SHAPE)?;
        let agent = AgentId::from_bytes(as_id(
            agent,
            "actor key 5 must hold a 16-byte principal id",
        )?);
        ([issuer, subject, method, authenticated_at], Some(agent))
    } else {
        (fields::<4>(value, SHAPE)?, None)
    };
    let code = as_uint(&method, "an authentication method is a code")?;
    if code == wire::OPERATOR_OR_SERVICE_ACCOUNT && version == EVENT_VERSION && named {
        return Err(malformed(
            "an operator actor carries no principal id under key 5",
        ));
    }
    let method = wire::method_from(code, agent).map_err(malformed)?;
    Ok(Actor::new(
        LoginBinding::new(
            &as_text(issuer, "an actor's issuer is text")?,
            &as_text(subject, "an actor's subject is text")?,
        )?,
        Provenance::new(
            method,
            as_uint(&authenticated_at, "an authentication time is seconds")?,
        ),
    ))
}

/// The event an event body's bytes name, refused unless the bytes are its canonical encoding.
pub fn decode_body(body: &[u8]) -> Result<IdentityEvent, IdentityError> {
    const SHAPE: &str = "the body is a map of keys 1 to 7";
    let value = cbor(body, SHAPE)?;
    if let Value::Map(pairs) = &value
        && let Some((_, version)) = pairs.first()
    {
        let version = as_uint(version, SHAPE)?;
        if version != EVENT_VERSION && version != 2 {
            return Err(IdentityError::VersionUnsupported { version });
        }
    }
    let [
        version,
        operation,
        actor,
        identity,
        recorded_at,
        kind,
        change,
    ] = fields::<7>(value, SHAPE)?;
    let version = as_uint(&version, SHAPE)?;
    let actor = decode_actor(actor, version)?;
    let [identity_kind, identity_id] =
        fields::<2>(identity, "an identity is a map of keys 1 and 2")?;
    let identity_id = as_id(identity_id, "an identity id is 16 bytes")?;
    let identity = match as_uint(&identity_kind, "an identity kind is a code")? {
        wire::PERSON => IdentityId::Person(PersonId::from_bytes(identity_id)),
        wire::AGENT => IdentityId::Agent(AgentId::from_bytes(identity_id)),
        _ => return Err(malformed("an identity kind code is 1 or 2")),
    };
    let event = IdentityEvent::new(
        OperationId::from_bytes(as_id(operation, "an operation id is 16 bytes")?),
        actor,
        identity,
        as_uint(&recorded_at, "a recorded time is seconds")?,
        decode_change(as_uint(&kind, "a change kind is a code")?, change)?,
    )?;
    if event.version() != version {
        return Err(malformed(
            "event version does not match actor authentication method",
        ));
    }
    if encode_body(&event) != body {
        return Err(IdentityError::EventNotCanonical);
    }
    Ok(event)
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
