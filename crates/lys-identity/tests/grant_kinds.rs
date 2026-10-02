#![cfg(test)]

//! DIRECTORY-077 R6: a one-time grant admits exactly one exercise.

mod support;

use std::error::Error;

use lys_identity::IdentityId;
use lys_identity::grants::{
    ExerciseRequest, GrantError, MemoryRelationships, ONE_TIME_SPENT, PassOn, RecipientKind, Route,
    decode_grant, encode_grant,
};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

#[test]
fn a_plain_check_spends_a_one_time_grant_and_the_second_exercise_is_refused() -> TestResult {
    let mut world = World::new()?;
    let (dana, agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let asked = world.request(dana, root, agent, "tern", PassOn::UseOnly, Some(T0 + 1_000))?;
    let directory = world.directory.projection()?;
    let now = world.now;
    let lent = world
        .grants
        .delegate_once(directory, &asked, now)?
        .event
        .grant();
    let kept = world.grants.book().grant(lent).ok_or("the grant is kept")?;
    assert!(kept.is_once());
    assert_eq!(decode_grant(&encode_grant(kept))?, *kept);
    assert_eq!(world.exercise(agent, "read", Route::Tool)?.grant, lent);
    let record = world
        .grants
        .book()
        .record(lent)
        .ok_or("the grant is kept")?;
    let revocation = record.revoked().ok_or("a spent grant is revoked")?;
    assert_eq!(revocation.reason, ONE_TIME_SPENT);
    // The use that spends it is the revocation: one append, so no failure
    // between two appends can leave it spendable.
    let lys_identity::grants::LastUse::Seen { index, .. } = record.last_use() else {
        return Err("the use is recorded".into());
    };
    assert_eq!(revocation.index, index, "spent by the same leaf as its use");
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        world.exercise(agent, "read", Route::Tool),
        Err(GrantError::Revoked {
            grant: lent.to_string()
        })
    );
    Ok(())
}

#[test]
fn a_one_time_grant_cannot_be_passed_on() -> TestResult {
    let mut world = World::new()?;
    let (dana, agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let asked = world.request(
        dana,
        root,
        agent,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let directory = world.directory.projection()?;
    let now = world.now;
    assert!(matches!(
        world.grants.delegate_once(directory, &asked, now),
        Err(GrantError::UseOnly { .. })
    ));
    Ok(())
}

#[test]
fn an_ordinary_grant_admits_every_exercise() -> TestResult {
    let mut world = World::new()?;
    let (dana, agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let asked = world.request(dana, root, agent, "tern", PassOn::UseOnly, None)?;
    let lent = world.delegate(&asked)?.event.grant();
    let request = ExerciseRequest {
        caller: agent,
        route: Route::Tool,
        resource: alpha()?,
        action: actions(&["read"])?.into_iter().next().ok_or("no action")?,
    };
    let before = world.events();
    let now = world.now;
    for _ in 0..2 {
        let directory = world.directory.projection()?;
        assert_eq!(
            world.grants.check(directory, &request, now, None)?.grant,
            lent
        );
    }
    assert_eq!(world.events(), before + 2, "each exercise records one use");
    Ok(())
}
