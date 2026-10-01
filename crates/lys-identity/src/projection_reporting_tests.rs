//! Reporting indexes and gaps survive snapshot decoding and event replay.

use std::error::Error;

use super::{Projection, ReportingGap, state};
use crate::encoding::{decode_body, encode_body};
use crate::{
    Actor, AgentId, AuthMethod, Change, IdentityEvent, IdentityId, LifecycleState, LoginBinding,
    OperationId, PersonId, Profile, Provenance, Transition,
};

fn apply(
    directory: &mut Projection,
    identity: IdentityId,
    change: Change,
) -> Result<(), Box<dyn Error>> {
    let index = u8::try_from(directory.operations.len())?;
    let event = IdentityEvent::new(
        OperationId::from_bytes([index; 16]),
        Actor::new(
            LoginBinding::new("https://issuer.test", "reporting")?,
            Provenance::new(AuthMethod::Oidc, 1),
        ),
        identity,
        1,
        change,
    )?;
    let bytes = encode_body(&event);
    let read = decode_body(&bytes)?;
    assert_eq!(read, event);
    assert_eq!(encode_body(&read), bytes);
    directory.apply(&read, u64::from(index))?;
    Ok(())
}

fn transition(
    directory: &mut Projection,
    id: IdentityId,
    transition: Transition,
) -> Result<(), Box<dyn Error>> {
    let from = directory.record(id).ok_or("identity missing")?.state();
    apply(
        directory,
        id,
        Change::Transition {
            transition,
            from,
            to: transition.target(from)?,
            reason: "Reporting lifecycle".to_owned(),
        },
    )
}

#[test]
fn snapshot_restores_reporting_indexes_gaps_and_shared_change_provenance()
-> Result<(), Box<dyn Error>> {
    let mut directory = Projection::new();
    let first = PersonId::from_bytes([1; 16]);
    let second = PersonId::from_bytes([2; 16]);
    for person in [first, second] {
        let identity = IdentityId::Person(person);
        apply(
            &mut directory,
            identity,
            Change::RegisterPerson {
                profile: Profile::new("Person")?,
            },
        )?;
        transition(&mut directory, identity, Transition::Activate)?;
    }
    let root = IdentityId::Agent(AgentId::from_bytes([3; 16]));
    let child = IdentityId::Agent(AgentId::from_bytes([4; 16]));
    apply(
        &mut directory,
        root,
        Change::RegisterAgent {
            responsible: first,
            profile: Profile::new("Root")?,
        },
    )?;
    transition(&mut directory, root, Transition::Activate)?;
    apply(
        &mut directory,
        child,
        Change::ReportingRegistration {
            responsible: first,
            profile: Profile::new("Child")?,
            reports_to: root,
        },
    )?;
    transition(&mut directory, child, Transition::Activate)?;
    apply(
        &mut directory,
        root,
        Change::ReportsToChanged {
            from: IdentityId::Person(first),
            to: IdentityId::Person(second),
            responsible_from: first,
            responsible_to: second,
        },
    )?;
    transition(&mut directory, root, Transition::Suspend)?;
    let encoded = state::encode(&directory);
    let mut restored = state::decode(encoded).map_err(std::io::Error::other)?;
    assert_eq!(restored, directory);
    assert_eq!(restored.agents_of(first).count(), 0);
    let agents = restored
        .agents_of(second)
        .map(|entry| entry.map(|(id, _)| *id))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(agents, vec![root, child]);
    let record = restored.record(child).ok_or("child missing")?;
    assert_eq!(record.events(), &[6, 7, 8]);
    assert_eq!(
        record.reporting_gap(),
        Some(ReportingGap {
            identity: root,
            state: LifecycleState::Suspended
        })
    );
    transition(&mut restored, root, Transition::Reinstate)?;
    assert_eq!(
        restored
            .record(child)
            .ok_or("child missing")?
            .reporting_gap(),
        None
    );
    assert_eq!(
        state::decode(state::encode(&restored)).map_err(std::io::Error::other)?,
        restored
    );
    Ok(())
}
