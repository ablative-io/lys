//! Definite canonical arrays: version, kind, then each member in order. A
//! creation's hash is SHA-256 of exactly these bytes.

use super::{
    Approved, Created, Executed, ProductDraftEvent, Refused, RefusedOnExecution, Target, VERSION,
};
use crate::ServiceAccountId;
use crate::encoding::{
    MAJOR_ARRAY, actor, as_bytes, as_text, as_uint, bytes, cbor, decode_actor, head, malformed,
    text, uint,
};
use crate::grants::{GrantId, Mode};
use crate::{AgentId, ConnectorId, IdentityError, IdentityId, OperationId, PersonId};
use ciborium::Value;
use std::str::FromStr;
use std::sync::Arc;

const CREATED: u64 = 0;
const APPROVED: u64 = 1;
const REFUSED: u64 = 2;
const EXECUTED: u64 = 3;
const REFUSED_ON_EXECUTION: u64 = 4;

fn mode_number(mode: Mode) -> u64 {
    match mode {
        Mode::Outright => 0,
        Mode::ByDraft => 1,
        Mode::ByTwo => 2,
    }
}

fn opening(out: &mut Vec<u8>, fields: u64, kind: u64, operation: OperationId) {
    head(out, MAJOR_ARRAY, fields);
    uint(out, VERSION);
    uint(out, kind);
    text(out, &operation.to_string());
}

/// The exact canonical COSE payload of `event`.
pub fn encode(event: &ProductDraftEvent) -> Vec<u8> {
    let mut out = Vec::new();
    match event {
        ProductDraftEvent::Created(value) => {
            opening(&mut out, 13, CREATED, value.operation);
            text(&mut out, &value.client_operation);
            text(&mut out, &value.holder.to_string());
            text(&mut out, &value.responsible.to_string());
            uint(&mut out, value.recorded_at);
            text(&mut out, &value.app);
            text(&mut out, &value.grant.to_string());
            uint(&mut out, mode_number(value.mode));
            head(&mut out, MAJOR_ARRAY, 3);
            text(&mut out, &value.target.kind);
            text(&mut out, &value.target.id);
            text(&mut out, &value.target.action);
            bytes(&mut out, &value.request_digest);
            text(&mut out, &value.words);
        }
        ProductDraftEvent::Approved(value) => {
            opening(&mut out, 8, APPROVED, value.operation);
            actor(&mut out, &value.actor);
            text(&mut out, &value.approver.to_string());
            uint(&mut out, value.recorded_at);
            text(&mut out, &value.draft.to_string());
            bytes(&mut out, &value.draft_hash);
        }
        ProductDraftEvent::Refused(value) => {
            opening(&mut out, 9, REFUSED, value.operation);
            actor(&mut out, &value.actor);
            text(&mut out, &value.approver.to_string());
            uint(&mut out, value.recorded_at);
            text(&mut out, &value.draft.to_string());
            bytes(&mut out, &value.draft_hash);
            text(&mut out, &value.reason);
        }
        ProductDraftEvent::Executed(value) => {
            opening(&mut out, 8, EXECUTED, value.operation);
            uint(&mut out, value.recorded_at);
            text(&mut out, &value.draft.to_string());
            bytes(&mut out, &value.draft_hash);
            text(&mut out, &value.app);
            bytes(&mut out, &value.receipt_digest);
        }
        ProductDraftEvent::RefusedOnExecution(value) => {
            opening(&mut out, 9, REFUSED_ON_EXECUTION, value.operation);
            uint(&mut out, value.recorded_at);
            text(&mut out, &value.draft.to_string());
            bytes(&mut out, &value.draft_hash);
            text(&mut out, &value.app);
            text(&mut out, &value.refusal);
            text(&mut out, &value.reason);
        }
    }
    out
}

fn tuple<const N: usize>(value: Value) -> Result<[Value; N], IdentityError> {
    let Value::Array(items) = value else {
        return Err(malformed("a product draft field is a definite array"));
    };
    items
        .try_into()
        .ok()
        .ok_or_else(|| malformed("a product draft array has the wrong number of fields"))
}

fn operation(value: Value) -> Result<OperationId, IdentityError> {
    as_text(value, "a product draft operation is canonical text")?.parse()
}

fn digest(value: Value) -> Result<[u8; 32], IdentityError> {
    as_bytes(value, "a product draft digest is 32 bytes")?
        .try_into()
        .ok()
        .ok_or_else(|| malformed("a product draft digest is 32 bytes"))
}

fn person(value: Value) -> Result<PersonId, IdentityError> {
    as_text(value, "a person is canonical text")?.parse()
}

fn identity(value: Value) -> Result<IdentityId, IdentityError> {
    let value = as_text(value, "a holder is canonical identity text")?;
    match value.split_once('-').map(|(prefix, _)| prefix) {
        Some("person") => PersonId::from_str(&value).map(IdentityId::Person),
        Some("agent") => AgentId::from_str(&value).map(IdentityId::Agent),
        Some("connector") => ConnectorId::from_str(&value).map(IdentityId::Connector),
        Some("op") => ServiceAccountId::from_str(&value).map(IdentityId::ServiceAccount),
        _ => Err(IdentityError::IdentifierMalformed {
            kind: "person, agent, connector or service account",
            text: value,
        }),
    }
}

fn mode(value: &Value) -> Result<Mode, IdentityError> {
    match as_uint(value, "a grant mode is unsigned")? {
        1 => Ok(Mode::ByDraft),
        2 => Ok(Mode::ByTwo),
        _ => Err(malformed("a product draft's mode is by draft or by two")),
    }
}

fn created(value: Value) -> Result<ProductDraftEvent, IdentityError> {
    let [
        _,
        _,
        op,
        client,
        holder,
        responsible,
        at,
        app,
        grant,
        held,
        target,
        request,
        words,
    ] = tuple::<13>(value)?;
    let [kind, id, action] = tuple::<3>(target)?;
    let grant = as_text(grant, "a grant is canonical text")?;
    Ok(ProductDraftEvent::Created(Arc::new(Created {
        operation: operation(op)?,
        client_operation: as_text(client, "a product operation is text")?,
        holder: identity(holder)?,
        responsible: person(responsible)?,
        recorded_at: as_uint(&at, "a creation time is seconds")?,
        app: as_text(app, "an app is text")?,
        grant: GrantId::from_str(&grant)
            .map_err(|_malformed| malformed("a product draft's grant is a grant id"))?,
        mode: mode(&held)?,
        target: Target {
            kind: as_text(kind, "a target kind is text")?,
            id: as_text(id, "a target id is text")?,
            action: as_text(action, "a target action is text")?,
        },
        request_digest: digest(request)?,
        words: as_text(words, "a product draft's words are text")?,
    })))
}

fn approved(value: Value) -> Result<ProductDraftEvent, IdentityError> {
    let [_, _, op, by, approver, at, draft, hash] = tuple::<8>(value)?;
    Ok(ProductDraftEvent::Approved(Arc::new(Approved {
        operation: operation(op)?,
        actor: decode_actor(by, 2)?,
        approver: person(approver)?,
        recorded_at: as_uint(&at, "a decision time is seconds")?,
        draft: operation(draft)?,
        draft_hash: digest(hash)?,
    })))
}

fn refused(value: Value) -> Result<ProductDraftEvent, IdentityError> {
    let [_, _, op, by, approver, at, draft, hash, reason] = tuple::<9>(value)?;
    Ok(ProductDraftEvent::Refused(Arc::new(Refused {
        operation: operation(op)?,
        actor: decode_actor(by, 2)?,
        approver: person(approver)?,
        recorded_at: as_uint(&at, "a decision time is seconds")?,
        draft: operation(draft)?,
        draft_hash: digest(hash)?,
        reason: as_text(reason, "a refusal's reason is text")?,
    })))
}

fn executed(value: Value) -> Result<ProductDraftEvent, IdentityError> {
    let [_, _, op, at, draft, hash, app, receipt] = tuple::<8>(value)?;
    Ok(ProductDraftEvent::Executed(Arc::new(Executed {
        operation: operation(op)?,
        recorded_at: as_uint(&at, "a close time is seconds")?,
        draft: operation(draft)?,
        draft_hash: digest(hash)?,
        app: as_text(app, "an app is text")?,
        receipt_digest: digest(receipt)?,
    })))
}

fn refused_on_execution(value: Value) -> Result<ProductDraftEvent, IdentityError> {
    let [_, _, op, at, draft, hash, app, refusal, reason] = tuple::<9>(value)?;
    Ok(ProductDraftEvent::RefusedOnExecution(Arc::new(
        RefusedOnExecution {
            operation: operation(op)?,
            recorded_at: as_uint(&at, "a close time is seconds")?,
            draft: operation(draft)?,
            draft_hash: digest(hash)?,
            app: as_text(app, "an app is text")?,
            refusal: as_text(refusal, "a refusal's name is text")?,
            reason: as_text(reason, "a refusal's reason is text")?,
        },
    )))
}

/// Read only the canonical encoding of a valid product draft payload.
pub fn decode(body: &[u8]) -> Result<ProductDraftEvent, IdentityError> {
    let value = cbor(body, "a product draft payload is a definite array")?;
    let Value::Array(items) = &value else {
        return Err(malformed("a product draft payload is a definite array"));
    };
    let [version, kind, ..] = items.as_slice() else {
        return Err(malformed(
            "a product draft payload lacks its version and kind",
        ));
    };
    if as_uint(version, "a product draft version is unsigned")? != VERSION {
        return Err(malformed("the product draft version is not supported"));
    }
    let event = match as_uint(kind, "a product draft kind is unsigned")? {
        CREATED => created(value)?,
        APPROVED => approved(value)?,
        REFUSED => refused(value)?,
        EXECUTED => executed(value)?,
        REFUSED_ON_EXECUTION => refused_on_execution(value)?,
        _ => return Err(malformed("the product draft kind is not supported")),
    };
    event.validate()?;
    if encode(&event) != body {
        return Err(IdentityError::EventNotCanonical);
    }
    Ok(event)
}
