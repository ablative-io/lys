//! A held grant is never exercised at once (ACCESS-001 R1): its act is taken
//! through an approved draft, so an exercise of a by_draft or by_two grant is
//! refused `ModeHeld` by name, before any use is recorded; asking why is still
//! answered, naming the held grant. A held root is issued under its mode,
//! answers a retry of its operation, and refuses the same operation asked
//! again in another mode.

mod support;

use std::error::Error;

use lys_identity::grants::{
    Action, ExerciseRequest, GrantError, Mode, PassOn, Relation, RootRequest, Route, Window,
};
use lys_identity::{IdentityId, OperationId};
use support::{T0, World, alpha};

type TestResult = Result<(), Box<dyn Error>>;

fn request(world: &World, operation: OperationId) -> Result<RootRequest, Box<dyn Error>> {
    Ok(RootRequest {
        operation,
        caller: IdentityId::Person(world.admin),
        route: Route::Api,
        holder: world.dana,
        resource: alpha()?,
        relation: Relation::new("heron")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    })
}

fn issue(world: &mut World, mode: Mode, operation: OperationId) -> Result<String, Box<dyn Error>> {
    let asked = request(world, operation)?;
    let directory = world.directory.projection()?;
    let recorded = world.grants.issue_root_in(directory, &asked, mode, world.now)?;
    Ok(recorded.event.grant().to_string())
}

#[test]
fn exercising_a_held_grant_is_refused_by_name_and_records_no_use() -> TestResult {
    for mode in [Mode::ByDraft, Mode::ByTwo] {
        let mut world = World::new()?;
        let grant = issue(&mut world, mode, OperationId::generate()?)?;
        let before = world.events();
        let refused = world
            .exercise(IdentityId::Person(world.dana), "write", Route::Api)
            .err()
            .ok_or("a held grant was exercised at once")?;
        assert_eq!(
            refused,
            GrantError::ModeHeld {
                grant: grant.clone(),
                mode: mode.as_str(),
            },
            "{mode:?}"
        );
        assert_eq!(world.events(), before, "{mode:?}: no use is recorded");
        let asked = ExerciseRequest {
            caller: IdentityId::Person(world.dana),
            route: Route::Api,
            resource: alpha()?,
            action: Action::new("write")?,
        };
        let directory = world.directory.projection()?;
        let why = world.grants.explain(directory, &asked, world.now, None)?;
        assert_eq!(why.grant.to_string(), grant, "{mode:?}: why names it");
    }
    Ok(())
}

#[test]
fn an_outright_grant_is_still_exercised() -> TestResult {
    let mut world = World::new()?;
    issue(&mut world, Mode::Outright, OperationId::generate()?)?;
    let before = world.events();
    world.exercise(IdentityId::Person(world.dana), "write", Route::Api)?;
    assert_eq!(world.events(), before + 1, "the use is recorded");
    Ok(())
}

#[test]
fn a_held_root_answers_its_retry_and_refuses_another_mode() -> TestResult {
    let mut world = World::new()?;
    let operation = OperationId::generate()?;
    let first = issue(&mut world, Mode::ByTwo, operation)?;
    let size = world.events();
    assert_eq!(issue(&mut world, Mode::ByTwo, operation)?, first);
    assert_eq!(world.events(), size, "a retry writes nothing");
    for other in [Mode::ByDraft, Mode::Outright] {
        let refused = issue(&mut world, other, operation)
            .err()
            .ok_or("the operation was reused in another mode")?;
        assert!(
            refused.to_string().starts_with("OperationReused"),
            "{other:?}: {refused}"
        );
    }
    let held = world
        .grants
        .book()
        .records()
        .find(|record| record.grant().id().to_string() == first)
        .ok_or("the book holds it")?;
    assert_eq!(held.grant().mode(), Mode::ByTwo);
    Ok(())
}
