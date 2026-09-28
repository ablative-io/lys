//! R4: inherited expiry, role versions and reinstatement (`GRANT_EXPIRY` and
//! `GRANT_REINSTATE`).

mod support;

use std::error::Error;

use lys_identity::grants::admission::effective;
use lys_identity::grants::{
    GrantError, GrantId, MemoryRelationships, Model, PassOn, RecipientKind, Relation,
    RevokeRequest, Route, Window,
};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, Provenance,
    Transition,
};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

fn expired(grant: GrantId, ended_at: u64) -> GrantError {
    GrantError::Expired {
        grant: grant.to_string(),
        ended_at,
    }
}

#[test]
fn grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom, lee, agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Person(world.lee),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &BOTH)?,
        Some(T0 + 1_000),
    )?;
    let to_tom = world.request(
        dana,
        root,
        tom,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        Some(T0 + 500),
    )?;
    let to_tom = world.delegate(&to_tom)?.event.grant();
    let to_agent = world.request(tom, to_tom, agent, "tern", PassOn::UseOnly, Some(T0 + 500))?;
    let to_agent = world.delegate(&to_agent)?.event.grant();
    let mut later = world.request(dana, root, lee, "tern", PassOn::UseOnly, None)?;
    later.window = Window::new(T0 + 200, Some(T0 + 300))?;
    let later = world.delegate(&later)?.event.grant();

    let newer = Model::new(
        2,
        [
            (Relation::new("tern")?, actions(&["read", "write"])?),
            (Relation::new("wren")?, actions(&["read"])?),
        ],
    )?;
    assert!(matches!(
        world.grants.set_model(support_model_again()?),
        Err(GrantError::ModelInvalid { .. })
    ));
    world.grants.set_model(newer)?;
    world.now = T0 + 499;
    assert!(
        matches!(
            world.exercise(agent, "write", Route::Tool),
            Err(GrantError::NotHeld { .. })
        ),
        "a newer version of the relation does not widen a grant held"
    );
    let widened = world.request(tom, to_tom, agent, "tern", PassOn::UseOnly, Some(T0 + 500))?;
    assert!(matches!(
        world.delegate(&widened),
        Err(GrantError::ActionsOutside {
            model_version: 2,
            ..
        })
    ));
    let lengthened = world.request(tom, to_tom, agent, "wren", PassOn::UseOnly, Some(T0 + 600))?;
    assert!(
        matches!(world.delegate(&lengthened), Err(GrantError::DelegationOutlivesSource { source_ends, .. }) if source_ends == T0 + 500)
    );

    let boundaries = [
        (agent, T0 + 499, None),
        (agent, T0 + 500, Some(expired(to_tom, T0 + 500))),
        (tom, T0 + 499, None),
        (tom, T0 + 500, Some(expired(to_tom, T0 + 500))),
        (dana, T0 + 999, None),
        (dana, T0 + 1_000, Some(expired(root, T0 + 1_000))),
        (
            lee,
            T0 + 199,
            Some(GrantError::NotStarted {
                grant: later.to_string(),
                starts_at: T0 + 200,
            }),
        ),
        (lee, T0 + 200, None),
        (lee, T0 + 299, None),
        (lee, T0 + 300, Some(expired(later, T0 + 300))),
    ];
    for (who, at, refusal) in &boundaries {
        world.now = *at;
        let decision = world.exercise(*who, "read", Route::Api);
        match refusal {
            None => assert!(decision.is_ok(), "{who} at {at}: {decision:?}"),
            Some(refusal) => assert_eq!(decision.as_ref(), Err(refusal), "{who} at {at}"),
        }
    }
    world.now = T0 + 500;
    let after_end = world.request(tom, to_tom, agent, "wren", PassOn::UseOnly, Some(T0 + 500))?;
    assert_eq!(
        world.delegate(&after_end),
        Err(expired(to_tom, T0 + 500)),
        "nothing is passed on after its end"
    );
    assert_eq!(
        world
            .grants
            .book()
            .grant(to_agent)
            .map(|held| held.window().ends_at()),
        Some(Some(T0 + 500))
    );
    assert_eq!(boundaries.len(), 10);
    Ok(())
}

/// The first model again, which is not newer than the model it would replace.
fn support_model_again() -> Result<Model, GrantError> {
    Model::new(
        1,
        [(
            Relation::new("tern")?,
            actions(&["read", "write", "delete"])?,
        )],
    )
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

#[test]
fn grant_reinstate_rechecks_every_grant_and_resurrects_none() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let revoked = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    let revoked = world.delegate(&revoked)?.event.grant();
    let ending = world.request(dana, root, tom, "tern", PassOn::UseOnly, Some(T0 + 100))?;
    let ending = world.delegate(&ending)?.event.grant();
    let control_root = world.root(world.dana, "heron", pass(&["read", "write"], &BOTH)?, None)?;
    let control = world.request(dana, control_root, tom, "heron", PassOn::UseOnly, None)?;
    let control = world.delegate(&control)?.event.grant();
    let now = world.now;
    world.directory.transition(
        administrator()?,
        OperationId::generate()?,
        tom,
        Transition::Suspend,
        "on leave",
        now,
    )?;
    assert!(matches!(
        world.exercise(tom, "write", Route::Api),
        Err(GrantError::IdentityNotActive {
            state: LifecycleState::Suspended,
            ..
        })
    ));
    let request = RevokeRequest {
        operation: OperationId::generate()?,
        caller: dana,
        route: Route::Browser,
        grant: revoked,
        reason: "no longer needed".to_owned(),
    };
    world.grants.revoke(&request, now)?;
    world.now = T0 + 150;
    let before = world.events();
    let now = world.now;
    world.directory.transition(
        administrator()?,
        OperationId::generate()?,
        tom,
        Transition::Reinstate,
        "",
        now,
    )?;
    assert_eq!(
        world.events(),
        before,
        "reinstatement records no grant event"
    );
    let expected = [
        (
            revoked,
            Err(GrantError::Revoked {
                grant: revoked.to_string(),
            }),
        ),
        (ending, Err(expired(ending, T0 + 100))),
        (control, Ok(vec![control, control_root])),
    ];
    for (grant, outcome) in expected {
        let directory = world.directory.projection()?;
        let decided =
            effective(world.grants.book(), directory, grant, world.now).map(|lineage| lineage.path);
        assert_eq!(decided, outcome, "{grant}");
    }
    let permit = world.exercise(tom, "read", Route::Api)?;
    assert_eq!(
        (permit.grant, permit.path),
        (control, vec![control, control_root]),
        "only the control is usable"
    );
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(world.exercise(tom, "write", Route::Api)?.grant, control);
    assert_eq!(
        world
            .grants
            .book()
            .record(ending)
            .and_then(|record| record.revoked()),
        None,
        "expiry is not a revocation"
    );
    assert_eq!(
        world
            .grants
            .book()
            .grant(control)
            .map(|held| held.resource().clone()),
        Some(alpha()?)
    );
    Ok(())
}

#[test]
fn grant_to_an_identity_that_is_not_active_is_refused_when_given() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let now = world.now;
    world.directory.transition(
        administrator()?,
        OperationId::generate()?,
        tom,
        Transition::Suspend,
        "on leave",
        now,
    )?;
    let before = world.events();
    let request = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    assert!(matches!(
        world.delegate(&request),
        Err(GrantError::IdentityNotActive {
            state: LifecycleState::Suspended,
            ..
        })
    ));
    assert_eq!(world.events(), before);
    Ok(())
}
