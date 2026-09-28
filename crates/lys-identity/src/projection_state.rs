//! The directory projection as a snapshot holds it.
//!
//! Every record, every operation with the index it made, and every link-audit
//! source with its index. The login indexes are not written: each is the
//! bindings its records hold, and is rebuilt from them on reading exactly as
//! [`Projection::apply`] builds it.

use ciborium::Value;

use super::{Projection, Record};
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
    ] = tuple::<7>(value, "a directory record")?;
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
    ])
}

/// The projection a state value holds, with its login indexes rebuilt.
pub(crate) fn decode(value: Value) -> Result<Projection, Unreadable> {
    let [records, operations, link_sources] = tuple::<3>(value, "a directory projection")?;
    let mut projection = Projection::new();
    for held in list(records, "the directory records")? {
        let (id, record) = read_record(held)?;
        for bound in &record.bindings {
            let taken = match id {
                IdentityId::Person(person) => {
                    projection.bindings.insert(bound.clone(), person).is_some()
                }
                IdentityId::Agent(agent) => projection
                    .agent_bindings
                    .insert(bound.clone(), agent)
                    .is_some(),
            };
            if taken {
                return Err(format!("a login is bound twice, the second time to {id}"));
            }
        }
        if projection.records.insert(id, record).is_some() {
            return Err(format!("{id} is recorded twice"));
        }
    }
    projection.operations = read_indexed(operations, "an operation", read_operation)?
        .into_iter()
        .collect();
    projection.link_sources = read_indexed(link_sources, "a link-audit source", |source| {
        read_text(source, "a link-audit source")
    })?
    .into_iter()
    .collect();
    Ok(projection)
}
