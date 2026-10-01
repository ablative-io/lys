//! R5: every allowed transition is one signed event, every other is refused with nothing recorded.

use std::error::Error;

use identity_contract::fixtures::{administrator, op, shown};
use identity_contract::harness::Harness;
use lys_identity::{
    AgentId, Change, IdentityError, IdentityId, LifecycleState, Transition, verify_event,
};

type TestResult = Result<(), Box<dyn Error>>;

/// Register, activate, suspend, reinstate and retire one agent: five signed
/// events, each naming the actor, the identity, the state it moved from and
/// to, and when; the reason given travels with its transition, and the
/// projection after a reopen equals replay.
#[test]
fn register_activate_suspend_reinstate_retire_is_five_signed_events() -> TestResult {
    use LifecycleState::{Active, Registered, Retired, Suspended};
    use Transition::{Activate, Reinstate, Retire, Suspend};

    let harness = Harness::new(5)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let (agent, _) = directory.register_agent(administrator()?, op(2), person, shown("A")?, 11)?;
    let agent = IdentityId::Agent(agent);
    let steps = [
        (Activate, ""),
        (Suspend, "review"),
        (Reinstate, ""),
        (Retire, "replaced"),
    ];
    for (n, (transition, reason)) in (3..).zip(steps) {
        let at = u64::from(n) + 17;
        directory.transition(administrator()?, op(n), agent, transition, reason, at)?;
    }
    let key = directory.service_key();
    let mut seen = Vec::new();
    for index in 0..directory.log()?.len()? {
        let leaf = directory.log()?.leaf(index)?.ok_or("leaf missing")?;
        let signed = verify_event(&leaf, &key)?;
        let event = signed.event()?;
        if event.identity() != agent {
            continue;
        }
        assert_eq!(
            event.actor(),
            &administrator()?,
            "each event names its actor"
        );
        let (from, to, transition, reason) = match event.change() {
            Change::RegisterAgent { responsible, .. } => {
                assert_eq!(*responsible, person);
                (None, Registered, None, String::new())
            }
            Change::Transition {
                transition,
                from,
                to,
                reason,
            } => (Some(*from), *to, Some(*transition), reason.clone()),
            other => return Err(format!("unexpected change: {other:?}").into()),
        };
        seen.push((event.recorded_at(), from, to, transition, reason));
    }
    let expected = [
        (11, None, Registered, None, ""),
        (20, Some(Registered), Active, Some(Activate), ""),
        (21, Some(Active), Suspended, Some(Suspend), "review"),
        (22, Some(Suspended), Active, Some(Reinstate), ""),
        (23, Some(Active), Retired, Some(Retire), "replaced"),
    ];
    assert_eq!(seen.len(), 5, "five signed events about the agent");
    for (got, want) in seen.iter().zip(expected) {
        assert_eq!(got.0, want.0, "each event names when");
        assert_eq!(got.1, want.1, "each event names the state it left");
        assert_eq!(got.2, want.2, "each event names the state it entered");
        assert_eq!(got.3, want.3, "each event names its transition");
        assert_eq!(got.4, want.4, "the reason given is recorded");
    }
    let live = directory.projection()?.clone();
    let retired = directory
        .record(agent)?
        .ok_or("a retired agent stays readable")?;
    assert_eq!(retired.state(), Retired);
    assert_eq!(
        retired.events().len(),
        5,
        "its whole history stays readable"
    );
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
