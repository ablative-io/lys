//! Definite canonical arrays preserve both action bytes and original request evidence.

use super::{Approved, Created, DraftEvent, RequestEvidence, Target};
use crate::encoding::{
    MAJOR_ARRAY, actor, as_bytes, as_text, as_uint, bytes, cbor, decode_actor, head, malformed,
    text, uint,
};
use crate::{AgentId, IdentityError, IdentityId, OperationId, PersonId, ServiceAccountId};
use ciborium::Value;
use std::sync::Arc;

fn nullable_operation(out: &mut Vec<u8>, value: Option<OperationId>) {
    if let Some(value) = value {
        text(out, &value.to_string());
    } else {
        out.push(0xf6);
    }
}

fn evidence(out: &mut Vec<u8>, value: Option<&RequestEvidence>) {
    let Some(value) = value else {
        out.push(0xf6);
        return;
    };
    head(out, MAJOR_ARRAY, 7);
    text(out, &value.agent.to_string());
    text(out, &value.method);
    text(out, &value.path);
    bytes(out, &value.body);
    uint(out, value.signed_at_ms);
    text(out, &value.nonce);
    bytes(out, &value.cose_sign1);
}

/// The exact canonical COSE payload; a creation's hash is SHA-256 of these bytes.
pub fn encode(event: &DraftEvent) -> Vec<u8> {
    let mut out = Vec::new();
    match event {
        DraftEvent::Created(value) => {
            head(&mut out, MAJOR_ARRAY, 13);
            uint(&mut out, 1);
            uint(&mut out, 0);
            text(&mut out, &value.operation.to_string());
            actor(&mut out, &value.actor);
            uint(&mut out, value.recorded_at);
            head(&mut out, MAJOR_ARRAY, 3);
            text(&mut out, &value.target.kind);
            text(&mut out, &value.target.id);
            text(&mut out, &value.target.action);
            text(&mut out, &value.method);
            text(&mut out, &value.path);
            bytes(&mut out, &value.body);
            text(&mut out, &value.note);
            text(&mut out, &value.reviewer.to_string());
            nullable_operation(&mut out, value.corrects);
            evidence(&mut out, value.evidence.as_ref());
        }
        DraftEvent::Approved(value) => {
            head(&mut out, MAJOR_ARRAY, 9);
            uint(&mut out, 1);
            uint(&mut out, 1);
            text(&mut out, &value.operation.to_string());
            actor(&mut out, &value.actor);
            uint(&mut out, value.recorded_at);
            text(&mut out, &value.draft.to_string());
            bytes(&mut out, &value.draft_hash);
            text(&mut out, &value.application.to_string());
            evidence(&mut out, value.evidence.as_ref());
        }
    }
    out
}

fn tuple<const N: usize>(value: Value) -> Result<[Value; N], IdentityError> {
    let Value::Array(items) = value else {
        return Err(malformed("a draft field is a definite array"));
    };
    items
        .try_into()
        .ok()
        .ok_or_else(|| malformed("a draft array has the wrong number of fields"))
}

fn operation(value: Value) -> Result<OperationId, IdentityError> {
    as_text(value, "a draft operation is canonical text")?.parse()
}

fn identity(value: Value) -> Result<IdentityId, IdentityError> {
    let value = as_text(value, "a reviewer is canonical identity text")?;
    if value.starts_with("person-") {
        return value.parse::<PersonId>().map(IdentityId::Person);
    }
    if value.starts_with("agent-") {
        return value.parse::<AgentId>().map(IdentityId::Agent);
    }
    value
        .parse::<ServiceAccountId>()
        .map(IdentityId::ServiceAccount)
}

fn read_evidence(value: Value) -> Result<Option<RequestEvidence>, IdentityError> {
    if value == Value::Null {
        return Ok(None);
    }
    let [agent, method, path, body, at, nonce, cose] = tuple::<7>(value)?;
    Ok(Some(RequestEvidence {
        agent: as_text(agent, "an evidence agent is canonical text")?.parse()?,
        method: as_text(method, "a request method is text")?,
        path: as_text(path, "a request path is text")?,
        body: as_bytes(body, "a request body is bytes")?,
        signed_at_ms: as_uint(&at, "a request timestamp is milliseconds")?,
        nonce: as_text(nonce, "a request nonce is text")?,
        cose_sign1: as_bytes(cose, "a request signature is COSE bytes")?,
    }))
}

fn created(value: Value) -> Result<DraftEvent, IdentityError> {
    let [
        version,
        kind,
        op,
        by,
        at,
        target,
        method,
        path,
        body,
        note,
        reviewer,
        corrects,
        proof,
    ] = tuple::<13>(value)?;
    if as_uint(&version, "a draft version is unsigned")? != 1
        || as_uint(&kind, "a draft kind is unsigned")? != 0
    {
        return Err(malformed("a creation is draft version 1 kind 0"));
    }
    let [kind, id, action] = tuple::<3>(target)?;
    Ok(DraftEvent::Created(Arc::new(Created {
        operation: operation(op)?,
        actor: decode_actor(by, 2)?,
        recorded_at: as_uint(&at, "a creation time is seconds")?,
        target: Target {
            kind: as_text(kind, "a target kind is text")?,
            id: as_text(id, "a target id is text")?,
            action: as_text(action, "a target action is text")?,
        },
        method: as_text(method, "a mutation method is text")?,
        path: as_text(path, "a mutation path is text")?,
        body: as_bytes(body, "a mutation body is bytes")?,
        note: as_text(note, "a review note is text")?,
        reviewer: identity(reviewer)?,
        corrects: if corrects == Value::Null {
            None
        } else {
            Some(operation(corrects)?)
        },
        evidence: read_evidence(proof)?,
    })))
}

fn approved(value: Value) -> Result<DraftEvent, IdentityError> {
    let [version, kind, op, by, at, draft, hash, application, proof] = tuple::<9>(value)?;
    if as_uint(&version, "a draft version is unsigned")? != 1
        || as_uint(&kind, "a draft kind is unsigned")? != 1
    {
        return Err(malformed("an approval is draft version 1 kind 1"));
    }
    Ok(DraftEvent::Approved(Arc::new(Approved {
        operation: operation(op)?,
        actor: decode_actor(by, 2)?,
        recorded_at: as_uint(&at, "a decision time is seconds")?,
        draft: operation(draft)?,
        draft_hash: as_bytes(hash, "a draft hash is 32 bytes")?
            .try_into()
            .ok()
            .ok_or_else(|| malformed("a draft hash is 32 bytes"))?,
        application: operation(application)?,
        evidence: read_evidence(proof)?,
    })))
}

/// Read only the canonical encoding of a validated draft payload.
pub fn decode(body: &[u8]) -> Result<DraftEvent, IdentityError> {
    if body.len() > crate::signer::MAX_EVENT_BYTES {
        return Err(IdentityError::EventTooLarge {
            len: body.len(),
            limit: crate::signer::MAX_EVENT_BYTES,
        });
    }
    let value = cbor(body, "a draft payload is a definite array")?;
    let Value::Array(items) = &value else {
        return Err(malformed("a draft payload is a definite array"));
    };
    let event = match items.len() {
        13 => created(value)?,
        9 => approved(value)?,
        _ => return Err(malformed("a draft payload has 13 or 9 fields")),
    };
    event.validate()?;
    if encode(&event) != body {
        return Err(IdentityError::EventNotCanonical);
    }
    Ok(event)
}
