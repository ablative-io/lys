#![cfg(test)]
//! R8: a renewal is a new recorded grant at the version the holding's
//! policy names (`ROLE_RENEW`, `ROLE_RENEW_NONE`, `ROLE_RENEW_AT_END`,
//! `ROLE_RENEW_OWNER`, `ROLE_RENEW_DELIBERATE` and `ROLE_RENEW_REFUSED`).

use std::error::Error;

use lys_identity::grants::GrantError;
use lys_identity::grants::admission::effective;
use lys_identity::roles::test_support::{DAY, E, RoleWorld, T0};
use lys_identity::roles::{
    Capacity, ChangePolicy, MovePolicy, Renew, RoleChange, RoleError, RoleEvent,
};
use lys_identity::{AgentId, IdentityId, OperationId, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

const HOUR: u64 = 3_600;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

fn renew(
    world: &mut RoleWorld,
    actor: PersonId,
    agent: AgentId,
    ends_at: u64,
) -> Result<RoleEvent, RoleError> {
    let request = Renew {
        operation: OperationId::generate()?,
        actor: IdentityId::Person(actor),
        holding: world.holding(agent)?.id,
        ends_at,
    };
    let (roles, mut acting) = world.acting()?;
    roles.renew(&mut acting, &request)
}

/// Refuse unless `agent` holds version `version` with the grants of
/// `actions`, each ending at `ends_at`.
fn holds(
    world: &RoleWorld,
    agent: AgentId,
    version: u64,
    actions: &[&str],
    ends_at: u64,
) -> TestResult {
    let holding = world.holding(agent)?;
    assert_eq!(holding.version, version);
    assert_eq!(holding.ends_at, Some(ends_at));
    let mut carried = Vec::new();
    for id in &holding.grants {
        let grant = world.grant(*id)?;
        assert_eq!(grant.window().ends_at(), Some(ends_at));
        carried.extend(grant.actions().iter().map(ToString::to_string));
    }
    assert_eq!(carried, actions);
    Ok(())
}

fn renewed(event: &RoleEvent) -> Result<(u64, u64), Box<dyn Error>> {
    match event.change() {
        RoleChange::HoldingRenewed {
            version, ends_at, ..
        } => Ok((*version, *ends_at)),
        other => Err(format!("a renewal commits a holding renewed event, not {other:?}").into()),
    }
}

/// A1 on version 1 under move at the next renewal, ending E, assigned by
/// `assigner`, with version 2 made at T0 plus one day.
fn a1_with_version_2(
    assigner: fn(&RoleWorld) -> PersonId,
) -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let (dir, mut world) = world()?;
    world.assign(assigner(&world), world.a1, Some(E))?;
    world.now = T0 + DAY;
    world.make_version_2()?;
    Ok((dir, world))
}

#[test]
fn role_renew_under_move_at_next_renewal_lands_on_the_current_version() -> TestResult {
    let (_dir, mut world) = a1_with_version_2(|world| world.h1)?;
    let (h1, a1) = (world.h1, world.a1);
    assert_eq!(world.holding(world.a1)?.policy, MovePolicy::MoveAtNextRenewal);
    world.now = E - 2 * HOUR;
    assert_eq!(world.holding(world.a1)?.version, 1);
    world.now = E - HOUR;
    let event = renew(&mut world, h1, a1, E + 30 * DAY)?;
    assert_eq!(event.actor(), world.h1);
    assert_eq!(event.capacity(), Capacity::ResponsiblePerson);
    let RoleChange::HoldingRenewed { holding, .. } = event.change() else {
        return Err("a renewal commits a holding renewed event".into());
    };
    assert_eq!(*holding, world.holding(world.a1)?.id);
    assert_eq!(renewed(&event)?, (2, E + 30 * DAY));
    holds(&world, world.a1, 2, &["read", "push"], E + 30 * DAY)
}

#[test]
fn role_renew_none_lapses_on_the_version_it_held() -> TestResult {
    let (_dir, mut world) = a1_with_version_2(|world| world.h1)?;
    world.now = E - 1;
    let holding = world.holding(world.a1)?;
    assert_eq!(holding.version, 1);
    assert!(!holding.lapsed(world.now));
    world.now = E + 1;
    assert!(holding.lapsed(world.now));
    let directory = world.directory.projection()?;
    for id in &holding.grants {
        let answered = effective(world.grants.book(), directory, *id, E + 1).err();
        assert!(
            matches!(answered, Some(GrantError::Expired { ended_at, .. }) if ended_at == E),
            "{answered:?}"
        );
    }
    assert_eq!(world.holding(world.a1)?.version, 1);
    assert_eq!(world.events_of_kind(5), 0);
    assert_eq!(world.events_of_kind(6), 0);
    Ok(())
}

#[test]
fn role_renew_at_end_is_admitted_at_exactly_the_end_date() -> TestResult {
    let (_dir, mut world) = a1_with_version_2(|world| world.h1)?;
    let (h1, a1) = (world.h1, world.a1);
    world.now = E;
    let event = renew(&mut world, h1, a1, E + 30 * DAY)?;
    assert_eq!(event.actor(), world.h1);
    assert_eq!(renewed(&event)?, (2, E + 30 * DAY));
    holds(&world, world.a1, 2, &["read", "push"], E + 30 * DAY)
}

#[test]
fn role_renew_owner_is_recorded_as_project_owner() -> TestResult {
    let (_dir, mut world) = a1_with_version_2(|world| world.o1)?;
    let (o1, a1) = (world.o1, world.a1);
    world.now = E - HOUR;
    let event = renew(&mut world, o1, a1, E + 30 * DAY)?;
    assert_eq!(event.actor(), world.o1);
    assert_eq!(event.capacity(), Capacity::ProjectOwner);
    assert_eq!(renewed(&event)?, (2, E + 30 * DAY));
    Ok(())
}

#[test]
fn role_renew_deliberate_stays_on_the_held_version() -> TestResult {
    let (_dir, mut world) = a1_with_version_2(|world| world.h1)?;
    let (h1, a1) = (world.h1, world.a1);
    let request = ChangePolicy {
        operation: OperationId::generate()?,
        actor: IdentityId::Person(world.h1),
        holding: world.holding(world.a1)?.id,
        policy: MovePolicy::DeliberateOnly,
    };
    {
        let (roles, mut acting) = world.acting()?;
        roles.change_policy(&mut acting, &request)?;
    }
    world.now = E - HOUR;
    let event = renew(&mut world, h1, a1, E + 30 * DAY)?;
    assert_eq!(renewed(&event)?, (1, E + 30 * DAY));
    holds(&world, world.a1, 1, &["read", "test"], E + 30 * DAY)
}

#[test]
fn role_renew_refused_commits_nothing() -> TestResult {
    let (_dir, mut world) = a1_with_version_2(|world| world.h1)?;
    let (h1, x1, a1, a2) = (world.h1, world.x1, world.a1, world.a2);
    world.assign(world.h1, world.a2, None)?;
    let before = world.roles.events().len();
    let refused = renew(&mut world, x1, a1, E + 30 * DAY).err();
    assert!(matches!(refused, Some(RoleError::RenewRefused { .. })), "{refused:?}");
    world.now = E + 1;
    let refused = renew(&mut world, h1, a1, E + 30 * DAY).err();
    assert!(
        matches!(refused, Some(RoleError::HoldingLapsed { ended_at: E, .. })),
        "{refused:?}"
    );
    let refused = renew(&mut world, h1, a2, E + 30 * DAY).err();
    assert!(matches!(refused, Some(RoleError::NoEndDate { .. })), "{refused:?}");
    assert_eq!(world.roles.events().len() - before, 0);
    Ok(())
}
