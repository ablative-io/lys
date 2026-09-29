//! Who may act across many resources is answered from one reading of the
//! permission engine, and a graph of twenty resources is answered in under
//! a second against an engine as slow as a local `SpiceDB`.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use lys_identity::grants::{
    Action, DelegateRequest, ExerciseRequest, GrantError, GrantId, MemoryRelationships, PassOn,
    RecipientKind, Relation, Relationship, RelationshipStore, Resource, RootRequest, Route, Window,
};
use lys_identity::log::Reopen;
use lys_identity::{IdentityId, OperationId};
use lys_log_store::FileLeafStore;
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

/// What one read of every relationship took on a local `SpiceDB` holding a
/// hundred grants: a revision, two schema reads and seven kinds.
const READ: Duration = Duration::from_millis(47);
/// What one revision read took there.
const REVISION: Duration = Duration::from_millis(4);

/// An engine that answers as slowly as a local `SpiceDB` while `slow` is
/// set, counts every read of the relationships, and refuses that read
/// alone while `broken` is set.
#[derive(Debug, Clone, Default)]
struct Measured {
    inner: MemoryRelationships,
    slow: Arc<AtomicBool>,
    broken: Arc<AtomicBool>,
    reads: Arc<AtomicUsize>,
}

impl Measured {
    fn wait(&self, cost: Duration) {
        if self.slow.load(Ordering::SeqCst) {
            std::thread::sleep(cost);
        }
    }
}

impl RelationshipStore for Measured {
    fn revision(&self) -> Result<u64, GrantError> {
        self.wait(REVISION);
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
        self.wait(READ);
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
            pass_on: pass(
                &["delete", "read", "write"],
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
fn twenty_resources_are_answered_from_one_read_in_under_a_second() -> TestResult {
    let engine = Measured::default();
    let mut world = World::with(
        Box::new(|path: &Path| -> Reopen<FileLeafStore> {
            let path = path.to_owned();
            Box::new(move || FileLeafStore::open(&path))
        }),
        engine.clone(),
    )?;
    let resources = twenty(&mut world)?;
    engine.slow.store(true, Ordering::SeqCst);
    engine.reads.store(0, Ordering::SeqCst);
    let started = Instant::now();
    let answered = reach(&mut world, &resources)?;
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(1),
        "twenty resources took {took:?}"
    );
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
    engine.slow.store(false, Ordering::SeqCst);
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
