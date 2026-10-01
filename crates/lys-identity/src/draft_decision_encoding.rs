//! Terminal decisions bind the original hash and preserve a correction's ownership.

use super::{decode, encode, evidence, operation, read_evidence, tuple};
use crate::IdentityError;
use crate::draft_event::{Correction, DraftEvent, Refused};
use crate::encoding::{
    MAJOR_ARRAY, actor, as_bytes, as_text, as_uint, bytes, cbor, decode_actor, head, malformed,
    text, uint,
};
use ciborium::Value;
use std::sync::Arc;

pub(super) fn write_refused(out: &mut Vec<u8>, value: &Refused) {
    head(out, MAJOR_ARRAY, 9);
    uint(out, 2);
    uint(out, 2);
    text(out, &value.operation.to_string());
    actor(out, &value.actor);
    uint(out, value.recorded_at);
    text(out, &value.draft.to_string());
    bytes(out, &value.draft_hash);
    text(out, &value.reason);
    evidence(out, value.evidence.as_ref());
}

pub(super) fn write_correction(out: &mut Vec<u8>, value: &Correction) {
    head(out, MAJOR_ARRAY, 10);
    uint(out, 2);
    uint(out, 3);
    text(out, &value.operation.to_string());
    actor(out, &value.actor);
    uint(out, value.recorded_at);
    text(out, &value.draft.to_string());
    bytes(out, &value.draft_hash);
    text(out, &value.reason);
    bytes(
        out,
        &encode(&DraftEvent::Created(Arc::clone(&value.corrected))),
    );
    bytes(out, &value.corrected_hash);
}

fn hash(value: Value) -> Result<[u8; 32], IdentityError> {
    as_bytes(value, "a draft hash is 32 bytes")?
        .try_into()
        .ok()
        .ok_or_else(|| malformed("a draft hash is 32 bytes"))
}

pub(super) fn refused(value: Value) -> Result<DraftEvent, IdentityError> {
    let [version, kind, op, by, at, draft, commitment, reason, proof] = tuple::<9>(value)?;
    if as_uint(&version, "a refusal version is unsigned")? != 2
        || as_uint(&kind, "a refusal kind is unsigned")? != 2
    {
        return Err(malformed("a refusal is draft version 2 kind 2"));
    }
    Ok(DraftEvent::Refused(Arc::new(Refused {
        operation: operation(op)?,
        actor: decode_actor(by, 2)?,
        recorded_at: as_uint(&at, "a refusal time is seconds")?,
        draft: operation(draft)?,
        draft_hash: hash(commitment)?,
        reason: as_text(reason, "a refusal reason is text")?,
        evidence: read_evidence(proof)?,
    })))
}

pub(super) fn correction(value: Value) -> Result<DraftEvent, IdentityError> {
    let [
        version,
        kind,
        op,
        by,
        at,
        draft,
        commitment,
        reason,
        corrected,
        corrected_hash,
    ] = tuple::<10>(value)?;
    if as_uint(&version, "a correction version is unsigned")? != 2
        || as_uint(&kind, "a correction kind is unsigned")? != 3
    {
        return Err(malformed("a correction is draft version 2 kind 3"));
    }
    let corrected = as_bytes(corrected, "the corrected change is canonical bytes")?;
    let value = cbor(&corrected, "the corrected change is a creation")?;
    if !matches!(&value, Value::Array(items) if items.len() >= 2 && as_uint(&items[1], "a creation kind is unsigned")? == 0)
    {
        return Err(malformed("a correction contains only a creation payload"));
    }
    let DraftEvent::Created(corrected) = decode(&corrected)? else {
        return Err(malformed("a correction contains a creation payload"));
    };
    Ok(DraftEvent::Correction(Arc::new(Correction {
        operation: operation(op)?,
        actor: decode_actor(by, 2)?,
        recorded_at: as_uint(&at, "a correction time is seconds")?,
        draft: operation(draft)?,
        draft_hash: hash(commitment)?,
        reason: as_text(reason, "a correction reason is text")?,
        corrected,
        corrected_hash: hash(corrected_hash)?,
    })))
}
