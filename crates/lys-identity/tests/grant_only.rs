//! A decision held to one grant rests on that grant alone: of two grants
//! the caller holds for the same action, the named one is exercised and its
//! use recorded, and a grant the caller does not hold permits nothing even
//! while another grant would.

mod support;

use std::error::Error;

use lys_identity::IdentityId;
use lys_identity::grants::{
    Action, ExerciseRequest, GrantChange, GrantError, GrantId, PassOn, RecipientKind, Route,
};
use support::{World, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

/// A root of Dana's, and its use-only delegation to Tom.
fn lent(world: &mut World) -> Result<(GrantId, GrantId), Box<dyn Error>> {
    let dana = IdentityId::Person(world.dana);
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let request = world.request(
        dana,
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    Ok((root, world.delegate(&request)?.event.grant()))
}

fn read(world: &World) -> Result<ExerciseRequest, Box<dyn Error>> {
    Ok(ExerciseRequest {
        caller: IdentityId::Person(world.tom),
        route: Route::Tool,
        resource: alpha()?,
        action: Action::new("read")?,
    })
}

#[test]
fn a_decision_held_to_one_grant_exercises_that_grant_and_no_other() -> TestResult {
    let mut world = World::new()?;
    let (_, first) = lent(&mut world)?;
    let (dana_root, second) = lent(&mut world)?;
    let request = read(&world)?;
    let tom = IdentityId::Person(world.tom);
    let free = world.exercise(tom, "read", Route::Tool)?;
    let other = if free.grant == first { second } else { first };

    let directory = world.directory.projection()?;
    let held = world
        .grants
        .check_by(directory, &request, Some(other), world.now, None)?;
    assert_eq!(held.grant, other, "the named grant is the one exercised");
    let index = held.use_event.ok_or("check records a use")??;
    let (signed, _) = &world.grants.events()?[usize::try_from(index)?];
    assert_eq!(
        signed.event().change(),
        &GrantChange::Use {
            grant: other,
            route: Route::Tool
        },
        "the use is recorded against the named grant"
    );

    let before = world.events();
    let directory = world.directory.projection()?;
    let refused = world
        .grants
        .check_by(directory, &request, Some(dana_root), world.now, None);
    assert!(
        matches!(refused, Err(GrantError::NotHeld { .. })),
        "a grant Tom does not hold permits nothing: {refused:?}"
    );
    let directory = world.directory.projection()?;
    let explained = world
        .grants
        .explain_by(directory, &request, Some(dana_root), world.now, None);
    assert!(
        matches!(explained, Err(GrantError::NotHeld { .. })),
        "{explained:?}"
    );
    assert_eq!(world.events(), before, "a refusal records no use");
    Ok(())
}
