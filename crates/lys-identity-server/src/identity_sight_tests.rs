#![cfg(test)]
//! Visibility follows recorded identity and responsibility, not names or state.

use std::collections::BTreeSet;

use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, IdentityEvent, IdentityId, LifecycleState, LoginBinding,
    OperationId, PersonId, Profile, Provenance, ServiceAccountId, Transition,
};

use super::visible;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn person(byte: u8) -> PersonId {
    PersonId::from_bytes([byte; 16])
}

fn apply(directory: &mut Projection, serial: u8, id: IdentityId, change: Change) -> TestResult {
    let event = IdentityEvent::new(
        OperationId::from_bytes([serial; 16]),
        Actor::new(
            LoginBinding::new("https://issuer.example", "admin")?,
            Provenance::new(AuthMethod::Oidc, 1),
        ),
        id,
        u64::from(serial),
        change,
    )?;
    directory.apply(&event, u64::from(serial))?;
    Ok(())
}

fn agent(byte: u8) -> IdentityId {
    IdentityId::Agent(AgentId::from_bytes([byte; 16]))
}

fn fixture() -> TestResult<Projection> {
    let mut directory = Projection::new();
    for (serial, person) in [(1, person(1)), (2, person(2))] {
        apply(
            &mut directory,
            serial,
            IdentityId::Person(person),
            Change::RegisterPerson {
                profile: Profile::new("Same display name")?,
            },
        )?;
    }
    for (serial, person) in [(3, person(1)), (4, person(2))] {
        apply(
            &mut directory,
            serial,
            agent(serial),
            Change::RegisterAgent {
                responsible: person,
                profile: Profile::new("Same display name")?,
            },
        )?;
    }
    Ok(directory)
}

fn ids(values: &[IdentityId]) -> BTreeSet<String> {
    values.iter().map(ToString::to_string).collect()
}

#[test]
fn each_person_sees_self_and_own_agents_despite_identical_names() -> TestResult {
    let directory = fixture()?;
    assert_eq!(
        visible(&directory, person(1), false),
        ids(&[IdentityId::Person(person(1)), agent(3)])
    );
    assert_eq!(
        visible(&directory, person(2), false),
        ids(&[IdentityId::Person(person(2)), agent(4)])
    );
    assert_eq!(
        visible(&directory, person(1), true),
        ids(&[
            IdentityId::Person(person(1)),
            IdentityId::Person(person(2)),
            agent(3),
            agent(4),
        ])
    );
    // A later non-administrator evaluation must not reuse the broader result.
    assert_eq!(
        visible(&directory, person(1), false),
        ids(&[IdentityId::Person(person(1)), agent(3)])
    );
    Ok(())
}

#[test]
fn lifecycle_changes_do_not_silently_hide_records() -> TestResult {
    let mut directory = fixture()?;
    let before = visible(&directory, person(1), false);
    for (serial, transition, from, to) in [
        (
            5,
            Transition::Activate,
            LifecycleState::Registered,
            LifecycleState::Active,
        ),
        (
            6,
            Transition::Suspend,
            LifecycleState::Active,
            LifecycleState::Suspended,
        ),
        (
            7,
            Transition::Retire,
            LifecycleState::Suspended,
            LifecycleState::Retired,
        ),
    ] {
        apply(
            &mut directory,
            serial,
            agent(3),
            Change::Transition {
                transition,
                from,
                to,
                reason: "recorded lifecycle change".to_owned(),
            },
        )?;
        assert_eq!(visible(&directory, person(1), false), before);
    }
    Ok(())
}

#[test]
fn includes_service_records_only_if_the_supplied_projection_contains_them() -> TestResult {
    let mut directory = fixture()?;
    let base = visible(&directory, person(1), false);
    let account = ServiceAccountId::from_bytes([8; 16]);
    directory.service_account(
        account,
        person(1),
        Profile::new("Same display name")?,
        true,
        LoginBinding::new("https://issuer.example", "admin")?,
    )?;
    let mut expected = base;
    expected.insert(IdentityId::ServiceAccount(account).to_string());
    assert_eq!(visible(&directory, person(1), false), expected);
    assert!(
        !visible(&directory, person(2), false)
            .contains(&IdentityId::ServiceAccount(account).to_string())
    );
    assert!(
        visible(&directory, person(2), true)
            .contains(&IdentityId::ServiceAccount(account).to_string())
    );
    Ok(())
}
