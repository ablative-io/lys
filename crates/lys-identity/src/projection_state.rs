//! The directory projection as a snapshot holds it.
//!
//! Every record, every operation with the index it made, and every link-audit
//! source with its index. The login indexes are not written: each is the
//! bindings its records hold, and is rebuilt from them on reading exactly as
//! [`Projection::apply`] builds it.

use std::sync::Arc;

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
    let mut parts = vec![
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
        super::draft::state::encode(projection),
    ];
    // Product drafts follow only when one is held, so a snapshot of a
    // directory without them keeps its earlier bytes and readers.
    if !projection.product_drafts.is_empty() {
        parts.push(super::product_draft::state::encode(projection));
    }
    array(parts)
}

/// The projection a state value holds, with its login indexes rebuilt.
pub(crate) fn decode(value: Value) -> Result<Projection, Unreadable> {
    let mut parts = list(value, "a directory projection")?;
    let product_drafts = if parts.len() == 5 { parts.pop() } else { None };
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
                IdentityId::Connector(_) => {
                    return Err("a connector cannot hold login bindings".to_owned());
                }
            };
            if taken {
                return Err(format!("a login is bound twice, the second time to {id}"));
            }
        }
        match id {
            IdentityId::Person(person) => {
                std::sync::Arc::make_mut(&mut projection.people).insert(id.to_string(), person);
            }
            // Only a person is indexed by name; the reporting rebuild below
            // refuses a kind the directory never stores.
            IdentityId::Agent(_) | IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => {}
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
    super::draft::state::decode(&mut projection, drafts)?;
    if let Some(product_drafts) = product_drafts {
        super::product_draft::state::decode(&mut projection, product_drafts)?;
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
            IdentityId::Connector(_) => {
                return Err("a connector is not a stored directory identity".to_owned());
            }
        };
        let mut fields = Vec::from(fields);
        fields.push(target);
        migrated.push(array(fields));
    }
    decode(array(vec![array(migrated), operations, link_sources]))
}
