//! Closing events extend the wire without changing historical creation bytes.
use super::{
    Arc, IdentityError, MAJOR_ARRAY, OperationId, Value, actor, as_bytes, as_text, as_uint, bytes,
    decode_actor, head, malformed, text, tuple, uint,
};
use crate::draft_event::{DraftEvent, Executed, RefusedOnExecution};

pub(super) fn executed(out: &mut Vec<u8>, e: &Executed) {
    head(out, MAJOR_ARRAY, 10);
    uint(out, 2);
    uint(out, 4);
    text(out, &e.operation.to_string());
    actor(out, &e.actor);
    uint(out, e.recorded_at);
    text(out, &e.draft.to_string());
    bytes(out, &e.draft_hash);
    text(out, &e.application.to_string());
    uint(out, u64::from(e.status));
    uint(out, e.receipt_index);
}

pub(super) fn refused(out: &mut Vec<u8>, e: &RefusedOnExecution) {
    head(out, MAJOR_ARRAY, 10);
    uint(out, 2);
    uint(out, 5);
    text(out, &e.operation.to_string());
    actor(out, &e.actor);
    uint(out, e.recorded_at);
    text(out, &e.draft.to_string());
    bytes(out, &e.draft_hash);
    text(out, &e.application.to_string());
    text(out, &e.refusal);
    text(out, &e.reason);
}

pub(super) fn read(value: Value) -> Result<DraftEvent, IdentityError> {
    let [
        version,
        kind,
        op,
        by,
        at,
        draft,
        hash,
        application,
        one,
        two,
    ] = tuple::<10>(value)?;
    if as_uint(&version, "a close version is unsigned")? != 2 {
        return Err(malformed("a close is draft version 2"));
    }
    let operation = operation_from(op)?;
    let actor = decode_actor(by, 2)?;
    let recorded_at = as_uint(&at, "a closing time is seconds")?;
    let draft = operation_from(draft)?;
    let draft_hash = as_bytes(hash, "a draft hash is bytes")?
        .try_into()
        .map_err(|_| malformed("a draft hash is 32 bytes"))?;
    let application = operation_from(application)?;
    match as_uint(&kind, "a close kind is unsigned")? {
        4 => Ok(DraftEvent::Executed(Arc::new(Executed {
            operation,
            actor,
            recorded_at,
            draft,
            draft_hash,
            application,
            status: u16::try_from(as_uint(&one, "an answered status is unsigned")?)
                .map_err(|_| malformed("an answered status is u16"))?,
            receipt_index: as_uint(&two, "a receipt index is unsigned")?,
        }))),
        5 => Ok(DraftEvent::RefusedOnExecution(Arc::new(
            RefusedOnExecution {
                operation,
                actor,
                recorded_at,
                draft,
                draft_hash,
                application,
                refusal: as_text(one, "a refusal name is text")?,
                reason: as_text(two, "a refusal reason is text")?,
            },
        ))),
        _ => Err(malformed("a close is kind 4 or 5")),
    }
}
fn operation_from(value: Value) -> Result<OperationId, IdentityError> {
    super::operation(value)
}
