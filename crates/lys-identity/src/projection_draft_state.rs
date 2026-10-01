//! Snapshots retain terminal decisions and both sides of every correction link.

use super::DraftRecord;
use crate::draft_event::{self, DraftEvent};
use crate::projection::Projection;
use crate::state_value::{
    Unreadable, array, bytes, list, nullable, operation, read_nullable, read_operation, read_uint,
    tuple, uint,
};
use ciborium::Value;
use std::sync::Arc;

pub(in crate::projection) fn encode(projection: &Projection) -> Value {
    array(
        projection
            .drafts
            .values()
            .map(|record| {
                array(vec![
                    bytes(&draft_event::encode(&DraftEvent::Created(Arc::clone(
                        &record.created,
                    )))),
                    uint(record.created_index),
                    nullable(record.decision().map(|(decision, index)| {
                        array(vec![bytes(&draft_event::encode(&decision)), uint(index)])
                    })),
                    nullable(record.replacement_of.map(operation)),
                ])
            })
            .collect(),
    )
}

fn event(value: Value) -> Result<(DraftEvent, [u8; 32]), Unreadable> {
    let Value::Bytes(body) = value else {
        return Err("a stored draft event is canonical bytes".to_owned());
    };
    let hash = crate::encoding::payload_commitment(&body);
    Ok((
        draft_event::decode(&body).map_err(|error| error.to_string())?,
        hash,
    ))
}

pub(in crate::projection) fn decode(
    projection: &mut Projection,
    value: Value,
) -> Result<(), Unreadable> {
    for value in list(value, "the draft records")? {
        let mut fields = list(value, "a draft record")?;
        if fields.len() == 3 {
            fields.push(Value::Null);
        }
        let [created, index, decision, replacement] = tuple::<4>(array(fields), "a draft record")?;
        let (created, hash) = event(created)?;
        let DraftEvent::Created(created) = created else {
            return Err("a draft record starts with a creation".to_owned());
        };
        let created_index = read_uint(&index, "a draft creation index")?;
        if projection.operation(created.operation) != Some(created_index) {
            return Err("a creation disagrees with its operation index".to_owned());
        }
        let replacement_of = read_nullable(replacement).map(read_operation).transpose()?;
        let mut record = DraftRecord {
            created,
            hash,
            created_index,
            approved: None,
            refused: None,
            correction: None,
            replacement_of,
        };
        if let Some(decision) = read_nullable(decision) {
            let [value, index] = tuple::<2>(decision, "a draft decision")?;
            let (decision, _) = event(value)?;
            let index = read_uint(&index, "a draft decision index")?;
            if decision.decision() != Some((record.created.operation, hash))
                || index <= created_index
                || projection.operation(decision.operation()) != Some(index)
                || replacement_of.is_some()
            {
                return Err("a draft decision disagrees with its creation".to_owned());
            }
            match decision {
                DraftEvent::Approved(value) => record.approved = Some((value, index)),
                DraftEvent::Refused(value) => record.refused = Some((value, index)),
                DraftEvent::Correction(value) => record.correction = Some((value, index)),
                DraftEvent::Created(_) => {
                    return Err("a creation is not a draft decision".to_owned());
                }
            }
        }
        if Arc::make_mut(&mut projection.drafts)
            .insert(record.created.operation, Arc::new(record))
            .is_some()
        {
            return Err("a draft is recorded twice".to_owned());
        }
    }
    for record in projection.drafts.values() {
        if let Some((correction, index)) = &record.correction {
            let own = projection
                .draft(correction.corrected.operation)
                .ok_or("a correction's own change is missing")?;
            if own.created != correction.corrected
                || own.hash != correction.corrected_hash
                || own.created_index != *index
                || own.replacement_of != Some(correction.draft)
            {
                return Err("a correction's replacement disagrees with its original".to_owned());
            }
        }
        if let Some(original) = record.replacement_of {
            let parent = projection
                .draft(original)
                .ok_or("a replacement's original is missing")?;
            if !parent.correction.as_ref().is_some_and(|(event, index)| {
                event.corrected.operation == record.created.operation
                    && *index == record.created_index
            }) {
                return Err("a replacement has no corresponding correction".to_owned());
            }
        }
    }
    Ok(())
}
