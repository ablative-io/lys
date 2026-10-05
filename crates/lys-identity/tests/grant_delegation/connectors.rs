#![cfg(test)]
//! A connector holds grants as any holder does and answers to the
//! administrator who approved its app (DIRECTORY-080 R1): a grant passed on
//! to it names that person as responsible, it is signed as a version 3 event,
//! and reopening the grant log keeps its kind. It is active only while its
//! approver is and its app is not retired.

use super::support;

use std::error::Error;
use std::sync::Arc;

use lys_identity::grants::{
    DelegateRequest, GrantError, GrantId, MemoryRelationships, PassOn, RecipientKind, Relation,
    RootRequest, Route, Window,
};
use lys_identity::projection::Projection;
use lys_identity::projection::accounts::Accounts;
use lys_identity::{
    Actor, AuthMethod, ConnectorId, IdentityId, LifecycleState, LoginBinding, OperationId,
    PersonId, Profile, Provenance, Transition,
};
use support::{T0, World, alpha, pass};

type Outcome = Result<(), Box<dyn Error>>;

/// The directory with one connector, approved by Dana, its app `retired` or not.
fn connector(world: &mut World, retired: bool) -> Result<(Projection, IdentityId), Box<dyn Error>> {
    let id = ConnectorId::from_bytes([81; 16]);
    let mut accounts = Accounts::default();
    accounts.put_connector(
        id,
        world.dana,
        &Profile::new("Cambium")?,
        retired,
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
    let (projection, app) = connector(&mut world, false)?;
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
    let (projection, app) = connector(&mut world, false)?;
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

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

/// Whether `result` is the refusal naming `app` as not active, in `state`.
fn not_active<T>(result: &Result<T, GrantError>, app: IdentityId, state: LifecycleState) -> bool {
    matches!(
        result,
        Err(GrantError::IdentityNotActive { identity, state: held })
            if *identity == app.to_string() && *held == state
    )
}

/// A request from `caller` passing `tern` of `source` on to `recipient`.
fn tern(
    operation: u8,
    caller: IdentityId,
    source: GrantId,
    recipient: IdentityId,
    responsible: PersonId,
) -> Result<DelegateRequest, Box<dyn Error>> {
    Ok(DelegateRequest {
        operation: OperationId::from_bytes([operation; 16]),
        caller,
        route: Route::Api,
        source,
        recipient,
        responsible,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        pass_on: pass(&["read"], &[RecipientKind::Person])?,
        window: Window::new(T0, None)?,
    })
}

#[test]
fn a_retired_connector_is_refused_as_recipient_and_as_caller_naming_retired() -> Outcome {
    let mut world = World::new()?;
    let dana = IdentityId::Person(world.dana);
    let root = world.root(
        world.dana,
        "kite",
        pass(
            &["read"],
            &[RecipientKind::Connector, RecipientKind::Person],
        )?,
        None,
    )?;
    let (active, app) = connector(&mut world, false)?;
    let held = world
        .grants
        .delegate(&active, &tern(87, dana, root, app, world.dana)?, T0)?
        .event
        .grant();
    let (retired, _) = connector(&mut world, true)?;
    assert_eq!(
        retired.record(app).ok_or("no record")?.state(),
        LifecycleState::Retired
    );
    let written = world.events();
    let to_retired = world
        .grants
        .delegate(&retired, &tern(88, dana, root, app, world.dana)?, T0);
    assert!(
        not_active(&to_retired, app, LifecycleState::Retired),
        "{to_retired:?}"
    );
    let tom = IdentityId::Person(world.tom);
    let from_retired = world
        .grants
        .delegate(&retired, &tern(89, app, held, tom, world.tom)?, T0);
    assert!(
        not_active(&from_retired, app, LifecycleState::Retired),
        "{from_retired:?}"
    );
    assert_eq!(world.events(), written, "a refusal writes nothing");
    Ok(())
}

#[test]
fn a_connector_follows_its_approver_through_suspension_and_reinstatement() -> Outcome {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let kinds = [RecipientKind::Connector, RecipientKind::Person];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    let toms = world.root(world.tom, "kite", pass(&["read"], &kinds)?, None)?;
    let (active, app) = connector(&mut world, false)?;
    let held = world
        .grants
        .delegate(&active, &tern(90, dana, root, app, world.dana)?, T0)?
        .event
        .grant();
    let now = world.now;
    let move_dana = |transition: Transition,
                     world: &mut World|
     -> Result<(Projection, IdentityId), Box<dyn Error>> {
        world.directory.transition(
            administrator()?,
            OperationId::generate()?,
            dana,
            transition,
            "on leave",
            now,
        )?;
        connector(world, false)
    };
    let (suspended, _) = move_dana(Transition::Suspend, &mut world)?;
    assert_eq!(
        suspended.record(app).ok_or("no record")?.state(),
        LifecycleState::Suspended
    );
    let (from_app, to_app) = (
        tern(91, app, held, tom, world.tom)?,
        tern(92, tom, toms, app, world.dana)?,
    );
    let written = world.events();
    for request in [&from_app, &to_app] {
        let refused = world.grants.delegate(&suspended, request, T0);
        assert!(
            not_active(&refused, app, LifecycleState::Suspended),
            "{refused:?}"
        );
    }
    assert_eq!(world.events(), written, "a refusal writes nothing");
    let (reinstated, _) = move_dana(Transition::Reinstate, &mut world)?;
    assert_eq!(
        reinstated.record(app).ok_or("no record")?.state(),
        LifecycleState::Active
    );
    for request in [&from_app, &to_app] {
        world.grants.delegate(&reinstated, request, T0)?;
    }
    assert_eq!(world.events(), written + 2);
    Ok(())
}
