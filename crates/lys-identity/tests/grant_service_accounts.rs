#![cfg(test)]
//! A service account has only delegated authority. Reopening the real grant
//! log must retain its kind, and narrowing, retirement and revocation must
//! apply through the same evaluator used for people and agents.

mod support;

use std::error::Error;

use lys_identity::grants::{
    DelegateRequest, GrantError, MemoryRelationships, PassOn, RecipientKind, Relation,
    RevokeRequest, RootRequest, Route, Window,
};
use lys_identity::projection::Projection;
use lys_identity::{IdentityId, LoginBinding, OperationId, Profile, ServiceAccountId};
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
