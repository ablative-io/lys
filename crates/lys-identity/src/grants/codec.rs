//! The canonical encoding of a grant.
//!
//! A grant is a canonical CBOR map (RFC 8949 section 4.2) with integer keys 1
//! to 12, every one required; docs/design/identity/GRANT-CONTRACT.md gives
//! each key and code. Decoding is strict: a key outside 1 to 14 is refused
//! `MemberUnknown`, a missing key `MemberMissing`, an unknown recipient kind
//! `RecipientKindUnknown`, a malformed lineage `LineageMalformed`, and bytes
//! that are not the canonical encoding of what they decode to
//! `GrantNotCanonical`. No member has a default. Two keys are written only
//! when they say something: 13, true for a one-time grant, and 14, the mode
//! (`1` by draft, `2` by two); absent, the grant is outright, so a grant
//! written before the mode existed keeps its bytes (ACCESS-001 R1).

use std::collections::BTreeSet;

use ciborium::Value;

use super::error::GrantError;
use super::types::{
    Action, Grant, GrantId, GrantParts, Mode, PassOn, RecipientKind, Relation, Resource, Source,
    Window,
};
use crate::encoding::{MAJOR_ARRAY, bytes, head, map, text, uint};
use crate::id::{AgentId, ConnectorId, ID_LEN, IdentityId, PersonId, ServiceAccountId};
use crate::operation::OperationId;

/// The application-envelope identifier a signed grant event names in its
/// protected header. It is not `lys/identity-event/v1` and not
/// `lys/delegation/v1`: neither of those is read as a grant, and a grant is
/// never read as either. docs/design/identity/GRANT-CONTRACT.md records it.
pub const GRANT_ENVELOPE: &str = "application/vnd.lys.grant-event.v1+cbor";

/// The members of a grant, by key: key 1 is `MEMBERS[0]`.
pub const MEMBERS: [&str; 12] = [
    "id",
    "issuer",
    "holder",
    "responsible person",
    "resource",
    "relation",
    "actions",
    "pass-on",
    "source",
    "window",
    "model version",
    "authorising operation",
];

const NULL: u8 = 0xf6;
const TRUE: u8 = 0xf5;

pub(crate) fn write_identity(out: &mut Vec<u8>, identity: IdentityId) {
    let (kind, id) = match identity {
        IdentityId::Person(id) => (1, *id.as_bytes()),
        IdentityId::Agent(id) => (2, *id.as_bytes()),
        IdentityId::ServiceAccount(id) => (3, *id.as_bytes()),
        IdentityId::Connector(id) => (4, *id.as_bytes()),
    };
    map(out, 2);
    uint(out, 1);
    uint(out, kind);
    uint(out, 2);
    bytes(out, &id);
}

fn write_actions(out: &mut Vec<u8>, actions: &BTreeSet<Action>) {
    head(out, MAJOR_ARRAY, actions.len() as u64);
    for action in actions {
        text(out, action.as_str());
    }
}

/// A recipient kind's wire code.
pub(crate) fn recipient_code(kind: RecipientKind) -> u64 {
    match kind {
        RecipientKind::Person => 1,
        RecipientKind::Agent => 2,
        RecipientKind::ServiceAccount => 3,
        RecipientKind::Connector => 4,
    }
}

/// Append the canonical encoding of `grant` to `out`.
pub(crate) fn write_grant(out: &mut Vec<u8>, grant: &Grant) {
    let parts = grant.parts();
    let held = mode_code(grant.mode());
    map(
        out,
        12 + u64::from(grant.is_once()) + u64::from(held.is_some()),
    );
    uint(out, 1);
    bytes(out, parts.id.as_bytes());
    uint(out, 2);
    write_identity(out, parts.issuer);
    uint(out, 3);
    write_identity(out, parts.holder);
    uint(out, 4);
    bytes(out, parts.responsible.as_bytes());
    uint(out, 5);
    map(out, 2);
    uint(out, 1);
    text(out, parts.resource.kind());
    uint(out, 2);
    text(out, parts.resource.id());
    uint(out, 6);
    text(out, parts.relation.as_str());
    uint(out, 7);
    write_actions(out, &parts.actions);
    uint(out, 8);
    match &parts.pass_on {
        PassOn::UseOnly => uint(out, 0),
        PassOn::To {
            actions,
            recipients,
        } => {
            map(out, 2);
            uint(out, 1);
            write_actions(out, actions);
            uint(out, 2);
            head(out, MAJOR_ARRAY, recipients.len() as u64);
            for kind in recipients {
                uint(out, recipient_code(*kind));
            }
        }
    }
    uint(out, 9);
    match parts.source {
        Source::Root => uint(out, 0),
        Source::Grant(source) => bytes(out, source.as_bytes()),
    }
    uint(out, 10);
    map(out, 2);
    uint(out, 1);
    uint(out, parts.window.starts_at());
    uint(out, 2);
    match parts.window.ends_at() {
        Some(ends) => uint(out, ends),
        None => out.push(NULL),
    }
    uint(out, 11);
    uint(out, parts.model_version);
    uint(out, 12);
    bytes(out, parts.operation.as_bytes());
    if grant.is_once() {
        uint(out, 13);
        out.push(TRUE);
    }
    if let Some(code) = held {
        uint(out, 14);
        uint(out, code);
    }
}

/// A held mode's wire code; outright is written by its absence.
fn mode_code(mode: Mode) -> Option<u64> {
    match mode {
        Mode::Outright => None,
        Mode::ByDraft => Some(1),
        Mode::ByTwo => Some(2),
    }
}

/// The canonical bytes of `grant`.
pub fn encode_grant(grant: &Grant) -> Vec<u8> {
    let mut out = Vec::new();
    write_grant(&mut out, grant);
    out
}

fn malformed(reason: &'static str) -> GrantError {
    GrantError::GrantMalformed { reason }
}

pub(crate) fn as_uint(value: &Value, reason: &'static str) -> Result<u64, GrantError> {
    match value {
        Value::Integer(integer) => u64::try_from(*integer)
            .ok()
            .ok_or_else(|| malformed(reason)),
        _ => Err(malformed(reason)),
    }
}

pub(crate) fn as_text(value: Value, reason: &'static str) -> Result<String, GrantError> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(malformed(reason)),
    }
}

pub(crate) fn as_id(value: Value, reason: &'static str) -> Result<[u8; ID_LEN], GrantError> {
    match value {
        Value::Bytes(raw) => <[u8; ID_LEN]>::try_from(raw)
            .ok()
            .ok_or_else(|| malformed(reason)),
        _ => Err(malformed(reason)),
    }
}

/// The values of a map whose keys are exactly 1 to `N`, in order.
pub(crate) fn fields<const N: usize>(
    value: Value,
    reason: &'static str,
) -> Result<[Value; N], GrantError> {
    let Value::Map(pairs) = value else {
        return Err(malformed(reason));
    };
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

pub(crate) fn read_identity(value: Value) -> Result<IdentityId, GrantError> {
    const SHAPE: &str = "an identity is a map of keys 1 and 2";
    let [kind, id] = fields::<2>(value, SHAPE)?;
    let id = as_id(id, "an identity id is 16 bytes")?;
    match as_uint(&kind, SHAPE)? {
        1 => Ok(IdentityId::Person(PersonId::from_bytes(id))),
        2 => Ok(IdentityId::Agent(AgentId::from_bytes(id))),
        3 => Ok(IdentityId::ServiceAccount(ServiceAccountId::from_bytes(id))),
        4 => Ok(IdentityId::Connector(ConnectorId::from_bytes(id))),
        _ => Err(malformed(
            "an identity kind code is 1 for a person, 2 for an agent, 3 for a service account or 4 for a connector",
        )),
    }
}

fn read_actions(value: Value) -> Result<BTreeSet<Action>, GrantError> {
    let Value::Array(items) = value else {
        return Err(malformed("actions are an array of text"));
    };
    items
        .into_iter()
        .map(|item| Action::new(&as_text(item, "an action is text")?))
        .collect()
}

fn read_pass_on(value: Value) -> Result<PassOn, GrantError> {
    if let Value::Integer(_) = value {
        return match as_uint(&value, "pass-on is 0 or a map")? {
            0 => Ok(PassOn::UseOnly),
            _ => Err(malformed(
                "pass-on is 0 for use-only or a map of keys 1 and 2",
            )),
        };
    }
    let [actions, recipients] = fields::<2>(value, "pass-on is a map of keys 1 and 2")?;
    let Value::Array(kinds) = recipients else {
        return Err(malformed("recipient kinds are an array of codes"));
    };
    let recipients = kinds
        .iter()
        .map(|kind| match as_uint(kind, "a recipient kind is a code")? {
            1 => Ok(RecipientKind::Person),
            2 => Ok(RecipientKind::Agent),
            3 => Ok(RecipientKind::ServiceAccount),
            4 => Ok(RecipientKind::Connector),
            code => Err(GrantError::RecipientKindUnknown { code }),
        })
        .collect::<Result<_, _>>()?;
    PassOn::to(read_actions(actions)?, recipients)
}

fn read_source(value: Value) -> Result<Source, GrantError> {
    const SHAPE: &str = "a source is 0 for a root or a 16-byte grant id";
    match value {
        Value::Integer(_) if as_uint(&value, SHAPE)? == 0 => Ok(Source::Root),
        Value::Bytes(_) => Ok(Source::Grant(GrantId::from_bytes(as_id(value, SHAPE)?))),
        _ => Err(GrantError::LineageMalformed { reason: SHAPE }),
    }
}

fn read_window(value: Value) -> Result<Window, GrantError> {
    let [starts, ends] = fields::<2>(value, "a window is a map of keys 1 and 2")?;
    let ends = match ends {
        Value::Null => None,
        ends => Some(as_uint(&ends, "a window end is seconds or null")?),
    };
    Window::new(as_uint(&starts, "a window start is seconds")?, ends)
}

/// Take every member of a grant map, refusing an unknown or a missing one by name.
fn members(value: Value) -> Result<([Value; 12], bool, Mode), GrantError> {
    let Value::Map(pairs) = value else {
        return Err(malformed("a grant is a map of keys 1 to 12"));
    };
    let mut slots: [Option<Value>; 12] = Default::default();
    let mut once = None;
    let mut mode = None;
    for (key, value) in pairs {
        let key = as_uint(&key, "a grant's keys are unsigned integers")?;
        if key == 13 {
            if value != Value::Bool(true) || once.replace(true).is_some() {
                return Err(malformed("a one-time grant names key 13 once, as true"));
            }
            continue;
        }
        if key == 14 {
            const MODE: &str = "a held mode names key 14 once, as 1 by draft or 2 by two";
            let held = match as_uint(&value, MODE)? {
                1 => Mode::ByDraft,
                2 => Mode::ByTwo,
                _ => return Err(malformed(MODE)),
            };
            if mode.replace(held).is_some() {
                return Err(malformed(MODE));
            }
            continue;
        }
        let slot = usize::try_from(key)
            .ok()
            .and_then(|key| key.checked_sub(1))
            .and_then(|index| slots.get_mut(index))
            .ok_or(GrantError::MemberUnknown { key })?;
        if slot.replace(value).is_some() {
            return Err(malformed("a grant names a member twice"));
        }
    }
    let mut taken = Vec::with_capacity(12);
    for (slot, member) in slots.into_iter().zip(MEMBERS) {
        taken.push(slot.ok_or(GrantError::MemberMissing { member })?);
    }
    let taken = <[Value; 12]>::try_from(taken)
        .ok()
        .ok_or_else(|| malformed("a grant is a map of keys 1 to 12"))?;
    Ok((taken, once.is_some(), mode.unwrap_or(Mode::Outright)))
}

/// The grant a decoded CBOR value names, checked against the contract.
pub(crate) fn read_grant(value: Value) -> Result<Grant, GrantError> {
    let (grant_members, once, mode) = members(value)?;
    let [
        id,
        issuer,
        holder,
        responsible,
        resource,
        relation,
        actions,
        pass_on,
        source,
        window,
        model_version,
        operation,
    ] = grant_members;
    let [kind, resource_id] = fields::<2>(resource, "a resource is a map of keys 1 and 2")?;
    let parts = GrantParts {
        id: GrantId::from_bytes(as_id(id, "a grant id is 16 bytes")?),
        issuer: read_identity(issuer)?,
        holder: read_identity(holder)?,
        responsible: PersonId::from_bytes(as_id(responsible, "a person id is 16 bytes")?),
        resource: Resource::new(
            &as_text(kind, "a resource kind is text")?,
            &as_text(resource_id, "a resource id is text")?,
        )?,
        relation: Relation::new(&as_text(relation, "a relation is text")?)?,
        actions: read_actions(actions)?,
        pass_on: read_pass_on(pass_on)?,
        source: read_source(source)?,
        window: read_window(window)?,
        model_version: as_uint(&model_version, "a model version is an unsigned integer")?,
        operation: OperationId::from_bytes(as_id(operation, "an operation id is 16 bytes")?),
    };
    let grant = if once {
        Grant::once(parts)
    } else {
        Grant::new(parts)
    }?;
    Ok(grant.with_mode(mode))
}

/// The grant `encoded` names, refused unless the bytes are its canonical encoding.
pub fn decode_grant(encoded: &[u8]) -> Result<Grant, GrantError> {
    let value: Value = ciborium::from_reader(encoded)
        .ok()
        .ok_or_else(|| malformed("the bytes are not one CBOR item"))?;
    let grant = read_grant(value)?;
    if encode_grant(&grant) != encoded {
        return Err(GrantError::GrantNotCanonical);
    }
    Ok(grant)
}
