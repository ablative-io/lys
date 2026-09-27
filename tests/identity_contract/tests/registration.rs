//! R1: people and agents registered with enduring ids, each agent under its responsible person.

use std::error::Error;

use identity_contract::fixtures::{administrator, op, shown};
use identity_contract::harness::Harness;
use lys_identity::{IdentityError, IdentityId, LifecycleState, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_person_and_an_agent_keep_their_ids_and_history_across_a_reopen() -> TestResult {
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let (agent, receipt) =
        directory.register_agent(administrator()?, op(2), person, shown("Builder")?, 11)?;
    directory.change_profile(
        administrator()?,
        op(3),
        IdentityId::Agent(agent),
        shown("Builder two")?,
        12,
    )?;
    let live = directory.projection()?.clone();
    drop(directory);

    let mut reopened = harness.open()?;
    assert_eq!(
        reopened.projection()?,
        &live,
        "the projection after reopen equals replay"
    );
    let record = reopened
        .record(IdentityId::Agent(agent))?
        .ok_or("agent missing")?;
    assert_eq!(record.responsible(), Some(person));
    assert_eq!(record.profile().display_name(), "Builder two");
    assert_eq!(record.state(), LifecycleState::Registered);
    assert_eq!(record.events().len(), 2);
    assert_eq!(receipt.identity(), IdentityId::Agent(agent));
    assert_eq!(reopened.log().len(), 3);
    Ok(())
}

#[test]
fn an_agent_needs_a_registered_responsible_person_and_nothing_is_recorded_otherwise() -> TestResult
{
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let nobody = PersonId::from_bytes([9; 16]);
    let refused = directory.register_agent(administrator()?, op(1), nobody, shown("Orphan")?, 10);
    assert!(matches!(
        refused,
        Err(IdentityError::IdentityUnknown { .. })
    ));
    assert_eq!(directory.log().len(), 0);
    Ok(())
}

#[test]
fn the_same_operation_answers_once_and_a_different_request_under_it_is_refused() -> TestResult {
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let first = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let again = directory.register_person(administrator()?, op(1), shown("Ada")?, 99)?;
    assert_eq!(first, again);
    assert_eq!(directory.log().len(), 1);
    let other = directory.register_person(administrator()?, op(1), shown("Grace")?, 10);
    assert!(matches!(other, Err(IdentityError::OperationReused { .. })));
    assert_eq!(directory.log().len(), 1);
    Ok(())
}
