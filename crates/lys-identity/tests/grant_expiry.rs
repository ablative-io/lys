//! R4: inherited expiry, role versions and reinstatement (`GRANT_EXPIRY` and
//! `GRANT_REINSTATE`).

mod support;

use std::collections::HashMap;
use std::error::Error;

use lys_identity::grants::admission::{effective, judge_delegation};
use lys_identity::grants::lineage::resolve;
use lys_identity::grants::{
    Grant, GrantError, GrantId, GrantParts, MemoryRelationships, Model, PassOn, RecipientKind,
    Relation, RevokeRequest, Route, Source, Window,
};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, PersonId, Provenance,
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
fn row_2_6_grant_expiry_every_end_binds_at_its_boundary_and_no_role_change_extends_it() -> TestResult
{
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
        matches!(world.delegate(&lengthened), Err(GrantError::ExpiryBeyondSource { source_ends, .. }) if source_ends == T0 + 500)
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

/// Admission's own check: a delegation ending past its source is refused
/// before anything reaches the book, so the check at commit never sees it.
#[test]
fn row_2_6_admission_refuses_a_delegation_ending_past_its_source_before_any_commit() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &BOTH)?,
        Some(T0 + 1_000),
    )?;
    let request = world.request(dana, root, tom, "tern", PassOn::UseOnly, Some(T0 + 1_001))?;
    let before = world.grants.book().clone();
    let judged = judge_delegation(
        world.grants.book(),
        world.directory.projection()?,
        world.grants.model(),
        &request,
        world.now,
        GrantId::from_bytes([4; 16]),
    );
    assert_eq!(
        judged,
        Err(GrantError::ExpiryBeyondSource {
            requested: (T0 + 1_001).to_string(),
            source_grant: root.to_string(),
            source_ends: T0 + 1_000,
        })
    );
    assert_eq!(
        world.grants.book(),
        &before,
        "a refused delegation changed the book"
    );
    Ok(())
}

/// The hop check at exercise: a chain written by hand, as neither admission
/// nor the check at commit would let it be, is refused by the walk every
/// exercise runs, at the hop whose end runs past its source's.
#[test]
fn row_2_6_resolve_refuses_a_derived_grant_ending_past_its_source_at_exercise() -> TestResult {
    let (root, derived) = (GrantId::from_bytes([1; 16]), GrantId::from_bytes([2; 16]));
    let person = PersonId::from_bytes([9; 16]);
    let grant = |id, source, ends| -> Result<Grant, GrantError> {
        Grant::new(GrantParts {
            id,
            issuer: IdentityId::Person(person),
            holder: IdentityId::Person(person),
            responsible: person,
            resource: alpha()?,
            relation: Relation::new("tern")?,
            actions: actions(&["read"])?,
            pass_on: pass(&["read"], &BOTH)?,
            source,
            window: Window::new(T0, Some(ends))?,
            model_version: 1,
            operation: OperationId::from_bytes([3; 16]),
        })
    };
    let held = HashMap::from([
        (root, grant(root, Source::Root, T0 + 1_000)?),
        (derived, grant(derived, Source::Grant(root), T0 + 2_000)?),
    ]);
    assert_eq!(
        resolve(derived, |id| held.get(&id)),
        Err(GrantError::ExpiryBeyondSource {
            requested: (T0 + 2_000).to_string(),
            source_grant: root.to_string(),
            source_ends: T0 + 1_000,
        })
    );
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
