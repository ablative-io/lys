#![cfg(test)]
//! R3: assigning a holding at the role's current version, its grants copied
//! from that version's templates (`ROLE_ASSIGN`, `ROLE_ASSIGN_OWNER`,
//! `ROLE_ASSIGN_OTHER_RESPONSIBLE`, `ROLE_ASSIGN_REFUSED`,
//! `ROLE_ASSIGN_POLICY` and `ROLE_ASSIGN_END`).

use std::error::Error;

use lys_identity::grants::{PassOn, Relation, RevokeRequest, Route};
use lys_identity::roles::test_support::{E, RoleWorld};
use lys_identity::roles::{
    Capacity, Check, Holding, MovePolicy, RoleChange, RoleError, RoleEvent, TemplateReason,
};
use lys_identity::{IdentityId, OperationId};

type TestResult = Result<(), Box<dyn Error>>;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

/// The action names each grant of `holding` carries, in the holding's order.
fn actions(world: &RoleWorld, holding: &Holding) -> Result<Vec<String>, Box<dyn Error>> {
    holding
        .grants
        .iter()
        .map(|id| {
            let grant = world.grant(*id)?;
            Ok(grant
                .actions()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","))
        })
        .collect()
}

fn granted(event: &RoleEvent) -> Result<&Holding, Box<dyn Error>> {
    match event.change() {
        RoleChange::HoldingGranted(holding) => Ok(holding),
        other => Err(format!("an assign commits a holding granted event, not {other:?}").into()),
    }
}

#[test]
fn role_assign_copies_the_current_versions_templates() -> TestResult {
    let (_dir, mut world) = world()?;
    let event = world.assign(world.h1, world.a1, None)?;
    let holding = world.holding(world.a1)?;
    assert_eq!(holding.version, 1);
    assert_eq!(actions(&world, &holding)?, ["read", "test"]);
    assert_eq!(event.actor(), world.h1);
    assert_eq!(event.capacity(), Capacity::ResponsiblePerson);
    assert_eq!(granted(&event)?.version, 1);
    assert_eq!(world.events_of_kind(4), 1);
    Ok(())
}

#[test]
fn role_assign_owner_is_admitted_as_project_owner() -> TestResult {
    let (_dir, mut world) = world()?;
    let event = world.assign(world.o1, world.a2, None)?;
    let holding = world.holding(world.a2)?;
    assert_eq!(holding.version, 1);
    assert_eq!(actions(&world, &holding)?, ["read", "test"]);
    assert_eq!(event.actor(), world.o1);
    assert_eq!(event.capacity(), Capacity::ProjectOwner);
    assert_eq!(world.events_of_kind(4), 1);
    Ok(())
}

#[test]
fn role_assign_other_responsible_copies_the_holders_own_responsible_person() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.o1, world.b1, None)?;
    let holding = world.holding(world.b1)?;
    assert_eq!(actions(&world, &holding)?, ["read", "test"]);
    for id in &holding.grants {
        let grant = world.grant(*id)?;
        assert_eq!(grant.responsible(), world.o2);
        assert_ne!(grant.responsible(), world.h1);
    }
    assert_eq!(
        world.capacity(world.o2, Some(world.b1), Check::Move)?,
        Some(Capacity::ResponsiblePerson)
    );
    assert_eq!(world.capacity(world.h1, Some(world.b1), Check::Move)?, None);
    world.make_version_2()?;
    let refused = world.move_to(world.h1, world.b1, 2, None).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::Move, .. })),
        "{refused:?}"
    );
    assert_eq!(world.events_of_kind(5), 0);
    Ok(())
}

#[test]
fn role_assign_refused_commits_no_holding_grant_or_event() -> TestResult {
    let (_dir, mut world) = world()?;
    let h1 = IdentityId::Person(world.h1);
    let relation = Relation::new("tester")?;
    let tester = world
        .grants
        .book()
        .held_by(h1)
        .find(|record| record.grant().parts().relation == relation)
        .map(|record| record.grant().id())
        .ok_or("H1 holds tester on P1")?;
    let revoke = RevokeRequest {
        operation: OperationId::generate()?,
        caller: IdentityId::Person(world.d1),
        route: Route::Api,
        grant: tester,
        reason: "H1's tester becomes use-only".to_owned(),
    };
    world.grants.revoke(&revoke, world.now)?;
    world.root(world.h1, "tester", PassOn::UseOnly)?;

    let holdings = world.roles.book().holdings().count();
    let grants = world.grants.revision();
    let events = world.roles.events().len();

    let refused = world.assign(world.x1, world.a1, None).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::Assign, .. })),
        "{refused:?}"
    );
    let refused = world.assign(world.d1, world.a1, None).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::Assign, .. })),
        "{refused:?}"
    );
    let refused = world.assign(world.h1, world.a1, None).err();
    let Some(RoleError::TemplatesRefused { refusals }) = refused else {
        return Err(format!("H1's assign is refused by template, not {refused:?}").into());
    };
    let named = refusals
        .iter()
        .map(|refusal| (refusal.relation.to_string(), refusal.reason))
        .collect::<Vec<_>>();
    assert_eq!(named, [("tester".to_owned(), TemplateReason::UseOnly)]);

    assert_eq!(world.roles.book().holdings().count() - holdings, 0);
    assert_eq!(world.grants.revision() - grants, 0);
    assert_eq!(world.roles.events().len() - events, 0);
    Ok(())
}

#[test]
fn role_assign_policy_follows_the_default_only_with_an_end_date() -> TestResult {
    let (_dir, mut world) = world()?;
    let role = world
        .roles
        .book()
        .role(world.builder)
        .ok_or("builder is made")?;
    assert_eq!(role.default_policy(), MovePolicy::MoveAtNextRenewal);
    world.assign(world.h1, world.a1, Some(E))?;
    world.assign(world.h1, world.a2, None)?;
    assert_eq!(world.holding(world.a1)?.policy, MovePolicy::MoveAtNextRenewal);
    assert_eq!(world.holding(world.a2)?.policy, MovePolicy::DeliberateOnly);
    Ok(())
}

#[test]
fn role_assign_end_is_written_on_every_grant() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    let holding = world.holding(world.a1)?;
    assert_eq!(holding.ends_at, Some(E));
    assert_eq!(holding.grants.len(), 2);
    for id in &holding.grants {
        assert_eq!(world.grant(*id)?.window().ends_at(), Some(E));
    }
    Ok(())
}
