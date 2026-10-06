#![cfg(test)]
//! A service account has only delegated authority. Reopening the real grant
//! log must retain its kind, and narrowing, retirement and revocation must
//! apply through the same evaluator used for people and agents.

use super::support;

use std::error::Error;

use lys_identity::grants::{
    Action, DelegateRequest, ExerciseRequest, GrantError, MemoryRelationships, PassOn,
    RecipientKind, Relation, RevokeRequest, RootRequest, Route, Window,
};
use lys_identity::projection::Projection;
use lys_identity::{
    IdentityId, LifecycleState, LoginBinding, OperationId, Profile, ServiceAccountId,
};
use support::{T0, World, alpha, pass};

type Outcome = Result<(), Box<dyn Error>>;

fn account(world: &mut World, retired: bool) -> Result<(Projection, IdentityId), Box<dyn Error>> {
    let id = ServiceAccountId::from_bytes([91; 16]);
    let mut projection = world.directory.projection()?.clone();
    projection.service_account(
        id,
        world.dana,
        Profile::new("Lys loader")?,
        retired,
        LoginBinding::new("https://issuer.test", "administrator")?,
    )?;
    Ok((projection, IdentityId::ServiceAccount(id)))
}

#[test]
fn delegated_service_account_survives_restart_and_reimport_then_revocation_stops_it() -> Outcome {
    let mut world = World::new()?;
    let root = world.root(
        world.dana,
        "kite",
        pass(
            &["read", "write"],
            &[RecipientKind::ServiceAccount, RecipientKind::Person],
        )?,
        None,
    )?;
    let (projection, service) = account(&mut world, false)?;
    let given = DelegateRequest {
        operation: OperationId::from_bytes([92; 16]),
        caller: IdentityId::Person(world.dana),
        route: Route::Api,
        source: root,
        recipient: service,
        responsible: world.dana,
        resource: alpha()?,
        relation: Relation::new("heron")?,
        pass_on: pass(&["read"], &[RecipientKind::Person])?,
        window: Window::new(T0, None)?,
    };
    let issued = world.grants.delegate(&projection, &given, T0)?;
    assert_eq!(issued.event.version(), 2);
    let held = issued.event.grant();
    let count = world.events();
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        world
            .grants
            .book()
            .grant(held)
            .ok_or("grant missing")?
            .holder(),
        service
    );
    assert_eq!(
        world
            .grants
            .delegate(&projection, &given, T0 + 1)?
            .event
            .grant(),
        held
    );
    assert_eq!(world.events(), count);

    let onward = DelegateRequest {
        operation: OperationId::from_bytes([93; 16]),
        caller: service,
        route: Route::Api,
        source: held,
        recipient: IdentityId::Person(world.tom),
        responsible: world.tom,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    };
    let delivered = world.grants.delegate(&projection, &onward, T0 + 2)?;
    assert_eq!(delivered.event.caller(), service);
    assert_eq!(delivered.event.version(), 2);
    assert_eq!(
        world
            .grants
            .book()
            .grant(delivered.event.grant())
            .ok_or("child missing")?
            .parts()
            .issuer,
        service
    );

    let mut too_wide = onward.clone();
    too_wide.operation = OperationId::from_bytes([94; 16]);
    too_wide.relation = Relation::new("heron")?;
    assert!(
        world
            .grants
            .delegate(&projection, &too_wide, T0 + 3)
            .is_err()
    );
    world.grants.revoke(
        &RevokeRequest {
            operation: OperationId::from_bytes([95; 16]),
            caller: IdentityId::Person(world.dana),
            route: Route::Api,
            grant: held,
            reason: "remove importer authority".into(),
        },
        T0 + 4,
    )?;
    let mut after = onward;
    after.operation = OperationId::from_bytes([96; 16]);
    assert!(world.grants.delegate(&projection, &after, T0 + 5).is_err());
    Ok(())
}

/// A check whose only path runs through a grant a retired service account
/// holds is refused naming Retired, with nothing recorded; with a second
/// standing grant beside it, the check is allowed by that one.
#[test]
fn a_check_through_a_retired_service_accounts_grant_is_refused_and_another_grant_allows_it()
-> Outcome {
    let mut world = World::new()?;
    let root = world.root(
        world.dana,
        "kite",
        pass(
            &["read", "write"],
            &[RecipientKind::ServiceAccount, RecipientKind::Person],
        )?,
        None,
    )?;
    let (active, service) = account(&mut world, false)?;
    let held = world
        .grants
        .delegate(
            &active,
            &DelegateRequest {
                operation: OperationId::from_bytes([81; 16]),
                caller: IdentityId::Person(world.dana),
                route: Route::Api,
                source: root,
                recipient: service,
                responsible: world.dana,
                resource: alpha()?,
                relation: Relation::new("heron")?,
                pass_on: pass(&["read"], &[RecipientKind::Person])?,
                window: Window::new(T0, None)?,
            },
            T0,
        )?
        .event
        .grant();
    let tom = world.tom;
    let onward = |operation: u8, caller: IdentityId, source| -> Result<_, GrantError> {
        Ok(DelegateRequest {
            operation: OperationId::from_bytes([operation; 16]),
            caller,
            route: Route::Api,
            source,
            recipient: IdentityId::Person(tom),
            responsible: tom,
            resource: alpha()?,
            relation: Relation::new("tern")?,
            pass_on: PassOn::UseOnly,
            window: Window::new(T0, None)?,
        })
    };
    world
        .grants
        .delegate(&active, &onward(82, service, held)?, T0 + 1)?;
    let read = ExerciseRequest {
        caller: IdentityId::Person(tom),
        route: Route::Api,
        resource: alpha()?,
        action: Action::new("read")?,
    };
    let (retired, _) = account(&mut world, true)?;
    let count = world.events();
    let refused = world.grants.check(&retired, &read, T0 + 2, None);
    assert!(
        matches!(
            &refused,
            Err(GrantError::IdentityNotActive { identity, state: LifecycleState::Retired })
                if *identity == service.to_string()
        ),
        "{refused:?}"
    );
    assert_eq!(world.events(), count, "a refused check records nothing");

    let beside = onward(83, IdentityId::Person(world.dana), root)?;
    let standing = world
        .grants
        .delegate(&retired, &beside, T0 + 3)?
        .event
        .grant();
    let permit = world.grants.check(&retired, &read, T0 + 4, None)?;
    assert_eq!(permit.grant, standing, "allowed by the standing grant");
    Ok(())
}

/// An agent below a service account reads through the account's grant while
/// the account stands; once the account is retired the agent's check is
/// refused naming Retired, with nothing recorded.
#[test]
fn an_agent_below_a_service_account_is_refused_once_the_account_is_retired() -> Outcome {
    let mut world = World::new()?;
    let root = world.root(
        world.dana,
        "kite",
        pass(
            &["read", "write"],
            &[RecipientKind::ServiceAccount, RecipientKind::Agent],
        )?,
        None,
    )?;
    let (active, service) = account(&mut world, false)?;
    let held = world
        .grants
        .delegate(
            &active,
            &DelegateRequest {
                operation: OperationId::from_bytes([71; 16]),
                caller: IdentityId::Person(world.dana),
                route: Route::Api,
                source: root,
                recipient: service,
                responsible: world.dana,
                resource: alpha()?,
                relation: Relation::new("heron")?,
                pass_on: pass(&["read"], &[RecipientKind::Agent])?,
                window: Window::new(T0, None)?,
            },
            T0,
        )?
        .event
        .grant();
    let agent = IdentityId::Agent(world.tom_agent);
    let mut below = world.request(service, held, agent, "tern", PassOn::UseOnly, None)?;
    below.operation = OperationId::from_bytes([72; 16]);
    let lent = world
        .grants
        .delegate(&active, &below, T0 + 1)?
        .event
        .grant();
    let read = ExerciseRequest {
        caller: agent,
        route: Route::Tool,
        resource: alpha()?,
        action: Action::new("read")?,
    };
    let permit = world.grants.check(&active, &read, T0 + 2, None)?;
    assert_eq!(permit.grant, lent, "the agent reads through the account");

    let (retired, _) = account(&mut world, true)?;
    let count = world.events();
    let refused = world.grants.check(&retired, &read, T0 + 3, None);
    assert!(
        matches!(
            &refused,
            Err(GrantError::IdentityNotActive { identity, state: LifecycleState::Retired })
                if *identity == service.to_string()
        ),
        "{refused:?}"
    );
    assert_eq!(world.events(), count, "a refused check records nothing");
    Ok(())
}

#[test]
fn a_service_account_cannot_mint_a_root_or_receive_a_person_only_grant() -> Outcome {
    let mut world = World::new()?;
    let (projection, service) = account(&mut world, false)?;
    let request = RootRequest {
        operation: OperationId::from_bytes([97; 16]),
        caller: service,
        route: Route::Api,
        holder: world.tom,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    };
    assert!(matches!(
        world.grants.issue_root(&projection, &request, T0),
        Err(GrantError::RootAuthorityRefused { .. })
    ));
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Person])?,
        None,
    )?;
    let request = DelegateRequest {
        operation: OperationId::from_bytes([98; 16]),
        caller: IdentityId::Person(world.dana),
        route: Route::Api,
        source: root,
        recipient: service,
        responsible: world.dana,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    };
    assert!(matches!(
        world.grants.delegate(&projection, &request, T0),
        Err(GrantError::RecipientRefused { .. })
    ));
    Ok(())
}
