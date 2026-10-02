//! Who may act across many resources is answered from one reading of the
//! permission engine, with complete answers independent of wall-clock speed. An engine that
//! takes a write and does not move is named after that one write, never
//! written to again and again while the directory waits.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use lys_identity::grants::{
    Action, DelegateRequest, ExerciseRequest, GrantError, GrantId, MemoryRelationships, PassOn,
    RecipientKind, Relation, Relationship, RelationshipStore, Resource, RootRequest, Route, Window,
};
use lys_identity::log::Reopen;
use lys_identity::{IdentityId, OperationId};
use lys_log_store::FileLeafStore;
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

/// An engine that counts relationship reads and names an injected read failure.
#[derive(Debug, Clone, Default)]
struct Measured {
    inner: MemoryRelationships,
    broken: Arc<AtomicBool>,
    reads: Arc<AtomicUsize>,
}

impl RelationshipStore for Measured {
    fn revision(&self) -> Result<u64, GrantError> {
        self.inner.revision()
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        self.inner.write(revision, touch, delete)
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.broken.load(Ordering::SeqCst) {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: "the relationships could not be read".to_owned(),
            });
        }
        self.inner.read()
    }
}

/// Twenty projects, each held by Dana at its root and lent on to Tom, Lee
/// and Tom's agent.
fn twenty(world: &mut World<FileLeafStore, Measured>) -> Result<Vec<Resource>, Box<dyn Error>> {
    let mut resources = Vec::new();
    for n in 0..20 {
        let resource = Resource::new("project", &format!("p{n:02}"))?;
        let directory = world.directory.projection()?;
        let root = RootRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(world.admin),
            route: Route::Api,
            holder: world.dana,
            resource: resource.clone(),
            relation: Relation::new("kite")?,
            // Delete is withheld from agents, so a root reaching agents
            // passes on only what heron and tern carry.
            pass_on: pass(
                &["read", "write"],
                &[RecipientKind::Person, RecipientKind::Agent],
            )?,
            window: Window::new(T0, None)?,
        };
        let root = world
            .grants
            .issue_root(directory, &root, world.now)?
            .event
            .grant();
        let lent = [
            (IdentityId::Person(world.tom), world.tom, "heron"),
            (IdentityId::Person(world.lee), world.lee, "tern"),
            (IdentityId::Agent(world.tom_agent), world.tom, "tern"),
        ];
        for (recipient, responsible, relation) in lent {
            let request = DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(world.dana),
                route: Route::Api,
                source: root,
                recipient,
                responsible,
                resource: resource.clone(),
                relation: Relation::new(relation)?,
                pass_on: PassOn::UseOnly,
                window: Window::new(T0, None)?,
            };
            let directory = world.directory.projection()?;
            world.grants.delegate(directory, &request, world.now)?;
        }
        resources.push(resource);
    }
    Ok(resources)
}

type Reach = BTreeMap<(Resource, IdentityId, &'static str), GrantId>;

/// Every holder of each resource, each action, and the grant that allows it.
fn reach(
    world: &mut World<FileLeafStore, Measured>,
    resources: &[Resource],
) -> Result<Reach, Box<dyn Error>> {
    let directory = world.directory.projection()?;
    let frame = world.grants.frame(directory, None)?;
    let mut out = BTreeMap::new();
    for resource in resources {
        let holders: BTreeSet<IdentityId> = world
            .grants
            .book()
            .on_resource(resource)
            .map(|record| record.grant().holder())
            .collect();
        for holder in holders {
            for action in ["delete", "read", "write"] {
                let request = ExerciseRequest {
                    caller: holder,
                    route: Route::Browser,
                    resource: resource.clone(),
                    action: Action::new(action)?,
                };
                if let Ok(permit) = world.grants.explain_in(&frame, &request, world.now) {
                    out.insert((resource.clone(), holder, action), permit.grant);
                }
            }
        }
    }
    Ok(out)
}

#[test]
fn twenty_resources_are_answered_from_one_engine_read() -> TestResult {
    let engine = Measured::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
    let resources = twenty(&mut world)?;
    engine.reads.store(0, Ordering::SeqCst);
    let answered = reach(&mut world, &resources)?;
    assert_eq!(
        engine.reads.load(Ordering::SeqCst),
        1,
        "one read of the engine"
    );
    assert_eq!(
        answered.len(),
        20 * (3 + 2 + 1 + 1),
        "every holder's every allowed action"
    );
    world.reopen(engine)?;
    assert_eq!(
        reach(&mut world, &resources)?,
        answered,
        "the same after a restart"
    );
    let directory = world.directory.projection()?;
    for ((resource, holder, action), grant) in &answered {
        let request = ExerciseRequest {
            caller: *holder,
            route: Route::Browser,
            resource: resource.clone(),
            action: Action::new(action)?,
        };
        let permit = world.grants.explain(directory, &request, world.now, None)?;
        assert_eq!(permit.grant, *grant, "{resource} {holder} {action}");
    }
    Ok(())
}

#[test]
fn a_frame_answers_each_question_as_check_does_and_records_nothing() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Person])?,
        None,
    )?;
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    world.delegate(&lent)?;
    let directory = world.directory.projection()?;
    let frame = world.grants.frame(directory, None)?;
    let before = world.grants.revision();
    let mut answers = Vec::new();
    for caller in [dana, tom] {
        for action in actions(&["delete", "read", "write"])? {
            let request = ExerciseRequest {
                caller,
                route: Route::Browser,
                resource: alpha()?,
                action: action.clone(),
            };
            let answer = world
                .grants
                .explain_in(&frame, &request, world.now)
                .map(|permit| permit.path);
            answers.push((caller, action, answer));
        }
    }
    assert_eq!(
        world.grants.revision(),
        before,
        "a question records nothing"
    );
    for (caller, action, answer) in answers {
        let checked = world
            .exercise(caller, action.as_str(), Route::Browser)
            .map(|permit| permit.path);
        assert_eq!(answer, checked, "{caller} {action}");
    }
    Ok(())
}

#[test]
fn a_frame_over_a_failed_read_is_refused_and_a_moved_frame_is_stale() -> TestResult {
    let engine = Measured::default();
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
        pass(&["read"], &[RecipientKind::Person])?,
        None,
    )?;
    engine.broken.store(true, Ordering::SeqCst);
    let directory = world.directory.projection()?;
    assert!(
        matches!(
            world.grants.frame(directory, None),
            Err(GrantError::PermissionEngineUnavailable { .. })
        ),
        "no frame over a read that failed"
    );
    engine.broken.store(false, Ordering::SeqCst);
    let request = ExerciseRequest {
        caller: dana,
        route: Route::Browser,
        resource: alpha()?,
        action: Action::new("read")?,
    };
    let directory = world.directory.projection()?.clone();
    let frame = world.grants.frame(&directory, None)?;
    let before = world.grants.explain_in(&frame, &request, world.now)?;
    assert_eq!(before.grant, root);
    let recorded = world.events();
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    let directory = world.directory.projection()?;
    world.grants.delegate(directory, &lent, world.now)?;
    assert!(
        matches!(
            world.grants.explain_in(&frame, &request, world.now),
            Err(GrantError::StaleDecision { .. })
        ),
        "a frame is not mixed with a later grant event"
    );
    assert_eq!(
        world.events(),
        recorded + 1,
        "the pass-on is the later event"
    );
    Ok(())
}

/// Writes a stalled engine takes before it refuses on its own, so that a
/// projection that never stops writing fails here instead of never ending.
const PATIENCE: usize = 3;

/// An engine that, while `standing` is set, takes every write and keeps
/// nothing, counting each write it is asked for.
#[derive(Debug, Clone, Default)]
struct Stalled {
    inner: MemoryRelationships,
    standing: Arc<AtomicBool>,
    writes: Arc<AtomicUsize>,
}

impl RelationshipStore for Stalled {
    fn revision(&self) -> Result<u64, GrantError> {
        self.inner.revision()
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        if !self.standing.load(Ordering::SeqCst) {
            return self.inner.write(revision, touch, delete);
        }
        let asked = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
        if asked > PATIENCE {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: format!("the stalled engine was asked for {asked} writes"),
            });
        }
        Ok(())
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        self.inner.read()
    }
}

#[test]
fn an_engine_whose_revision_stands_still_after_a_write_is_named_after_one_write() -> TestResult {
    let engine = Stalled::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
    engine.standing.store(true, Ordering::SeqCst);
    let refused = world.root(world.dana, "kite", PassOn::UseOnly, None);
    let named = refused.err().map(|error| error.to_string());
    assert!(
        named
            .as_deref()
            .is_some_and(|words| words.starts_with("ProjectionPending")),
        "a grant the engine did not take is pending: {named:?}"
    );
    assert_eq!(engine.writes.load(Ordering::SeqCst), 1, "written once");

    engine.writes.store(0, Ordering::SeqCst);
    let stalled = world.grants.project();
    let reason = match stalled {
        Err(GrantError::PermissionEngineUnavailable { reason }) => reason,
        other => return Err(format!("expected the stall named, got {other:?}").into()),
    };
    assert!(
        reason.contains("did not move"),
        "the stall is named: {reason}"
    );
    assert_eq!(engine.writes.load(Ordering::SeqCst), 1, "written once more");

    engine.standing.store(false, Ordering::SeqCst);
    let projected = world.grants.project()?;
    assert!(projected > 0, "an engine that moves again is caught up");
    Ok(())
}
