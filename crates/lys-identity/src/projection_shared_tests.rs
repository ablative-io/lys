//! Request snapshots share every index and retain their prior records after a write.

use std::error::Error;
use std::sync::Arc;

use super::Projection;
use crate::{
    Actor, AuthMethod, Change, IdentityEvent, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance,
};

#[test]
fn request_snapshots_share_every_directory_index_and_isolate_later_writes()
-> Result<(), Box<dyn Error>> {
    let mut directory = Projection::new();
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "owner")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let first = IdentityId::Person(PersonId::from_bytes([1; 16]));
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([1; 16]),
            actor.clone(),
            first,
            1,
            Change::SetupPerson {
                profile: Profile::new("owner")?,
            },
        )?,
        0,
    )?;
    let view = directory.shared();
    assert!(Arc::ptr_eq(&directory.records, &view.records));
    assert!(Arc::ptr_eq(&directory.people, &view.people));
    assert!(Arc::ptr_eq(
        &directory.agents_by_person,
        &view.agents_by_person
    ));
    assert!(Arc::ptr_eq(
        &directory.reporting_children,
        &view.reporting_children
    ));
    assert!(Arc::ptr_eq(&directory.bindings, &view.bindings));
    assert!(Arc::ptr_eq(&directory.agent_bindings, &view.agent_bindings));
    assert!(Arc::ptr_eq(&directory.operations, &view.operations));
    assert!(Arc::ptr_eq(&directory.link_sources, &view.link_sources));
    assert!(Arc::ptr_eq(&directory.accounts, &view.accounts));
    let second = IdentityId::Person(PersonId::from_bytes([2; 16]));
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([2; 16]),
            actor,
            second,
            2,
            Change::RegisterPerson {
                profile: Profile::new("other")?,
            },
        )?,
        1,
    )?;
    assert!(directory.record(second).is_some());
    assert!(view.record(second).is_none());
    assert_eq!(view.records().count(), 1);
    assert_eq!(view.operation(OperationId::from_bytes([2; 16])), None);
    Ok(())
}

/// The directory state written at main 5215ed6c from the inputs below, a
/// person's record and an agent's, never regenerated (DIRECTORY-080 R1). It
/// is pinned here because the state value's writer is private to the crate.
const DIRECTORY_STATE: &str = include_str!("../tests/fixtures/directory-state-5215ed6c.hex");
/// The lowercase hexadecimal digits.
const DIGITS: &[u8; 16] = b"0123456789abcdef";

#[test]
fn a_directory_state_of_a_person_and_an_agent_keeps_its_bytes() -> Result<(), Box<dyn Error>> {
    let mut directory = Projection::new();
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "owner")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let person = PersonId::from_bytes([0xd1; 16]);
    let agent = IdentityId::Agent(crate::AgentId::from_bytes([0xa7; 16]));
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([1; 16]),
            actor.clone(),
            IdentityId::Person(person),
            1,
            Change::SetupPerson {
                profile: Profile::new("owner")?,
            },
        )?,
        0,
    )?;
    directory.apply(
        &IdentityEvent::new(
            OperationId::from_bytes([2; 16]),
            actor,
            agent,
            2,
            Change::RegisterAgent {
                responsible: person,
                profile: Profile::new("helper")?,
            },
        )?,
        1,
    )?;
    let mut written = String::new();
    for byte in crate::directory_state::encode(&directory, 2)? {
        written.push(char::from(DIGITS[usize::from(byte >> 4)]));
        written.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    let fixture = DIRECTORY_STATE.trim();
    assert_eq!(
        fixture, written,
        "today's writer no longer writes the fixture's directory state"
    );
    let bytes = (0..fixture.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&fixture[at..at + 2], 16))
        .collect::<Result<Vec<u8>, _>>()?;
    let read = crate::directory_state::decode(&bytes, 2)?;
    for id in [IdentityId::Person(person), agent] {
        assert_eq!(
            read.record(id),
            directory.record(id),
            "{id}: today's reader"
        );
    }
    Ok(())
}
