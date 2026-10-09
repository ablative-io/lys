#![cfg(test)]

//! R4: revocation and current permission decisions (`GRANT_REVOKE` and
//! `GRANT_FRESHNESS`).

mod support;

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_identity::grants::admission::effective;
use lys_identity::grants::test_support::FailingRelationships;
use lys_identity::grants::{
    ExerciseRequest, GrantError, GrantId, MemoryRelationships, PassOn, RecipientKind, Relationship,
    RelationshipStore, Resource, RevokeRequest, Route,
};
use lys_identity::log::Reopen;
use lys_identity::{IdentityId, OperationId};
use lys_log_store::FileLeafStore;
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

fn held<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match slot.lock() {
        Ok(held) => held,
        Err(error) => panic!("fixture_lock_poisoned: {error}"),
    }
}

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

fn revoke(
    world: &World<FileLeafStore, impl RelationshipStore>,
    grant: GrantId,
) -> Result<RevokeRequest, Box<dyn Error>> {
    Ok(RevokeRequest {
        operation: OperationId::generate()?,
        caller: IdentityId::Person(world.admin),
        route: Route::Api,
        grant,
        reason: "the root authority withdrew it".to_owned(),
    })
}

#[test]
fn row_2_5_grant_revoke_withdraws_every_derived_grant_and_leaves_the_independent_one() -> TestResult
{
    let mut world = World::new()?;
    let (dana, tom, tom_agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let first = world.request(
        dana,
        root,
        tom,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let first = world.delegate(&first)?.event.grant();
    let second = world.request(tom, first, tom_agent, "tern", PassOn::UseOnly, None)?;
    let second = world.delegate(&second)?.event.grant();
    let independent_root =
        world.root(world.dana, "heron", pass(&["read", "write"], &BOTH)?, None)?;
    let sibling = world.request(dana, independent_root, tom, "heron", PassOn::UseOnly, None)?;
    let sibling = world.delegate(&sibling)?.event.grant();
    let derived = [(tom, first), (tom_agent, second)];
    for (holder, grant) in derived {
        let directory = world.directory.projection()?;
        let lineage = effective(world.grants.book(), directory, grant, world.now)?;
        assert_eq!(
            world
                .grants
                .book()
                .grant(grant)
                .map(lys_identity::grants::Grant::holder),
            Some(holder)
        );
        assert!(lineage.path.ends_with(&[root]));
    }
    assert_eq!(
        world.exercise(tom_agent, "read", Route::Tool)?.grant,
        second
    );
    assert!(world.grants.relationships_derived_from(root)? > 0);
    let request = revoke(&world, root)?;
    let now = world.now;
    world.grants.revoke(&request, now)?;
    for (_, grant) in derived {
        let directory = world.directory.projection()?;
        assert_eq!(
            effective(world.grants.book(), directory, grant, world.now),
            Err(GrantError::Revoked {
                grant: root.to_string()
            }),
            "the same action through {grant} is refused"
        );
    }
    assert_eq!(
        world.exercise(tom_agent, "read", Route::Tool),
        Err(GrantError::Revoked {
            grant: root.to_string()
        })
    );
    assert_eq!(
        world.grants.relationships_derived_from(root)?,
        0,
        "RELATIONSHIPS_AFTER_ROOT_REVOKE"
    );
    let control = world.exercise(tom, "read", Route::Api)?;
    assert_eq!(
        control.path,
        vec![sibling, independent_root],
        "the independent control succeeds"
    );
    assert!(world.exercise(tom, "write", Route::Api).is_ok());
    let onward = world.request(tom, first, tom_agent, "tern", PassOn::UseOnly, None)?;
    assert_eq!(
        world.delegate(&onward),
        Err(GrantError::Revoked {
            grant: root.to_string()
        })
    );
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        world.grants.relationships_derived_from(root)?,
        0,
        "and after a replay"
    );
    assert_eq!(
        world.events(),
        9,
        "six grant changes and three observed uses"
    );
    Ok(())
}

/// An in-process engine that can be paused: while paused, every write fails.
#[derive(Clone, Default)]
struct Pausable {
    inner: MemoryRelationships,
    paused: Arc<Mutex<bool>>,
}

impl Pausable {
    fn pause(&self, paused: bool) {
        *held(&self.paused) = paused;
    }
}

impl RelationshipStore for Pausable {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        self.inner.admit_resource(resource)
    }

    fn revision(&self) -> Result<u64, GrantError> {
        self.inner.revision()
    }
    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        if *held(&self.paused) {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: "projection paused".to_owned(),
            });
        }
        self.inner.write(revision, touch, delete)
    }
    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        self.inner.read()
    }
}

#[test]
fn grant_freshness_a_stale_engine_never_permits_after_a_committed_revoke() -> TestResult {
    let engine = Pausable::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
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
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    assert!(
        world.delegate(&lent).is_err(),
        "no end under a source that ends is refused, not clamped"
    );
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, Some(T0 + 1_000))?;
    let lent = world.delegate(&lent)?.event.grant();
    let before = world.exercise(tom, "read", Route::Api)?;
    assert_eq!(before.grant, lent);
    engine.pause(true);
    let request = revoke(&world, root)?;
    let now = world.now;
    let Err(GrantError::ProjectionPending { index, .. }) = world.grants.revoke(&request, now)
    else {
        return Err("a revoke committed while projection is paused is answered pending".into());
    };
    let required = index + 1;
    let exercise = ExerciseRequest {
        caller: tom,
        route: Route::Api,
        resource: alpha()?,
        action: actions(&["read"])?.into_iter().next().ok_or("no action")?,
    };
    let directory = world.directory.projection()?;
    assert_eq!(
        world
            .grants
            .check(directory, &exercise, now, Some(required)),
        Err(GrantError::StaleDecision {
            required,
            projected: engine.revision()?
        }),
        "a caller presenting the required revision gets no permit from the old state"
    );
    assert_eq!(
        world.grants.check(directory, &exercise, now, None),
        Err(GrantError::Revoked {
            grant: root.to_string()
        }),
        "nor does a caller presenting none"
    );
    assert_eq!(
        world.grants.relationships_derived_from(root)?,
        6,
        "the old relationships are still there"
    );
    engine.pause(false);
    world.reopen(engine)?;
    let directory = world.directory.projection()?;
    assert_eq!(
        world
            .grants
            .check(directory, &exercise, now, Some(required)),
        Err(GrantError::Revoked {
            grant: root.to_string()
        }),
        "after reopen the named revoked decision"
    );
    assert_eq!(world.grants.relationships_derived_from(root)?, 0);
    assert_eq!(
        world.grants.revoke(&request, now)?.index,
        index,
        "the revoke's retry answers it once"
    );
    assert_eq!(world.events(), required);
    Ok(())
}

#[test]
fn grant_engine_outage_refuses_by_name_and_writes_nothing() -> TestResult {
    let engine = FailingRelationships::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    world.delegate(&lent)?;
    let relationships = engine.read()?;
    let events = world.events();
    engine.set_down(true);
    for route in [Route::Browser, Route::Api, Route::Tool] {
        assert_eq!(
            world.exercise(tom, "read", route),
            Err(GrantError::PermissionEngineUnavailable {
                reason: "the permission engine is down".to_owned()
            }),
            "{route:?}: an outage is a named refusal, never a permit"
        );
    }
    assert_eq!(world.events(), events, "a refused check writes no use");
    engine.set_down(false);
    assert_eq!(
        engine.read()?,
        relationships,
        "nothing was written during the outage"
    );
    assert!(
        world.exercise(tom, "read", Route::Api).is_ok(),
        "the engine back, the same check permits"
    );
    Ok(())
}

/// The paused projection's original failure, as a degraded reading keeps it.
fn paused_by_name(permit: &lys_identity::grants::Permit, revision: u64) -> bool {
    permit.revision == revision
        && permit.degraded.as_ref().is_some_and(|degraded| {
            degraded.revision() == revision
                && degraded.error()
                    == &GrantError::PermissionEngineUnavailable {
                        reason: "projection paused".to_owned(),
                    }
        })
}

fn read_by(caller: IdentityId) -> Result<ExerciseRequest, Box<dyn Error>> {
    Ok(ExerciseRequest {
        caller,
        route: Route::Api,
        resource: alpha()?,
        action: actions(&["read"])?.into_iter().next().ok_or("no action")?,
    })
}

#[test]
fn control_1_a_degraded_reading_refuses_an_ancestor_revoke_and_keeps_an_independent_chain()
-> TestResult {
    let engine = Pausable::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
    let (dana, tom, tom_agent, lee) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
        IdentityId::Person(world.lee),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let first = world.request(
        dana,
        root,
        tom,
        "tern",
        pass(&["read"], &[RecipientKind::Agent])?,
        None,
    )?;
    let first = world.delegate(&first)?.event.grant();
    let second = world.request(tom, first, tom_agent, "tern", PassOn::UseOnly, None)?;
    let second = world.delegate(&second)?.event.grant();
    let independent = world.root(world.lee, "heron", pass(&["read"], &BOTH)?, None)?;
    let sibling = world.request(lee, independent, tom_agent, "tern", PassOn::UseOnly, None)?;
    let sibling = world.delegate(&sibling)?.event.grant();
    engine.pause(true);
    let request = revoke(&world, root)?;
    let now = world.now;
    let Err(GrantError::ProjectionPending { .. }) = world.grants.revoke(&request, now) else {
        return Err("the ancestor revoke commits while its projection fails".into());
    };
    let events = world.events();
    let asked = read_by(tom_agent)?;
    let directory = world.directory.projection()?;
    for refused in [
        world
            .grants
            .explain_by(directory, &asked, Some(second), now, None),
        world
            .grants
            .check_by(directory, &asked, Some(second), now, None),
    ] {
        assert_eq!(
            refused,
            Err(GrantError::Revoked {
                grant: root.to_string()
            }),
            "a degraded reading refuses the chain whose ancestor is revoked"
        );
    }
    assert_eq!(world.events(), events, "a refused chain records no use");
    let revision = engine.revision()?;
    let proved = world.grants.explain(directory, &asked, now, None)?;
    assert_eq!(
        (proved.grant, proved.path.clone()),
        (sibling, vec![sibling, independent]),
        "the fully proved independent chain is used"
    );
    assert!(
        paused_by_name(&proved, revision),
        "with the marker of its reading: {:?}",
        proved.degraded
    );
    assert_eq!(world.events(), events, "an explanation records nothing");
    engine.pause(false);
    world.reopen(engine)?;
    let directory = world.directory.projection()?;
    assert_eq!(
        world
            .grants
            .explain_by(directory, &asked, Some(second), now, None),
        Err(GrantError::Revoked {
            grant: root.to_string()
        }),
        "replay agrees"
    );
    let healthy = world.grants.explain(directory, &asked, now, None)?;
    assert_eq!(healthy.grant, sibling);
    assert!(healthy.degraded.is_none(), "a recovered reading is healthy");
    Ok(())
}

#[test]
fn control_1_a_one_time_hop_spent_under_a_degraded_reading_refuses_its_second_use() -> TestResult {
    let engine = Pausable::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
    let (dana, tom, lee) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Person(world.lee),
    );
    let once_root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Person])?,
        None,
    )?;
    let once = world.request(dana, once_root, tom, "tern", PassOn::UseOnly, None)?;
    let directory = world.directory.projection()?;
    let now = world.now;
    let once = world
        .grants
        .delegate_once(directory, &once, now)?
        .event
        .grant();
    let independent = world.root(world.lee, "heron", PassOn::UseOnly, None)?;
    let spare = world.root(world.dana, "tern", PassOn::UseOnly, None)?;
    engine.pause(true);
    let request = revoke(&world, spare)?;
    let Err(GrantError::ProjectionPending { .. }) = world.grants.revoke(&request, now) else {
        return Err("an unrelated revoke commits while its projection fails".into());
    };
    let revision = engine.revision()?;
    let events = world.events();
    let spent = world.exercise(tom, "read", Route::Api)?;
    assert_eq!(spent.grant, once);
    assert_eq!(spent.use_event, Some(Ok(events)), "the use spends it");
    assert!(paused_by_name(&spent, revision), "{:?}", spent.degraded);
    assert_eq!(world.events(), events + 1);
    assert_eq!(
        world.exercise(tom, "read", Route::Api),
        Err(GrantError::Revoked {
            grant: once.to_string()
        }),
        "a spent one-time hop refuses under the degraded reading"
    );
    assert_eq!(world.events(), events + 1, "the refusal records no use");
    let directory = world.directory.projection()?;
    let proved = world.grants.explain(directory, &read_by(lee)?, now, None)?;
    assert_eq!(proved.grant, independent, "another proved chain is used");
    assert!(paused_by_name(&proved, revision), "{:?}", proved.degraded);
    engine.pause(false);
    world.reopen(engine)?;
    assert_eq!(
        world.exercise(tom, "read", Route::Api),
        Err(GrantError::Revoked {
            grant: once.to_string()
        }),
        "replay agrees"
    );
    Ok(())
}
