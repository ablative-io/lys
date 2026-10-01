#![cfg(test)]

//! DIRECTORY-077 R6: a one-time grant admits exactly one exercise.

mod support;

use std::error::Error;

use lys_identity::IdentityId;
use lys_identity::grants::{
    ExerciseRequest, GrantError, MemoryRelationships, ONE_TIME_SPENT, PassOn, RecipientKind, Route,
};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

#[test]
fn a_one_time_grant_admits_one_exercise_and_is_then_revoked_by_its_issuer() -> TestResult {
    let mut world = World::new()?;
    let (dana, agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let lent = world.request(dana, root, agent, "tern", PassOn::UseOnly, None)?;
    let lent = world.delegate(&lent)?.event.grant();
    let request = ExerciseRequest {
        caller: agent,
        route: Route::Tool,
        resource: alpha()?,
        action: actions(&["read"])?.into_iter().next().ok_or("no action")?,
    };
    let once = |grant| grant == lent;
    let now = world.now;
    let directory = world.directory.projection()?;
    let first = world
        .grants
        .check_once(directory, &request, now, None, &once)?;
    assert_eq!(first.grant, lent);
    let record = world
        .grants
        .book()
        .record(lent)
        .ok_or("the grant is kept")?;
    let revocation = record.revoked().ok_or("a spent grant is revoked")?;
    assert_eq!(revocation.reason, ONE_TIME_SPENT);
    world.reopen(MemoryRelationships::default())?;
    let directory = world.directory.projection()?;
    assert_eq!(
        world
            .grants
            .check_once(directory, &request, now, None, &once),
        Err(GrantError::Revoked {
            grant: lent.to_string()
        })
    );
    Ok(())
}

#[test]
fn a_grant_not_named_one_time_admits_every_exercise() -> TestResult {
    let mut world = World::new()?;
    let (dana, agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let lent = world.request(dana, root, agent, "tern", PassOn::UseOnly, Some(T0 + 1_000))?;
    let lent = world.delegate(&lent)?.event.grant();
    let request = ExerciseRequest {
        caller: agent,
        route: Route::Tool,
        resource: alpha()?,
        action: actions(&["read"])?.into_iter().next().ok_or("no action")?,
    };
    let now = world.now;
    assert_eq!(world.exercise(agent, "read", Route::Tool)?.grant, lent);
    let before = world.events();
    for _ in 0..2 {
        let directory = world.directory.projection()?;
        let permit = world
            .grants
            .check_once(directory, &request, now, None, &|_| false)?;
        assert_eq!(permit.grant, lent);
    }
    assert_eq!(world.events(), before + 2, "each exercise records one use");
    Ok(())
}
