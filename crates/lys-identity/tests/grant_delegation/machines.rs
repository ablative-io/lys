#![cfg(test)]
//! A machine holds grants as any holder does and answers to the
//! administrator who issued its connection code (ACCESS-005 R1): a grant
//! passed on to it names that person as responsible, it is signed as a
//! version 6 event, and reopening the grant log keeps its kind. A machine
//! never issues or passes on a grant, and a machine a later join replaced is
//! retired.

use super::support;

use std::error::Error;
use std::sync::Arc;

use lys_identity::grants::{
    DelegateRequest, GrantError, MemoryRelationships, PassOn, RecipientKind, Relation, Route,
    Window, machine_may_hold,
};
use lys_identity::projection::Projection;
use lys_identity::projection::accounts::Accounts;
use lys_identity::{
    IdentityId, LifecycleState, LoginBinding, MachineId, OperationId, PersonId, Profile,
};
use support::{T0, World, alpha, pass};

type Outcome = Result<(), Box<dyn Error>>;

/// The directory with one machine whose code Dana gave, `retired` (replaced
/// by a later join) or not.
fn machine(world: &mut World, retired: bool) -> Result<(Projection, IdentityId), Box<dyn Error>> {
    let id = MachineId::from_bytes([91; 16]);
    let mut accounts = Accounts::default();
    accounts.put_machine(
        id,
        world.dana,
        &Profile::new("studio")?,
        retired,
        &LoginBinding::new("https://issuer.test", "dana")?,
    );
    let projection = world
        .directory
        .projection()?
        .with_accounts(Arc::new(accounts));
    Ok((projection, IdentityId::Machine(id)))
}

/// A request from `caller` passing `relation` of `source` on to `recipient`.
fn request(
    operation: u8,
    caller: IdentityId,
    source: lys_identity::grants::GrantId,
    recipient: IdentityId,
    responsible: PersonId,
    relation: &str,
) -> Result<DelegateRequest, Box<dyn Error>> {
    Ok(DelegateRequest {
        operation: OperationId::from_bytes([operation; 16]),
        caller,
        route: Route::Api,
        source,
        recipient,
        responsible,
        resource: alpha()?,
        relation: Relation::new(relation)?,
        pass_on: PassOn::UseOnly,
        window: Window::new(T0, None)?,
    })
}

#[test]
fn a_machine_answers_to_its_code_issuer_and_holds_a_grant_signed_as_version_six() -> Outcome {
    let mut world = World::new()?;
    let dana = IdentityId::Person(world.dana);
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read", "write"], &[RecipientKind::Machine])?,
        None,
    )?;
    let (projection, device) = machine(&mut world, false)?;
    let record = projection
        .record(device)
        .ok_or("the machine has no record")?;
    assert_eq!(record.responsible(), Some(world.dana));
    assert_eq!(record.state(), LifecycleState::Active);
    let given = request(92, dana, root, device, world.dana, "heron")?;
    let issued = world.grants.delegate(&projection, &given, T0)?;
    assert_eq!(issued.event.version(), 6);
    let held = issued.event.grant();
    world.reopen(MemoryRelationships::default())?;
    let kept = world
        .grants
        .book()
        .grant(held)
        .ok_or("grant missing after reopening")?;
    assert_eq!(kept.holder(), device);
    assert_eq!(kept.responsible(), world.dana);
    assert!(
        world
            .grants
            .book()
            .held_by(device)
            .any(|record| record.grant().id() == held)
    );
    Ok(())
}

#[test]
fn a_machine_never_passes_on_a_grant_even_one_it_holds() -> Outcome {
    let mut world = World::new()?;
    let dana = IdentityId::Person(world.dana);
    let root = world.root(
        world.dana,
        "kite",
        pass(
            &["read", "write"],
            &[RecipientKind::Machine, RecipientKind::Person],
        )?,
        None,
    )?;
    let (projection, device) = machine(&mut world, false)?;
    let mut given = request(93, dana, root, device, world.dana, "heron")?;
    given.pass_on = pass(&["read"], &[RecipientKind::Person])?;
    let held = world
        .grants
        .delegate(&projection, &given, T0)?
        .event
        .grant();
    let written = world.events();
    let onward = request(
        94,
        device,
        held,
        IdentityId::Person(world.tom),
        world.tom,
        "tern",
    )?;
    let refused = world.grants.delegate(&projection, &onward, T0 + 1);
    assert!(
        matches!(
            &refused,
            Err(GrantError::MachineRefused { machine, .. }) if *machine == device.to_string()
        ),
        "{refused:?}"
    );
    assert_eq!(world.events(), written, "a refusal writes nothing");
    Ok(())
}

#[test]
fn a_grant_to_a_machine_naming_another_person_or_to_a_replaced_machine_is_refused() -> Outcome {
    let mut world = World::new()?;
    let dana = IdentityId::Person(world.dana);
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Machine])?,
        None,
    )?;
    let (projection, device) = machine(&mut world, false)?;
    let wrong = request(95, dana, root, device, world.tom, "tern")?;
    let refused = world.grants.delegate(&projection, &wrong, T0);
    assert!(
        matches!(&refused, Err(GrantError::ResponsibleMismatch { .. })),
        "{refused:?}"
    );
    let (replaced, device) = machine(&mut world, true)?;
    let to_replaced = request(96, dana, root, device, world.dana, "tern")?;
    let refused = world.grants.delegate(&replaced, &to_replaced, T0);
    assert!(
        matches!(
            &refused,
            Err(GrantError::IdentityNotActive { identity, state: LifecycleState::Retired })
                if *identity == device.to_string()
        ),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn a_grant_not_passable_to_machines_is_refused_naming_the_machine_kind() -> Outcome {
    let mut world = World::new()?;
    let dana = IdentityId::Person(world.dana);
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Person, RecipientKind::Agent])?,
        None,
    )?;
    let (projection, device) = machine(&mut world, false)?;
    let refused = world.grants.delegate(
        &projection,
        &request(97, dana, root, device, world.dana, "tern")?,
        T0,
    );
    assert!(
        matches!(
            &refused,
            Err(GrantError::RecipientRefused {
                kind: RecipientKind::Machine,
                ..
            })
        ),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn a_machine_never_holds_a_responsibility_a_person_keeps_but_may_hold_an_apps_act() {
    for kept in [
        "grant.delegate",
        "request.approve",
        "secret.add",
        "role.create",
    ] {
        assert!(!machine_may_hold("directory", kept), "{kept}");
    }
    for ordinary in ["read", "view", "edit"] {
        assert!(machine_may_hold("directory", ordinary), "{ordinary}");
    }
    // A liminal link is an approved app's kind: its export and import are
    // the app's own acts, which a machine may hold (S305).
    assert!(machine_may_hold("liminal.link", "export"));
    assert!(machine_may_hold("liminal.link", "import"));
}
