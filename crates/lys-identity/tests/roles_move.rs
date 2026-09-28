#![cfg(test)]
//! R6: a move by a deliberate act that shows what changes first
//! (`ROLE_MOVE_PREVIEW`, `ROLE_MOVE_RECORD`, `ROLE_MOVE_CAPACITY` and
//! `ROLE_MOVE_END`).

use std::error::Error;

use lys_identity::IdentityId;
use lys_identity::grants::{GrantId, PassOn, RecipientKind, Relation};
use lys_identity::roles::test_support::{E, RoleWorld, T0};
use lys_identity::roles::{Capacity, Check, GrantView, RoleChange, RoleError, Timing};

type TestResult = Result<(), Box<dyn Error>>;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

fn assert_view(world: &RoleWorld, view: &GrantView, relation: &str, action: &str) -> TestResult {
    assert_eq!(view.relation, Relation::new(relation)?);
    assert_eq!(view.resource, world.p1);
    assert_eq!(
        view.actions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [action]
    );
    assert_eq!(view.pass_on, PassOn::UseOnly);
    assert_eq!(view.window.ends_at(), Some(E));
    assert_eq!(view.responsible, world.h1);
    Ok(())
}

fn action_of(world: &RoleWorld, ids: &[GrantId]) -> Result<Vec<String>, Box<dyn Error>> {
    ids.iter()
        .map(|id| {
            Ok(world
                .grant(*id)?
                .actions()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","))
        })
        .collect()
}

#[test]
fn role_move_preview_names_what_it_adds_and_removes_and_commits_nothing() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.make_version_2()?;
    let (roles, grants) = (world.roles.events().len(), world.grants.revision());
    let holding = world.holding(world.a1)?.id;
    let directory = world.directory.projection()?;
    let preview = world.roles.preview(directory, holding, 2)?;
    assert_eq!((preview.from, preview.to), (1, 2));
    assert_eq!(preview.added.len(), 1);
    assert_eq!(preview.removed.len(), 1);
    assert_view(&world, &preview.added[0], "pusher", "push")?;
    assert_view(&world, &preview.removed[0], "tester", "test")?;
    assert_eq!(world.roles.events().len(), roles);
    assert_eq!(world.grants.revision(), grants);
    Ok(())
}

#[test]
fn role_move_record_is_one_event_naming_actor_capacity_versions_timing_and_grants()
-> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.make_version_2()?;
    let event = world.move_to(world.h1, world.a1, 2, None)?;
    let RoleChange::HoldingMoved {
        from,
        to,
        timing,
        added,
        removed,
        ..
    } = event.change()
    else {
        return Err("a move commits a holding moved event".into());
    };
    assert_eq!(event.actor(), world.h1);
    assert_eq!(event.capacity(), Capacity::ResponsiblePerson);
    assert_eq!((*from, *to, *timing), (1, 2, Timing::NextStart));
    assert_eq!(action_of(&world, added)?, ["push"]);
    assert_eq!(action_of(&world, removed)?, ["test"]);
    assert_eq!(world.events_of_kind(5), 1);
    assert_eq!(world.holding(world.a1)?.version, 2);
    Ok(())
}

#[test]
fn role_move_capacity_admits_the_responsible_person_or_an_owner_only() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.assign(world.o1, world.a2, Some(E))?;
    world.make_version_2()?;
    let before = world.events_of_kind(5);

    let event = world.move_to(world.o1, world.a2, 2, None)?;
    assert_eq!(event.capacity(), Capacity::ProjectOwner);

    let refused = world.move_to(world.x1, world.a1, 2, None).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::Move, .. })),
        "{refused:?}"
    );
    let refused = world.move_to(world.d1, world.a1, 2, None).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::Move, .. })),
        "{refused:?}"
    );

    world.give_owner(IdentityId::Person(world.d1), IdentityId::Person(world.d1))?;
    let pass = PassOn::to(
        [lys_identity::grants::Action::new("push")?].into(),
        [RecipientKind::Agent].into(),
    )?;
    world.root(world.d1, "pusher", pass)?;
    let event = world.move_to(world.d1, world.a1, 2, None)?;
    assert_eq!(event.actor(), world.d1);
    assert_eq!(event.capacity(), Capacity::ProjectOwner);
    assert_eq!(world.events_of_kind(5) - before, 2);
    assert!(world.now > T0);
    Ok(())
}

#[test]
fn role_move_end_date_is_the_same_after_the_move() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.make_version_2()?;
    world.move_to(world.h1, world.a1, 2, None)?;
    let holding = world.holding(world.a1)?;
    assert_eq!(holding.version, 2);
    assert_eq!(holding.ends_at, Some(E));
    assert_eq!(holding.grants.len(), 2);
    for id in &holding.grants {
        assert_eq!(world.grant(*id)?.window().ends_at(), Some(E));
    }
    Ok(())
}
