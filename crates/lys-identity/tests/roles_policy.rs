#![cfg(test)]
//! R9: each holding's policy, move date and who can stop the move, and
//! changing a holding's policy or a role's default by a recorded act
//! (`ROLE_POLICY_SHOWN`, `ROLE_POLICY_CHANGE`, `ROLE_POLICY_CHANGE_OWNER`,
//! `ROLE_POLICY_NO_END`, `ROLE_DEFAULT` and `ROLE_DEFAULT_NO_END`).

use std::error::Error;

use lys_identity::grants::encode_grant;
use lys_identity::roles::policy::{DELIBERATE_WORDS, NEXT_RENEWAL_WORDS};
use lys_identity::roles::test_support::{E, RoleWorld};
use lys_identity::roles::{
    Capacity, ChangeDefault, ChangePolicy, Check, HoldingPolicy, MovePolicy, RoleChange,
    RoleError, RoleEvent,
};
use lys_identity::{AgentId, IdentityId, OperationId, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

/// A world with A1 ending E under move at the next renewal and A2 with no end date, both assigned by H1.
fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let mut world = RoleWorld::new(dir.path())?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.assign(world.h1, world.a2, None)?;
    Ok((dir, world))
}

fn change_policy(
    world: &mut RoleWorld,
    actor: PersonId,
    agent: AgentId,
    policy: MovePolicy,
) -> Result<RoleEvent, RoleError> {
    let request = ChangePolicy {
        operation: OperationId::generate()?,
        actor: IdentityId::Person(actor),
        holding: world.holding(agent)?.id,
        policy,
    };
    let (roles, mut acting) = world.acting()?;
    roles.change_policy(&mut acting, &request)
}

fn shown(world: &mut RoleWorld, agent: AgentId) -> Result<HoldingPolicy, RoleError> {
    let holding = world.holding(agent)?.id;
    let (roles, acting) = world.acting()?;
    roles.holding_policy(&acting, holding)
}

fn marked(world: &mut RoleWorld, agent: AgentId) -> Result<bool, Box<dyn Error>> {
    let role = world.builder;
    let (roles, acting) = world.acting()?;
    let answer = roles.role_policies(&acting, role)?;
    Ok(answer
        .holdings
        .iter()
        .find(|held| held.agent == agent)
        .ok_or("the holding is in the role's answer")?
        .differs_from_default)
}

#[test]
fn role_policy_shown_names_the_move_date_and_who_can_stop_it() -> TestResult {
    let (_dir, mut world) = world()?;
    let (a1, a2) = (world.a1, world.a2);
    let a1 = shown(&mut world, a1)?;
    assert_eq!(a1.policy, MovePolicy::MoveAtNextRenewal);
    assert_eq!(a1.moves_at, Some(E));
    assert_eq!(a1.words, NEXT_RENEWAL_WORDS);
    assert_eq!(a1.words, "Its next renewal moves it to the current version");
    assert_eq!(a1.who_can_stop, [world.h1, world.o1]);
    let a2 = shown(&mut world, a2)?;
    assert_eq!(a2.policy, MovePolicy::DeliberateOnly);
    assert_eq!(a2.words, DELIBERATE_WORDS);
    assert_eq!(a2.words, "Moves only by a deliberate act");
    assert_eq!(a2.moves_at, None);
    Ok(())
}

#[test]
fn role_policy_change_is_one_event_that_changes_nothing_else() -> TestResult {
    let (_dir, mut world) = world()?;
    let (h1, a1) = (world.h1, world.a1);
    let before = world.holding(world.a1)?;
    let bytes = |world: &RoleWorld, agent: AgentId| -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        world
            .holding(agent)?
            .grants
            .iter()
            .map(|id| Ok(encode_grant(world.grant(*id)?)))
            .collect()
    };
    let grants_before = bytes(&world, world.a1)?;
    let a2_before = world.holding(world.a2)?.policy;
    let event = change_policy(&mut world, h1, a1, MovePolicy::DeliberateOnly)?;
    let RoleChange::HoldingPolicyChanged {
        holding,
        before: from,
        after,
    } = event.change()
    else {
        return Err("a policy change commits a holding policy changed event".into());
    };
    assert_eq!(event.actor(), world.h1);
    assert_eq!(event.capacity(), Capacity::ResponsiblePerson);
    assert_eq!(*holding, before.id);
    assert_eq!(
        (*from, *after),
        (MovePolicy::MoveAtNextRenewal, MovePolicy::DeliberateOnly)
    );
    assert_eq!(world.events_of_kind(7), 1);
    let after_change = world.holding(world.a1)?;
    assert_eq!(world.holding(world.a2)?.policy, a2_before);
    assert_eq!(after_change.version, before.version);
    assert_eq!(after_change.ends_at, before.ends_at);
    assert_eq!(bytes(&world, world.a1)?, grants_before);
    Ok(())
}

#[test]
fn role_policy_change_owner_is_recorded_and_anyone_else_refused() -> TestResult {
    let (_dir, mut world) = world()?;
    let (h1, o1, x1, a1) = (world.h1, world.o1, world.x1, world.a1);
    change_policy(&mut world, h1, a1, MovePolicy::DeliberateOnly)?;
    let event = change_policy(&mut world, o1, a1, MovePolicy::MoveAtNextRenewal)?;
    assert_eq!(event.actor(), world.o1);
    assert_eq!(event.capacity(), Capacity::ProjectOwner);
    let before = world.roles.events().len();
    let refused = change_policy(&mut world, x1, a1, MovePolicy::DeliberateOnly).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::ChangePolicy, .. })),
        "{refused:?}"
    );
    assert_eq!(world.roles.events().len() - before, 0);
    Ok(())
}

#[test]
fn role_policy_no_end_refuses_move_at_next_renewal() -> TestResult {
    let (_dir, mut world) = world()?;
    let (h1, a2) = (world.h1, world.a2);
    let before = world.roles.events().len();
    let refused =
        change_policy(&mut world, h1, a2, MovePolicy::MoveAtNextRenewal).err();
    assert!(matches!(refused, Some(RoleError::NoEndDate { .. })), "{refused:?}");
    assert_eq!(world.roles.events().len() - before, 0);
    Ok(())
}

#[test]
fn role_default_applies_only_to_holdings_granted_after_it() -> TestResult {
    let (_dir, mut world) = world()?;
    let a1 = world.a1;
    let request = ChangeDefault {
        operation: OperationId::generate()?,
        actor: IdentityId::Person(world.o1),
        role: world.builder,
        policy: MovePolicy::DeliberateOnly,
    };
    let event = {
        let (roles, mut acting) = world.acting()?;
        roles.change_default(&mut acting, &request)?
    };
    assert!(matches!(
        event.change(),
        RoleChange::DefaultPolicyChanged {
            before: MovePolicy::MoveAtNextRenewal,
            after: MovePolicy::DeliberateOnly,
            ..
        }
    ));
    assert_eq!(event.actor(), world.o1);
    assert_eq!(world.holding(world.a1)?.policy, MovePolicy::MoveAtNextRenewal);
    assert!(marked(&mut world, a1)?);
    world.assign(world.h1, world.a4, Some(E))?;
    assert_eq!(world.holding(world.a4)?.policy, MovePolicy::DeliberateOnly);
    Ok(())
}

#[test]
fn role_default_no_end_is_never_marked_as_differing() -> TestResult {
    let (_dir, mut world) = world()?;
    let (h1, a1, a2) = (world.h1, world.a1, world.a2);
    change_policy(&mut world, h1, a1, MovePolicy::DeliberateOnly)?;
    assert!(marked(&mut world, a1)?);
    assert!(!marked(&mut world, a2)?);
    let a2 = shown(&mut world, a2)?;
    assert_eq!(a2.words, "Moves only by a deliberate act");
    assert_eq!(a2.moves_at, None);
    assert!(a2.who_can_stop.is_empty());
    Ok(())
}
