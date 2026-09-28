//! Observed grant use (`GRANT_LAST_USED`): a permitted exercise is recorded
//! in the grant log, a refused one is not, and a grant never seen exercised
//! reads as not seen.

mod support;

use std::error::Error;

use lys_identity::IdentityId;
use lys_identity::grants::{
    GrantChange, GrantError, GrantId, LastUse, MemoryRelationships, PassOn, RecipientKind, Route,
};
use support::{World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

fn last_use(world: &World, grant: GrantId) -> Result<LastUse, Box<dyn Error>> {
    Ok(world
        .grants
        .book()
        .record(grant)
        .ok_or("the grant is not held")?
        .last_use())
}

/// Dana's root, lent use-only to Tom.
fn lent(world: &mut World) -> Result<GrantId, Box<dyn Error>> {
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
    Ok(world.delegate(&request)?.event.grant())
}

#[test]
fn grant_last_used_an_exercise_is_recorded_with_its_holder_route_and_time() -> TestResult {
    let mut world = World::new()?;
    let tom = IdentityId::Person(world.tom);
    let grant = lent(&mut world)?;
    assert_eq!(
        last_use(&world, grant)?,
        LastUse::NotSeen,
        "never exercised reads as not seen"
    );
    let held = world
        .grants
        .book()
        .grant(grant)
        .ok_or("the grant is not held")?;
    assert_eq!(
        (held.resource(), held.actions()),
        (&alpha()?, &actions(&["read"])?)
    );
    let before = world.events();
    let permit = world.exercise(tom, "read", Route::Tool)?;
    let index = permit.use_event.ok_or("check records a use")??;
    assert_eq!(world.events(), before + 1, "one use event");
    assert_eq!(
        last_use(&world, grant)?,
        LastUse::Seen {
            at: world.now,
            route: Route::Tool,
            index
        }
    );
    let (signed, receipt) = &world.grants.events()[usize::try_from(index)?];
    assert_eq!(signed.event().caller(), tom, "the use names its holder");
    assert_eq!(
        signed.event().change(),
        &GrantChange::Use {
            grant,
            route: Route::Tool
        }
    );
    assert_eq!(receipt.change_kind, 3);
    world.now += 60;
    let later = world
        .exercise(tom, "read", Route::Browser)?
        .use_event
        .ok_or("check records a use")??;
    assert_eq!(
        last_use(&world, grant)?,
        LastUse::Seen {
            at: world.now,
            route: Route::Browser,
            index: later
        }
    );
    Ok(())
}

#[test]
fn grant_last_used_a_reopen_restores_it_from_the_log() -> TestResult {
    let mut world = World::new()?;
    let grant = lent(&mut world)?;
    let index = world
        .exercise(IdentityId::Person(world.tom), "read", Route::Api)?
        .use_event
        .ok_or("check records a use")??;
    let seen = last_use(&world, grant)?;
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(last_use(&world, grant)?, seen);
    assert_eq!(
        seen,
        LastUse::Seen {
            at: world.now,
            route: Route::Api,
            index
        }
    );
    let root = world
        .grants
        .book()
        .grant(grant)
        .and_then(|held| match held.source() {
            lys_identity::grants::Source::Grant(source) => Some(source),
            lys_identity::grants::Source::Root => None,
        });
    assert_eq!(
        last_use(&world, root.ok_or("no source")?)?,
        LastUse::NotSeen,
        "a source is not used by its descendant's use"
    );
    Ok(())
}

#[test]
fn grant_last_used_a_refused_check_writes_no_use() -> TestResult {
    let mut world = World::new()?;
    let grant = lent(&mut world)?;
    let before = world.events();
    assert!(matches!(
        world.exercise(IdentityId::Person(world.tom), "write", Route::Api),
        Err(GrantError::NotHeld { .. })
    ));
    assert!(matches!(
        world.exercise(IdentityId::Agent(world.tom_agent), "read", Route::Tool),
        Err(GrantError::NotHeld { .. })
    ));
    world.now = u64::MAX;
    assert!(
        world
            .exercise(IdentityId::Person(world.tom), "read", Route::Api)
            .is_ok(),
        "no end of its own, and none inherited"
    );
    world.now = support::T0 - 1;
    assert!(matches!(
        world.exercise(IdentityId::Person(world.tom), "read", Route::Api),
        Err(GrantError::NotStarted { .. })
    ));
    assert_eq!(
        world.events(),
        before + 1,
        "only the permitted check wrote a use"
    );
    assert!(matches!(
        last_use(&world, grant)?,
        LastUse::Seen { at: u64::MAX, .. }
    ));
    Ok(())
}
