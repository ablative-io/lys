//! A responsible person passes only a boss agent's live authority to its child.

#[path = "support/world.rs"]
mod support;

use lys_identity::grants::admission::effective;
use lys_identity::grants::{
    DelegateRequest, GrantError, GrantId, MemoryRelationships, PassOn, RecipientKind, Relation,
    RevokeRequest, Route, Source,
};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use std::error::Error;
use support::{T0, World, pass};

type TestResult = Result<(), Box<dyn Error>>;

fn actor() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

fn family() -> Result<(World, AgentId, GrantId, DelegateRequest), Box<dyn Error>> {
    let mut world = World::new()?;
    let child = world
        .directory
        .register_reporting_agent(
            actor()?,
            OperationId::generate()?,
            IdentityId::Agent(world.tom_agent),
            Profile::new("Child")?,
            T0,
        )?
        .agent;
    world.directory.transition(
        actor()?,
        OperationId::generate()?,
        IdentityId::Agent(child),
        Transition::Activate,
        "",
        T0,
    )?;
    let root = world.root(
        world.tom,
        "kite",
        pass(&["read", "write"], &[RecipientKind::Agent])?,
        None,
    )?;
    let asked = world.request(
        IdentityId::Person(world.tom),
        root,
        IdentityId::Agent(world.tom_agent),
        "heron",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let boss = world.delegate(&asked)?.event.grant();
    let child_request = world.request(
        IdentityId::Person(world.tom),
        boss,
        IdentityId::Agent(child),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    Ok((world, child, boss, child_request))
}

#[test]
fn another_person_including_the_administrator_cannot_delegate_for_the_boss() -> TestResult {
    let (mut world, child, boss, request) = family()?;
    let before = world.events();
    for person in [world.admin, world.dana, world.lee] {
        let wrong = DelegateRequest {
            caller: IdentityId::Person(person),
            ..request.clone()
        };
        assert!(matches!(
            world.delegate(&wrong),
            Err(GrantError::NotHolder { .. })
        ));
        assert_eq!(world.events(), before);
    }
    let recorded = world.delegate(&request)?;
    assert_eq!(recorded.receipt.caller, IdentityId::Person(world.tom));
    assert_eq!(recorded.event.caller(), IdentityId::Person(world.tom));
    let grant = world
        .grants
        .book()
        .grant(recorded.event.grant())
        .ok_or("no child grant")?;
    assert_eq!(grant.source(), Source::Grant(boss));
    assert_eq!(grant.parts().issuer, IdentityId::Person(world.tom));
    world.reopen(MemoryRelationships::default())?;
    let permit = world.exercise(IdentityId::Agent(child), "read", Route::Tool)?;
    assert_eq!(permit.path[1], boss);
    let retry = world.delegate(&request)?;
    assert_eq!(retry, recorded);
    Ok(())
}

#[test]
fn a_child_leaving_the_boss_is_refused_by_the_effective_check() -> TestResult {
    let (mut world, child, _, request) = family()?;
    let grant = world.delegate(&request)?.event.grant();
    world.directory.change_reports_to(
        actor()?,
        OperationId::generate()?,
        child,
        IdentityId::Person(world.tom),
        T0 + 1,
    )?;
    assert!(matches!(
        effective(
            world.grants.book(),
            world.directory.projection()?,
            grant,
            world.now
        ),
        Err(GrantError::NotHolder { .. })
    ));
    assert!(
        world
            .exercise(IdentityId::Agent(child), "read", Route::Tool)
            .is_err()
    );
    Ok(())
}

#[test]
fn revoking_the_boss_grant_immediately_refuses_the_child() -> TestResult {
    let (mut world, child, boss, request) = family()?;
    let grant = world.delegate(&request)?.event.grant();
    world.grants.revoke(
        &RevokeRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(world.tom),
            route: Route::Api,
            grant: boss,
            reason: "withdraw authority".to_owned(),
        },
        world.now,
    )?;
    assert!(matches!(
        effective(
            world.grants.book(),
            world.directory.projection()?,
            grant,
            world.now
        ),
        Err(GrantError::Revoked { .. })
    ));
    assert!(
        world
            .exercise(IdentityId::Agent(child), "read", Route::Tool)
            .is_err()
    );
    Ok(())
}

#[test]
fn child_delegation_preserves_the_boss_action_and_pass_on_bounds() -> TestResult {
    let (mut world, child, _, request) = family()?;
    world.delegate(&request)?;
    let before = world.events();
    let wider = DelegateRequest {
        operation: OperationId::generate()?,
        relation: Relation::new("heron")?,
        ..request.clone()
    };
    assert!(matches!(
        world.delegate(&wider),
        Err(GrantError::ActionsOutside { .. })
    ));
    let onward = DelegateRequest {
        operation: OperationId::generate()?,
        pass_on: pass(&["read", "write"], &[RecipientKind::Agent])?,
        ..request.clone()
    };
    assert!(matches!(
        world.delegate(&onward),
        Err(GrantError::PassOnBeyondSource { .. })
    ));
    assert_eq!(world.events(), before);
    assert!(
        world
            .exercise(IdentityId::Agent(child), "write", Route::Tool)
            .is_err()
    );
    let root = world.root(
        world.tom,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let asked = world.request(
        IdentityId::Person(world.tom),
        root,
        IdentityId::Agent(world.tom_agent),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    let source = world.delegate(&asked)?.event.grant();
    let refused = DelegateRequest {
        operation: OperationId::generate()?,
        source,
        ..request
    };
    assert!(matches!(
        world.delegate(&refused),
        Err(GrantError::UseOnly { .. })
    ));
    Ok(())
}

#[test]
fn a_suspended_boss_or_responsible_person_cannot_delegate_or_keep_effective_authority() -> TestResult
{
    let (mut world, _, _, request) = family()?;
    let grant = world.delegate(&request)?.event.grant();
    for identity in [
        IdentityId::Agent(world.tom_agent),
        IdentityId::Person(world.tom),
    ] {
        world.directory.transition(
            actor()?,
            OperationId::generate()?,
            identity,
            Transition::Suspend,
            "held",
            T0 + 1,
        )?;
        let fresh = DelegateRequest {
            operation: OperationId::generate()?,
            ..request.clone()
        };
        assert!(matches!(
            world.delegate(&fresh),
            Err(GrantError::IdentityNotActive { .. })
        ));
        assert!(matches!(
            effective(
                world.grants.book(),
                world.directory.projection()?,
                grant,
                world.now
            ),
            Err(GrantError::IdentityNotActive { .. })
        ));
        world.directory.transition(
            actor()?,
            OperationId::generate()?,
            identity,
            Transition::Reinstate,
            "",
            T0 + 2,
        )?;
    }
    Ok(())
}
