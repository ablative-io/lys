#![cfg(test)]
//! R4: a template edit makes a version and moves no holder; a title edit is
//! recorded and makes none (`ROLE_EDIT`, `ROLE_EDIT_TITLE`,
//! `ROLE_EDIT_REFUSED`, `ROLE_EDIT_NEW_DEFAULT` and `ROLE_EDIT_END`).

use std::error::Error;

use lys_identity::grants::encode_grant;
use lys_identity::roles::test_support::{E, RoleWorld};
use lys_identity::roles::{Capacity, ChangeTitle, Check, MovePolicy, RoleChange, RoleError};
use lys_identity::{AgentId, IdentityId, OperationId};

type TestResult = Result<(), Box<dyn Error>>;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

/// The encoded bytes of each grant `agent`'s holding carries.
fn grant_bytes(world: &RoleWorld, agent: AgentId) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    world
        .holding(agent)?
        .grants
        .iter()
        .map(|id| Ok(encode_grant(world.grant(*id)?)))
        .collect()
}

fn last_version(world: &RoleWorld) -> Result<u64, Box<dyn Error>> {
    Ok(world
        .roles
        .book()
        .role(world.builder)
        .ok_or("builder is made")?
        .current()
        .number())
}

#[test]
fn role_edit_makes_a_version_and_leaves_every_holder_and_grant() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, None)?;
    world.assign(world.h1, world.a2, None)?;
    let (a1_before, a2_before) = (grant_bytes(&world, world.a1)?, grant_bytes(&world, world.a2)?);
    assert_eq!(a1_before.len(), 2);
    let event = world.make_version_2()?;
    assert!(matches!(event.change(), RoleChange::VersionMade { version: 2, .. }));
    assert_eq!(last_version(&world)?, 2);
    assert_eq!(world.holding(world.a1)?.version, 1);
    assert_eq!(world.holding(world.a2)?.version, 1);
    assert_eq!(grant_bytes(&world, world.a1)?, a1_before);
    assert_eq!(grant_bytes(&world, world.a2)?, a2_before);
    Ok(())
}

#[test]
fn role_edit_title_is_recorded_and_makes_no_version() -> TestResult {
    let (_dir, mut world) = world()?;
    let request = ChangeTitle {
        operation: OperationId::generate()?,
        actor: IdentityId::Person(world.o1),
        role: world.builder,
        title: "Code builder".to_owned(),
    };
    let event = {
        let (roles, mut acting) = world.acting()?;
        roles.change_title(&mut acting, &request)?
    };
    let RoleChange::TitleChanged { before, after, .. } = event.change() else {
        return Err("a title change commits a title changed event".into());
    };
    assert_eq!((before.as_str(), after.as_str()), ("Builder", "Code builder"));
    assert_eq!(event.actor(), world.o1);
    assert_eq!(event.capacity(), Capacity::ProjectOwner);
    assert_eq!(world.events_of_kind(2), 1);
    assert_eq!(last_version(&world)?, 1);
    Ok(())
}

#[test]
fn role_edit_refused_for_anyone_but_an_owner() -> TestResult {
    let (_dir, mut world) = world()?;
    let before = world.roles.events().len();
    let pairs = [("reader", "read"), ("pusher", "push")];
    let refused = world.edit(world.h1, &pairs).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::EditTemplates, .. })),
        "{refused:?}"
    );
    let refused = world.edit(world.d1, &pairs).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::EditTemplates, .. })),
        "{refused:?}"
    );
    assert_eq!(last_version(&world)?, 1);
    assert_eq!(world.roles.events().len() - before, 0);
    Ok(())
}

#[test]
fn role_edit_new_default_is_move_at_next_renewal() -> TestResult {
    let (_dir, mut world) = world()?;
    let event = world.make_role("Reviewer", &[("reader", "read")])?;
    let RoleChange::VersionMade { role, .. } = event.change() else {
        return Err("making a role commits a version made event".into());
    };
    let reviewer = world.roles.book().role(*role).ok_or("reviewer is made")?;
    assert_eq!(reviewer.current().number(), 1);
    assert_eq!(reviewer.project(), &world.p1);
    assert_eq!(reviewer.default_policy(), MovePolicy::MoveAtNextRenewal);
    Ok(())
}

#[test]
fn role_edit_end_date_is_the_same_after_the_edit() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.make_version_2()?;
    let holding = world.holding(world.a1)?;
    assert_eq!(holding.ends_at, Some(E));
    assert_eq!(holding.grants.len(), 2);
    for id in &holding.grants {
        assert_eq!(world.grant(*id)?.window().ends_at(), Some(E));
    }
    Ok(())
}
