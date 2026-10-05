#![cfg(test)]
//! A connector holds grants as any holder does and answers to the
//! administrator who approved its app (DIRECTORY-080 R1): a grant passed on
//! to it names that person as responsible, it is signed as a version 3 event,
//! and reopening the grant log keeps its kind.

use super::support;

use std::error::Error;
use std::sync::Arc;

use lys_identity::grants::{
    DelegateRequest, GrantError, MemoryRelationships, PassOn, RecipientKind, Relation, RootRequest,
    Route, Window,
};
use lys_identity::projection::Projection;
use lys_identity::projection::accounts::Accounts;
use lys_identity::{ConnectorId, IdentityId, LoginBinding, OperationId, Profile};
use support::{T0, World, alpha, pass};

type Outcome = Result<(), Box<dyn Error>>;

/// The directory with one connector, approved by Dana.
fn connector(world: &mut World) -> Result<(Projection, IdentityId), Box<dyn Error>> {
    let id = ConnectorId::from_bytes([81; 16]);
    let mut accounts = Accounts::default();
    accounts.put_connector(
        id,
        world.dana,
        &Profile::new("Cambium")?,
        &LoginBinding::new("https://issuer.test", "dana")?,
    );
    let projection = world
        .directory
        .projection()?
        .with_accounts(Arc::new(accounts));
    Ok((projection, IdentityId::Connector(id)))
}

#[test]
fn a_connector_answers_to_its_approver_holds_a_grant_signed_as_version_three_and_passes_it_on()
-> Outcome {
    let mut world = World::new()?;
    let root = world.root(
        world.dana,
        "kite",
        pass(
            &["read", "write"],
            &[RecipientKind::Connector, RecipientKind::Person],
        )?,
        None,
    )?;
    let (projection, app) = connector(&mut world)?;
    let record = projection
        .record(app)
        .ok_or("the connector has no record")?;
    assert_eq!(record.responsible(), Some(world.dana));
    let given = DelegateRequest {
        operation: OperationId::from_bytes([82; 16]),
        caller: IdentityId::Person(world.dana),
        route: Route::Api,
        source: root,
        recipient: app,
        responsible: world.dana,
        resource: alpha()?,
        relation: Relation::new("heron")?,
        pass_on: pass(&["read"], &[RecipientKind::Person])?,
        window: Window::new(T0, None)?,
    };
    let issued = world.grants.delegate(&projection, &given, T0)?;
    assert_eq!(issued.event.version(), 3);
    let held = issued.event.grant();
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        world
            .grants
            .book()
            .grant(held)
            .ok_or("grant missing after reopening")?
            .holder(),
        app
    );
    let onward = DelegateRequest {
        operation: OperationId::from_bytes([83; 16]),
        caller: app,
        route: Route::Api,
        source: held,
        recipient: IdentityId::Person(world.tom),
        responsible: world.tom,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    };
    let delivered = world.grants.delegate(&projection, &onward, T0 + 1)?;
    assert_eq!(delivered.event.caller(), app);
    assert_eq!(delivered.event.version(), 3);
    Ok(())
}

#[test]
fn a_grant_to_a_connector_naming_another_person_or_not_passable_to_it_is_refused() -> Outcome {
    let mut world = World::new()?;
    let (projection, app) = connector(&mut world)?;
    let to_connectors = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Connector])?,
        None,
    )?;
    let mut request = DelegateRequest {
        operation: OperationId::from_bytes([84; 16]),
        caller: IdentityId::Person(world.dana),
        route: Route::Api,
        source: to_connectors,
        recipient: app,
        responsible: world.tom,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    };
    let refused = world.grants.delegate(&projection, &request, T0);
    assert!(
        matches!(&refused, Err(GrantError::ResponsibleMismatch { .. })),
        "{refused:?}"
    );
    let to_people = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Person])?,
        None,
    )?;
    request.operation = OperationId::from_bytes([85; 16]);
    request.source = to_people;
    request.responsible = world.dana;
    let refused = world.grants.delegate(&projection, &request, T0);
    assert!(
        matches!(
            &refused,
            Err(GrantError::RecipientRefused {
                kind: RecipientKind::Connector,
                ..
            })
        ),
        "{refused:?}"
    );
    let root = RootRequest {
        operation: OperationId::from_bytes([86; 16]),
        caller: app,
        route: Route::Api,
        holder: world.tom,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    };
    assert!(matches!(
        world.grants.issue_root(&projection, &root, T0),
        Err(GrantError::RootAuthorityRefused { .. })
    ));
    Ok(())
}
