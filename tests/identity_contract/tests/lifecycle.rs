//! R5: every allowed transition is one signed event, every other is refused with nothing recorded.

use std::error::Error;

use identity_contract::fixtures::{administrator, op, shown};
use identity_contract::harness::Harness;
use lys_identity::{AgentId, Change, IdentityError, IdentityId, Transition, verify_event};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn register_activate_suspend_reinstate_retire_is_five_signed_events() -> TestResult {
    let harness = Harness::new(5)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let (agent, _) = directory.register_agent(administrator()?, op(2), person, shown("A")?, 11)?;
    let agent = IdentityId::Agent(agent);
    let steps = [
        (Transition::Activate, ""),
        (Transition::Suspend, "paused for review"),
        (Transition::Reinstate, ""),
        (Transition::Retire, "replaced"),
    ];
    for (n, (transition, reason)) in (3..).zip(steps) {
        directory.transition(administrator()?, op(n), agent, transition, reason, 20)?;
    }
    let key = directory.service_key();
    let mut signed_about_agent = 0;
    for index in 0..directory.log()?.len()? {
        let leaf = directory.log()?.leaf(index)?.ok_or("leaf missing")?;
        let event = verify_event(leaf, &key)?;
        if event.event().identity() == agent {
            assert_eq!(event.event().actor(), &administrator()?);
            if let Change::Transition { from, to, .. } = event.event().change() {
                assert_ne!(from, to);
            }
            signed_about_agent += 1;
        }
    }
    assert_eq!(signed_about_agent, 5);
    let live = directory.projection()?.clone();
    drop(directory);
    assert_eq!(harness.open()?.projection()?, &live);
    Ok(())
}

#[test]
fn every_transition_outside_the_table_is_refused_and_the_log_does_not_grow() -> TestResult {
    let harness = Harness::new(5)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let (agent, _) = directory.register_agent(administrator()?, op(2), person, shown("A")?, 11)?;
    let (retired, _) =
        directory.register_agent(administrator()?, op(3), person, shown("R")?, 12)?;
    let retired = IdentityId::Agent(retired);
    directory.transition(
        administrator()?,
        op(4),
        retired,
        Transition::Activate,
        "",
        13,
    )?;
    directory.transition(
        administrator()?,
        op(5),
        retired,
        Transition::Retire,
        "done",
        14,
    )?;
    let size = directory.log()?.len()?;
    let agent = IdentityId::Agent(agent);
    let unknown = IdentityId::Agent(AgentId::from_bytes([8; 16]));
    let cases = [
        directory.transition(
            administrator()?,
            op(6),
            retired,
            Transition::Reinstate,
            "",
            15,
        ),
        directory.transition(
            administrator()?,
            op(7),
            agent,
            Transition::Suspend,
            "why",
            15,
        ),
        directory.transition(
            administrator()?,
            op(8),
            agent,
            Transition::Retire,
            "why",
            15,
        ),
        directory.transition(
            administrator()?,
            op(9),
            unknown,
            Transition::Activate,
            "",
            15,
        ),
    ];
    assert!(matches!(
        cases[0],
        Err(IdentityError::TransitionRefused { .. })
    ));
    assert!(matches!(
        cases[1],
        Err(IdentityError::TransitionRefused { .. })
    ));
    assert!(matches!(
        cases[2],
        Err(IdentityError::TransitionRefused { .. })
    ));
    assert!(matches!(
        cases[3],
        Err(IdentityError::IdentityUnknown { .. })
    ));
    assert_eq!(cases.iter().filter(|case| case.is_err()).count(), 4);
    assert_eq!(directory.log()?.len()?, size);
    Ok(())
}
