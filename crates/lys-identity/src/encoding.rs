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
        // A registration under no organisation keeps its one-key map, so
        // every leaf written before DIRECTORY-109 re-encodes to its bytes.
        Change::SetupPerson {
            profile: shown,
            organisation: None,
        }
        | Change::RegisterPerson {
            profile: shown,
            organisation: None,
        }
        | Change::ChangeProfile { profile: shown } => {
            map(out, 1);
            uint(out, 1);
            profile(out, shown);
        }
        Change::SetupPerson {
            profile: shown,
            organisation: Some(organisation),
        }
        | Change::RegisterPerson {
            profile: shown,
            organisation: Some(organisation),
        } => {
            map(out, 2);
            uint(out, 1);
            profile(out, shown);
            uint(out, 2);
            bytes(out, organisation.as_bytes());
        }
        Change::FoundOrganisation {
            profile: shown,
            founder,
        } => {
            map(out, 2);
            uint(out, 1);
            profile(out, shown);
            uint(out, 2);
            bytes(out, founder.as_bytes());
        }
        Change::AttachOrganisation { organisation } => {
            map(out, 1);
            uint(out, 1);
            bytes(out, organisation.as_bytes());
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
        Change::BindLogin { binding } | Change::UnbindLogin { binding } => {
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
        Change::AdmissionWithdrawn { digest } => {
            map(out, 1);
            uint(out, 1);
            text(out, digest);
        }
        Change::AgentCall(call) => {
            map(out, 4);
            uint(out, 1);
            text(out, call.method());
            uint(out, 2);
            text(out, call.path());
            uint(out, 3);
            text(out, call.body_sha256());
            uint(out, 4);
            text(out, call.signature());
        }
    }
}

/// The actor's map: keys 1 to 4, and the agent's id under key 5 when an agent
/// signed the request; a draft under key 6, and the app the act came through
/// under key 7, each with the keys before it written. An OIDC actor is
/// written as it always was.
pub(crate) fn actor(out: &mut Vec<u8>, value: &Actor) {
    let provenance = value.provenance();
    let principal = provenance
        .agent()
        .map(|id| *id.as_bytes())
        .or_else(|| provenance.service_account().map(|id| *id.as_bytes()));
    let through = value.through_app();
    map(
        out,
        if through.is_some() {
            7
        } else if value.draft().is_some() {
            6
        } else if principal.is_some() {
            5
        } else {
            4
        },
    );
    uint(out, 1);
    text(out, value.binding().issuer());
    uint(out, 2);
    text(out, value.binding().subject());
    uint(out, 3);
    uint(out, wire::method(provenance.method()));
    uint(out, 4);
    match provenance {
        Provenance::Authenticated {
            authenticated_at, ..
        } => uint(out, *authenticated_at),
        Provenance::AgentPass {
            launch, session, ..
        } => {
            head(out, MAJOR_ARRAY, 2);
            text(out, launch);
            text(out, session);
        }
    }
    if through.is_some() || value.draft().is_some() || principal.is_some() {
        uint(out, 5);
        if let Some(principal) = principal {
            bytes(out, &principal);
        } else {
            out.push(0xf6);
        }
    }
    if let Some(draft) = value.draft() {
        uint(out, 6);
        head(out, MAJOR_ARRAY, 3);
        text(out, &draft.draft.to_string());
        text(out, &draft.application.to_string());
        if let Some(agent) = draft.drafted_by {
            text(out, &agent.to_string());
        } else {
            out.push(0xf6);
        }
    } else if through.is_some() {
        uint(out, 6);
        out.push(0xf6);
    }
    if let Some(app) = through {
        uint(out, 7);
        text(out, app);
    }
}

/// The canonical body bytes of `event`: the COSE payload, and what the payload commitment is taken over.
#[must_use]
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
        IdentityId::Connector(id) => (wire::CONNECTOR, *id.as_bytes()),
        IdentityId::Machine(id) => (wire::MACHINE, *id.as_bytes()),
        IdentityId::Organisation(id) => (wire::ORGANISATION, *id.as_bytes()),
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
#[must_use]
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

#[path = "encoding_read.rs"]
mod read;

pub(crate) use read::decode_actor;
use read::decode_profile;
pub use read::{HIGHEST_KIND_BEFORE_ORGANISATIONS, decode_body, decode_body_before_organisations};

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "encoding_rollback_tests.rs"]
mod rollback_tests;
