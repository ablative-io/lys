//! R2: affirmative delegation and bounded ancestry (`GRANT_USE_VS_LEND`,
//! `GRANT_RECIPIENT`, `GRANT_ANCESTRY` and `GRANT_AGENT_PARITY`).

#[path = "grant_delegation/service_accounts.rs"]
mod service_accounts;
mod support;

use std::collections::HashMap;
use std::error::Error;

use lys_identity::grants::lineage::resolve;
use lys_identity::grants::{
    Grant, GrantError, GrantId, GrantParts, MemoryRelationships, PassOn, RecipientKind, Relation,
    Resource, Route, Source, Window,
};
use lys_identity::{IdentityId, OperationId};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const ROUTES: [Route; 3] = [Route::Browser, Route::Api, Route::Tool];
const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

#[test]
fn row_2_2_grant_use_vs_lend_use_only_is_exercised_and_never_passed_on() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom, tom_agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read", "write"], &BOTH)?, None)?;
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    let lent = world.delegate(&lent)?.event.grant();
    let permit = world.exercise(tom, "read", Route::Browser)?;
    assert_eq!(
        (permit.grant, permit.path, permit.root_person),
        (lent, vec![lent, root], world.dana)
    );
    let before = world.events();
    for route in ROUTES {
        let mut request = world.request(tom, lent, tom_agent, "tern", PassOn::UseOnly, None)?;
        request.route = route;
        assert_eq!(
            world.delegate(&request),
            Err(GrantError::UseOnly {
                grant: lent.to_string()
            }),
            "{route:?}"
        );
        assert_eq!(world.events(), before, "{route:?} created no grant event");
    }
    let lending = world.request(
        dana,
        root,
        tom,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let lending = world.delegate(&lending)?.event.grant();
    let agent_request = world.request(tom, lending, tom_agent, "tern", PassOn::UseOnly, None)?;
    let bounded = world.delegate(&agent_request)?.event.grant();
    assert_eq!(world.events(), before + 2);
    let permit = world.exercise(tom_agent, "read", Route::Tool)?;
    assert_eq!(permit.path, vec![bounded, lending, root]);
    assert_eq!(
        world.exercise(tom_agent, "write", Route::Tool),
        Err(GrantError::NotHeld {
            identity: tom_agent.to_string(),
            resource: alpha()?.to_string(),
            action: "write".to_owned(),
        }),
        "the bounded control carries read alone"
    );
    Ok(())
}

#[test]
fn row_2_2_grant_recipient_each_kind_is_permitted_or_refused_by_name() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom, lee, tom_agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Person(world.lee),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let lend = |world: &mut World, pass_on: PassOn| -> Result<GrantId, Box<dyn Error>> {
        let request = world.request(dana, root, tom, "tern", pass_on, None)?;
        Ok(world.delegate(&request)?.event.grant())
    };
    let people_only = lend(&mut world, pass(&["read"], &[RecipientKind::Person])?)?;
    let agents_only = lend(&mut world, pass(&["read"], &[RecipientKind::Agent])?)?;
    let use_only = lend(&mut world, PassOn::UseOnly)?;
    let refused_kind = |kind, grant: GrantId| GrantError::RecipientRefused {
        kind,
        grant: grant.to_string(),
    };
    let cases = [
        (people_only, lee, None),
        (
            people_only,
            tom_agent,
            Some(refused_kind(RecipientKind::Agent, people_only)),
        ),
        (
            agents_only,
            lee,
            Some(refused_kind(RecipientKind::Person, agents_only)),
        ),
        (agents_only, tom_agent, None),
        (
            use_only,
            lee,
            Some(GrantError::UseOnly {
                grant: use_only.to_string(),
            }),
        ),
        (
            use_only,
            tom_agent,
            Some(GrantError::UseOnly {
                grant: use_only.to_string(),
            }),
        ),
    ];
    let mut counted = 0;
    for (source, recipient, refusal) in cases {
        let before = world.events();
        let request = world.request(tom, source, recipient, "tern", PassOn::UseOnly, None)?;
        match refusal {
            None => {
                world.delegate(&request)?;
                assert_eq!(
                    world.events(),
                    before + 1,
                    "one event for a permitted pass-on"
                );
            }
            Some(refusal) => {
                assert_eq!(world.delegate(&request), Err(refusal));
                assert_eq!(world.events(), before, "zero events for a refusal");
            }
        }
        counted += 1;
    }
    assert_eq!(counted, 6, "three grants, each with both recipient kinds");
    Ok(())
}

/// Two grants deriving from each other, written by hand as no admission would.
fn cycle() -> Result<HashMap<GrantId, Grant>, Box<dyn Error>> {
    let (first, second) = (GrantId::from_bytes([1; 16]), GrantId::from_bytes([2; 16]));
    let person = lys_identity::PersonId::from_bytes([9; 16]);
    let grant = |id, source| -> Result<Grant, GrantError> {
        Grant::new(GrantParts {
            id,
            issuer: IdentityId::Person(person),
            holder: IdentityId::Person(person),
            responsible: person,
            resource: alpha()?,
            relation: Relation::new("tern")?,
            actions: actions(&["read"])?,
            pass_on: pass(&["read"], &BOTH)?,
            source: Source::Grant(source),
            window: Window::new(T0, None)?,
            model_version: 1,
            operation: OperationId::from_bytes([3; 16]),
        })
    };
    Ok(HashMap::from([
        (first, grant(first, second)?),
        (second, grant(second, first)?),
    ]))
}

/// A chain of `length` grants, each passed on from the one before it to the
/// same person, the first a root: an ancestry far longer than any bound.
fn chain(length: u128) -> Result<HashMap<GrantId, Grant>, Box<dyn Error>> {
    let person = lys_identity::PersonId::from_bytes([9; 16]);
    let id = |n: u128| GrantId::from_bytes((n + 1).to_be_bytes());
    let mut grants = HashMap::new();
    for n in 0..length {
        let source = if n == 0 {
            Source::Root
        } else {
            Source::Grant(id(n - 1))
        };
        let grant = Grant::new(GrantParts {
            id: id(n),
            issuer: IdentityId::Person(person),
            holder: IdentityId::Person(person),
            responsible: person,
            resource: alpha()?,
            relation: Relation::new("tern")?,
            actions: actions(&["read"])?,
            pass_on: pass(&["read"], &BOTH)?,
            source,
            window: Window::new(T0, None)?,
            model_version: 1,
            operation: OperationId::from_bytes([3; 16]),
        })?;
        grants.insert(id(n), grant);
    }
    Ok(grants)
}

#[test]
fn an_ancestry_of_any_length_resolves_to_its_root() -> TestResult {
    let chain = chain(1000)?;
    let last = GrantId::from_bytes(1000_u128.to_be_bytes());
    let lineage = resolve(last, |id| chain.get(&id))?;
    assert_eq!(lineage.path.len(), 1000);
    assert_eq!(lineage.path.first(), Some(&last));
    assert_eq!(
        lineage.path.last(),
        Some(&GrantId::from_bytes(1_u128.to_be_bytes()))
    );
    assert_eq!(
        lineage.root_person,
        lys_identity::PersonId::from_bytes([9; 16])
    );
    Ok(())
}

#[test]
fn row_2_1_row_2_3_grant_ancestry_refuses_at_the_blocking_boundary_and_explains_a_permitted_chain()
-> TestResult {
    let mut world = World::new()?;
    let (dana, tom, tom_agent, dana_agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
        IdentityId::Agent(world.dana_agent),
    );
    let ends = T0 + 1_000;
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read", "write"], &BOTH)?,
        Some(ends),
    )?;
    let request = world.request(
        dana,
        root,
        tom,
        "heron",
        pass(&["read"], &[RecipientKind::Agent])?,
        Some(ends),
    )?;
    let middle = world.delegate(&request)?.event.grant();
    let before = world.events();
    let mut refusals = Vec::new();
    let mut broader_resource =
        world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, Some(ends))?;
    broader_resource.resource = Resource::new("project", "beta")?;
    refusals.push((
        broader_resource,
        GrantError::ResourceOutside {
            requested: "project:beta".to_owned(),
            source_grant: middle.to_string(),
        },
    ));
    refusals.push((
        world.request(tom, middle, tom_agent, "heron", PassOn::UseOnly, Some(ends))?,
        GrantError::ActionsOutside {
            relation: "heron".to_owned(),
            outside: "write".to_owned(),
            model_version: 1,
        },
    ));
    let forged = GrantId::from_bytes([0xf0; 16]);
    refusals.push((
        world.request(tom, forged, tom_agent, "tern", PassOn::UseOnly, Some(ends))?,
        GrantError::SourceUnknown {
            grant: forged.to_string(),
        },
    ));
    refusals.push((
        world.request(tom, root, tom_agent, "tern", PassOn::UseOnly, Some(ends))?,
        GrantError::NotHolder {
            caller: tom.to_string(),
            grant: root.to_string(),
        },
    ));
    let mut changed_responsible =
        world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, Some(ends))?;
    changed_responsible.responsible = world.dana;
    refusals.push((
        changed_responsible,
        GrantError::ResponsibleMismatch {
            identity: tom_agent.to_string(),
            named: dana.to_string(),
            recorded: tom.to_string(),
        },
    ));
    refusals.push((
        world.request(
            tom,
            middle,
            tom_agent,
            "tern",
            PassOn::UseOnly,
            Some(ends + 1),
        )?,
        GrantError::ExpiryBeyondSource {
            requested: (ends + 1).to_string(),
            source_grant: middle.to_string(),
            source_ends: ends,
        },
    ));
    refusals.push((
        world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, None)?,
        GrantError::ExpiryBeyondSource {
            requested: "no end".to_owned(),
            source_grant: middle.to_string(),
            source_ends: ends,
        },
    ));
    refusals.push((
        world.request(
            tom,
            middle,
            tom_agent,
            "tern",
            pass(&["read"], &BOTH)?,
            Some(ends),
        )?,
        GrantError::PassOnBeyondSource {
            source_grant: middle.to_string(),
        },
    ));
    for (request, refusal) in &refusals {
        assert_eq!(world.delegate(request).as_ref(), Err(refusal));
    }
    let cycle = cycle()?;
    let start = GrantId::from_bytes([1; 16]);
    assert_eq!(
        resolve(start, |id| cycle.get(&id)),
        Err(GrantError::LineageCycle {
            grant: start.to_string()
        })
    );
    let other = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let lent = world.request(dana, other, tom, "tern", pass(&["read"], &BOTH)?, None)?;
    let lent = world.delegate(&lent)?.event.grant();
    let revoke = lys_identity::grants::RevokeRequest {
        operation: OperationId::generate()?,
        caller: IdentityId::Person(world.admin),
        route: Route::Api,
        grant: other,
        reason: "the project closed".to_owned(),
    };
    let now = world.now;
    world.grants.revoke(&revoke, now)?;
    let after_revoke = world.events();
    let under_revoked = world.request(tom, lent, dana_agent, "tern", PassOn::UseOnly, None)?;
    assert_eq!(
        world.delegate(&under_revoked),
        Err(GrantError::Revoked {
            grant: other.to_string()
        })
    );
    assert_eq!(
        world.events(),
        after_revoke,
        "a refused request records nothing"
    );
    assert_eq!(
        after_revoke,
        before + 3,
        "only the second root, its loan and the revocation were recorded"
    );
    let narrower = world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, Some(ends))?;
    let last = world.delegate(&narrower)?.event.grant();
    let permit = world.exercise(tom_agent, "read", Route::Api)?;
    assert_eq!(
        (permit.path, permit.root_person),
        (vec![last, middle, root], world.dana)
    );
    world.reopen(MemoryRelationships::default())?;
    let replayed = world.exercise(tom_agent, "read", Route::Api)?;
    assert_eq!(
        replayed.path,
        vec![last, middle, root],
        "a replay answers the same authority"
    );
    assert_eq!(refusals.len(), 8);
    Ok(())
}

#[test]
fn row_2_2_grant_agent_parity_routes_decide_alike_and_pass_on_decides_who_delegates() -> TestResult
{
    let mut world = World::new()?;
    let (dana, tom, tom_agent, dana_agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
        IdentityId::Agent(world.dana_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let to_tom = world.request(dana, root, tom, "tern", pass(&["read"], &BOTH)?, None)?;
    let to_tom = world.delegate(&to_tom)?.event.grant();
    let passing = world.request(
        tom,
        to_tom,
        tom_agent,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let passing = world.delegate(&passing)?.event.grant();
    let decisions = ROUTES.map(|route| {
        world.exercise(tom_agent, "read", route).map(|permit| {
            assert!(
                permit.use_event.as_ref().is_some_and(Result::is_ok),
                "{route:?}: the use is recorded"
            );
            (
                permit.grant,
                permit.path,
                permit.root_person,
                permit.actions,
                permit.model_version,
            )
        })
    });
    assert!(
        decisions.iter().all(|decision| decision == &decisions[0]),
        "{decisions:?}"
    );
    assert!(decisions[0].is_ok());
    let by_agent = world.request(
        tom_agent,
        passing,
        dana_agent,
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    let lent_on = world.delegate(&by_agent)?.event.grant();
    assert_eq!(
        world.exercise(dana_agent, "read", Route::Tool)?.path,
        vec![lent_on, passing, to_tom, root]
    );
    let refused: Vec<_> = ROUTES
        .iter()
        .map(|route| -> Result<_, Box<dyn Error>> {
            let mut request = world.request(
                dana_agent,
                lent_on,
                tom_agent,
                "tern",
                PassOn::UseOnly,
                None,
            )?;
            request.route = *route;
            Ok(world.delegate(&request))
        })
        .collect::<Result<_, _>>()?;
    let use_only = Err(GrantError::UseOnly {
        grant: lent_on.to_string(),
    });
    assert!(
        refused.iter().all(|decision| decision == &use_only),
        "{refused:?}"
    );
    let by_person = world.request(tom, to_tom, dana_agent, "tern", PassOn::UseOnly, None)?;
    assert!(
        world.delegate(&by_person).is_ok(),
        "a person with the same pass-on is judged alike"
    );
    Ok(())
}
