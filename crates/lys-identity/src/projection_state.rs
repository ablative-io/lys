//! The directory projection as a snapshot holds it.
//!
//! Every record, every operation with the index it made, and every link-audit
//! source with its index. The login indexes are not written: each is the
//! bindings its records hold, and is rebuilt from them on reading exactly as
//! [`Projection::apply`] builds it.

use std::sync::Arc;

use ciborium::Value;

use super::draft::DraftRecord;
use super::{Projection, Record};
use crate::draft_event::DraftEvent;
use crate::event::wire;
use crate::id::IdentityId;
use crate::id::{ID_LEN, PersonId};
use crate::profile::Profile;
use crate::state_value::{
    Unreadable, array, binding, bytes, identity, indexed, list, nullable, operation, read_binding,
    read_fixed, read_identity, read_indexed, read_nullable, read_operation, read_text, read_uint,
    text, tuple, uint,
};

fn record(id: IdentityId, record: &Record) -> Value {
    array(vec![
        identity(id),
        text(record.profile.display_name()),
        uint(wire::state(record.state)),
        nullable(record.responsible.map(|person| bytes(person.as_bytes()))),
        array(record.bindings.iter().map(binding).collect()),
        binding(&record.registered_by),
        array(record.events.iter().map(|index| uint(*index)).collect()),
        nullable(record.reports_to.map(identity)),
    ])
}

fn read_record(value: Value) -> Result<(IdentityId, Record), Unreadable> {
    let [
        id,
        name,
        state,
        responsible,
        bindings,
        registered_by,
        events,
        reports_to,
    ] = tuple::<8>(value, "a directory record")?;
    let state = read_uint(&state, "a lifecycle state")?;
    let record = Record {
        profile: Profile::new(&read_text(name, "a display name")?)
            .map_err(|error| error.to_string())?,
        state: wire::state_from(state)
            .ok_or_else(|| format!("lifecycle state {state} is not a state"))?,
        responsible: read_nullable(responsible)
            .map(|person| read_fixed::<ID_LEN>(person, "a responsible person"))
            .transpose()?
            .map(PersonId::from_bytes),
        reports_to: read_nullable(reports_to).map(read_identity).transpose()?,
        reporting_gap: None,
        bindings: list(bindings, "a record's bindings")?
            .into_iter()
            .map(read_binding)
            .collect::<Result<_, _>>()?,
        registered_by: read_binding(registered_by)?,
        events: list(events, "a record's events")?
            .iter()
            .map(|index| read_uint(index, "an event index"))
            .collect::<Result<_, _>>()?,
    };
    Ok((read_identity(id)?, record))
}

/// The projection as a state value.
pub(crate) fn encode(projection: &Projection) -> Value {
    array(vec![
        array(
            projection
                .records
                .iter()
                .map(|(id, held)| record(*id, held))
                .collect(),
        ),
        indexed(
            projection
                .operations
                .iter()
                .map(|(op, index)| (*op, *index))
                .collect(),
            |op| operation(*op),
        ),
        indexed(
            projection
                .link_sources
                .iter()
                .map(|(source, index)| (source.clone(), *index))
                .collect(),
            |source| text(source),
        ),
        array(
            projection
                .drafts
                .values()
                .map(|record| {
                    array(vec![
                        bytes(&crate::draft_event::encode(&DraftEvent::Created(
                            Arc::clone(&record.created),
                        ))),
                        uint(record.created_index),
                        nullable(record.approved.as_ref().map(|(approved, index)| {
                            array(vec![
                                bytes(&crate::draft_event::encode(&DraftEvent::Approved(
                                    Arc::clone(approved),
                                ))),
                                uint(*index),
                            ])
                        })),
                    ])
                })
                .collect(),
        ),
    ])
}

/// The projection a state value holds, with its login indexes rebuilt.
pub(crate) fn decode(value: Value) -> Result<Projection, Unreadable> {
    let mut parts = list(value, "a directory projection")?;
    let drafts = if parts.len() == 4 {
        parts
            .pop()
            .ok_or_else(|| "a directory projection lacks drafts".to_owned())?
    } else {
        array(Vec::new())
    };
    let [records, operations, link_sources] = tuple::<3>(array(parts), "a directory projection")?;
    let mut projection = Projection::new();
    for held in list(records, "the directory records")? {
        let (id, record) = read_record(held)?;
        for bound in &record.bindings {
            let taken = match id {
                IdentityId::Person(person) => Arc::make_mut(&mut projection.bindings)
                    .insert(bound.clone(), person)
                    .is_some(),
                IdentityId::Agent(agent) => Arc::make_mut(&mut projection.agent_bindings)
                    .insert(bound.clone(), agent)
                    .is_some(),
                IdentityId::ServiceAccount(_) => {
                    return Err("a service account cannot hold login bindings".to_owned());
                }
            };
            if taken {
                return Err(format!("a login is bound twice, the second time to {id}"));
            }
        }
        if let IdentityId::Person(person) = id {
            std::sync::Arc::make_mut(&mut projection.people).insert(id.to_string(), person);
        }
        if Arc::make_mut(&mut projection.records)
            .insert(id, record)
            .is_some()
        {
            return Err(format!("{id} is recorded twice"));
        }
    }
    projection.operations = Arc::new(
        read_indexed(operations, "an operation", read_operation)?
            .into_iter()
            .collect(),
    );
    projection.link_sources = Arc::new(
        read_indexed(link_sources, "a link-audit source", |source| {
            read_text(source, "a link-audit source")
        })?
        .into_iter()
        .collect(),
    );
    projection
        .rebuild_reporting_indexes()
        .map_err(|error| error.to_string())?;
    for held in list(drafts, "the draft records")? {
        let [created, index, approved] = tuple::<3>(held, "a draft record")?;
        let Value::Bytes(body) = created else {
            return Err("a draft creation is canonical bytes".to_owned());
        };
        let DraftEvent::Created(created) =
            crate::draft_event::decode(&body).map_err(|error| error.to_string())?
        else {
            return Err("a draft record must begin with a creation".to_owned());
        };
        let created_index = read_uint(&index, "a draft creation index")?;
        if projection.operation(created.operation) != Some(created_index) {
            return Err("a draft creation disagrees with its operation index".to_owned());
        }
        let hash = crate::encoding::payload_commitment(&body);
        let approved = read_nullable(approved)
            .map(|value| {
                let [body, index] = tuple::<2>(value, "a draft approval")?;
                let Value::Bytes(body) = body else {
                    return Err("a draft approval is canonical bytes".to_owned());
                };
                let DraftEvent::Approved(approval) =
                    crate::draft_event::decode(&body).map_err(|error| error.to_string())?
                else {
                    return Err("a draft decision must be an approval".to_owned());
                };
                let index = read_uint(&index, "a draft approval index")?;
                if approval.draft != created.operation
                    || approval.draft_hash != hash
                    || index <= created_index
                {
                    return Err("a draft approval disagrees with its creation".to_owned());
                }
                if projection.operation(approval.operation) != Some(index) {
                    return Err("a draft approval disagrees with its operation index".to_owned());
                }
                Ok((approval, index))
            })
            .transpose()?;
        let operation = created.operation;
        if Arc::make_mut(&mut projection.drafts)
            .insert(
                operation,
                Arc::new(DraftRecord {
                    created,
                    hash,
                    created_index,
                    approved,
                }),
            )
            .is_some()
        {
            return Err("a draft is recorded twice".to_owned());
        }
    }
    Ok(projection)
}

/// Materialise the direct reporting edges of a version-two projection.
pub(crate) fn migrate_v2(value: Value) -> Result<Projection, Unreadable> {
    let [records, operations, link_sources] = tuple::<3>(value, "a directory projection")?;
    let mut migrated = Vec::new();
    for held in list(records, "the directory records")? {
        let fields = tuple::<7>(held, "a version-two directory record")?;
        let id = read_identity(fields[0].clone())?;
        let target = match id {
            IdentityId::Agent(_) => {
                let person = read_fixed::<ID_LEN>(fields[3].clone(), "a responsible person")?;
                identity(IdentityId::Person(PersonId::from_bytes(person)))
            }
            IdentityId::Person(_) => Value::Null,
            IdentityId::ServiceAccount(_) => {
                return Err("a service account is not a stored directory identity".to_owned());
            }
        };
        let mut fields = Vec::from(fields);
        fields.push(target);
        migrated.push(array(fields));
    }
    decode(array(vec![array(migrated), operations, link_sources]))
}
