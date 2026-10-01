//! Reporting changes extend the event codes without changing earlier encodings.

use super::{
    AgentId, Change, IdentityError, IdentityId, PersonId, Profile, Value, as_id, as_uint, bytes,
    decode_profile, fields, malformed, map, profile, uint, wire,
};

fn target(out: &mut Vec<u8>, value: IdentityId) {
    let (kind, id) = match value {
        IdentityId::Person(id) => (wire::PERSON, *id.as_bytes()),
        IdentityId::Agent(id) => (wire::AGENT, *id.as_bytes()),
        IdentityId::ServiceAccount(id) => (wire::SERVICE_ACCOUNT, *id.as_bytes()),
    };
    map(out, 2);
    uint(out, 1);
    uint(out, kind);
    uint(out, 2);
    bytes(out, &id);
}

pub(super) fn registration(
    out: &mut Vec<u8>,
    responsible: PersonId,
    shown: &Profile,
    reports_to: IdentityId,
) {
    map(out, 3);
    uint(out, 1);
    bytes(out, responsible.as_bytes());
    uint(out, 2);
    profile(out, shown);
    uint(out, 3);
    target(out, reports_to);
}

pub(super) fn changed(
    out: &mut Vec<u8>,
    from: IdentityId,
    to: IdentityId,
    responsible_from: PersonId,
    responsible_to: PersonId,
) {
    map(out, 4);
    uint(out, 1);
    target(out, from);
    uint(out, 2);
    target(out, to);
    uint(out, 3);
    bytes(out, responsible_from.as_bytes());
    uint(out, 4);
    bytes(out, responsible_to.as_bytes());
}

fn read_target(value: Value) -> Result<IdentityId, IdentityError> {
    let [kind, id] = fields::<2>(value, "a reporting target has kind and id")?;
    let id = as_id(id, "a reporting target id is 16 bytes")?;
    match as_uint(&kind, "a reporting target kind is a code")? {
        wire::PERSON => Ok(IdentityId::Person(PersonId::from_bytes(id))),
        wire::AGENT => Ok(IdentityId::Agent(AgentId::from_bytes(id))),
        _ => Err(malformed("a reporting target is a person or agent")),
    }
}

pub(super) fn decode(kind: u64, value: Value) -> Result<Change, IdentityError> {
    match kind {
        wire::REPORTING_REGISTRATION => {
            let [responsible, shown, reports_to] =
                fields::<3>(value, "a reporting registration has three fields")?;
            Ok(Change::ReportingRegistration {
                responsible: PersonId::from_bytes(as_id(
                    responsible,
                    "a responsible person is 16 bytes",
                )?),
                profile: decode_profile(shown)?,
                reports_to: read_target(reports_to)?,
            })
        }
        wire::REPORTS_TO_CHANGED => {
            let [from, to, responsible_from, responsible_to] =
                fields::<4>(value, "a reporting change has four fields")?;
            Ok(Change::ReportsToChanged {
                from: read_target(from)?,
                to: read_target(to)?,
                responsible_from: PersonId::from_bytes(as_id(
                    responsible_from,
                    "a responsible person is 16 bytes",
                )?),
                responsible_to: PersonId::from_bytes(as_id(
                    responsible_to,
                    "a responsible person is 16 bytes",
                )?),
            })
        }
        _ => Err(malformed(
            "a reporting change is registration or reassignment",
        )),
    }
}
