#![cfg(test)]

//! R3: grants and their audit as one replayable operation (`GRANT_DURABILITY`
//! and `GRANT_IDEMPOTENCE`).

mod support;

#[path = "grant_faults/boundaries.rs"]
mod boundaries;

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Grant, GrantBook, GrantChange, GrantError, GrantEvent, GrantId, GrantParts,
    MemoryRelationships, PassOn, RecipientKind, Relation, Relationship, RelationshipStore,
    Resource, Route, Source, Window, sign_grant_event,
};
use lys_identity::log::Reopen;
use lys_identity::{IdentityId, OperationId};
use lys_log_store::{FileLeafStore, LeafStore, Log, PinnedRoot, StoreError, StoreResult};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

fn held<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match slot.lock() {
        Ok(held) => held,
        Err(error) => panic!("fixture_lock_poisoned: {error}"),
    }
}

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

/// Where the grant log's next append fails, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFault {
    None,
    BeforeLeaf,
    LeafStoredWriteFailed,
    AfterLeaf,
    AfterLeafUnreadable,
}

/// Where the permission engine's next write fails, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineFault {
    None,
    WriteRefused,
    StoredReportedFailed,
    Held,
}

type Plan<T> = Arc<Mutex<T>>;

fn get<T: Copy>(plan: &Plan<T>) -> T {
    *held(plan)
}

fn set<T>(plan: &Plan<T>, next: T) {
    *held(plan) = next;
}

fn injected(context: &str) -> StoreError {
    StoreError::Io {
        context: context.to_owned(),
        source: std::io::Error::other("injected"),
    }
}

/// A file store that fails where its plan says.
struct FaultStore {
    inner: FileLeafStore,
    plan: Plan<LogFault>,
}

impl LeafStore for FaultStore {
    fn origin(&self) -> &str {
        self.inner.origin()
    }
    fn extent(&self) -> u64 {
        self.inner.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }
    // The store writes leaves and pin as one act, so the boundaries a fault
    // can sit at are: before the act (nothing stored), and after the act's
    // flush with the answer lost (stored whole, the caller told it failed).
    // A leaf stored without its pin is not a state this store produces.
    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        match get(&self.plan) {
            LogFault::BeforeLeaf => {
                set(&self.plan, LogFault::None);
                Err(injected("leaf write"))
            }
            LogFault::LeafStoredWriteFailed => {
                set(&self.plan, LogFault::None);
                self.inner.append(index, leaves, pin)?;
                Err(injected("leaf durability"))
            }
            LogFault::AfterLeaf => {
                set(&self.plan, LogFault::None);
                self.inner.append(index, leaves, pin)?;
                Err(injected("pin write"))
            }
            LogFault::AfterLeafUnreadable => {
                self.inner.append(index, leaves, pin)?;
                Err(injected("pin write"))
            }
            LogFault::None => self.inner.append(index, leaves, pin),
        }
    }
    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        match get(&self.plan) {
            LogFault::AfterLeafUnreadable => Err(injected("pin write")),
            LogFault::None
            | LogFault::BeforeLeaf
            | LogFault::LeafStoredWriteFailed
            | LogFault::AfterLeaf => self.inner.pin(pin),
        }
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

/// An in-process engine whose writes fail where its plan says.
#[derive(Clone)]
struct FaultEngine {
    inner: MemoryRelationships,
    plan: Plan<EngineFault>,
}

impl FaultEngine {
    fn fresh() -> Self {
        Self {
            inner: MemoryRelationships::default(),
            plan: Arc::new(Mutex::new(EngineFault::None)),
        }
    }
}

impl RelationshipStore for FaultEngine {
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
        let refused = || GrantError::PermissionEngineUnavailable {
            reason: "injected".to_owned(),
        };
        match get(&self.plan) {
            EngineFault::None => self.inner.write(revision, touch, delete),
            EngineFault::WriteRefused => {
                set(&self.plan, EngineFault::None);
                Err(refused())
            }
            EngineFault::StoredReportedFailed => {
                set(&self.plan, EngineFault::None);
                self.inner.write(revision, touch, delete)?;
                Err(refused())
            }
            EngineFault::Held => Err(refused()),
        }
    }
    fn read_about(
        &self,
        subject: lys_identity::IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        let held = self.read()?;
        lys_identity::grants::about::select(
            subject,
            resource,
            |object| {
                Ok(held
                    .iter()
                    .filter(|row| row.subject == *object)
                    .cloned()
                    .collect())
            },
            |object| {
                Ok(held
                    .iter()
                    .filter(|row| row.resource == *object)
                    .cloned()
                    .collect())
            },
        )
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        self.inner.read()
    }
}

type FaultWorld = World<FaultStore, FaultEngine>;

fn world(plan: &Plan<LogFault>, engine: &FaultEngine) -> Result<FaultWorld, Box<dyn Error>> {
    let plan = Arc::clone(plan);
    World::with(
        Box::new(move |path: &Path| -> Reopen<FaultStore> {
            let (path, plan) = (path.to_owned(), Arc::clone(&plan));
            Box::new(move || {
                if get(&plan) == LogFault::AfterLeafUnreadable {
                    return Err(injected("reopen"));
                }
                Ok(FaultStore {
                    inner: FileLeafStore::open(&path)?,
                    plan: Arc::clone(&plan),
                })
            })
        }),
        engine.clone(),
    )
}

/// What a reader sees: the book and every relationship.
fn state(world: &FaultWorld) -> Result<(GrantBook, BTreeSet<Relationship>), Box<dyn Error>> {
    Ok((
        world.grants.book().clone(),
        world.grants.relationships().read()?,
    ))
}

/// Everyone in the world, as the grants name them.
fn everyone(world: &FaultWorld) -> [IdentityId; 5] {
    [
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Person(world.lee),
        IdentityId::Agent(world.tom_agent),
        IdentityId::Agent(world.dana_agent),
    ]
}

/// Refuse unless the live grants equal a replay of their log into a fresh engine.
fn equals_replay(world: &mut FaultWorld, live: &FaultEngine) -> TestResult {
    let decisions = everyone(world).map(|identity| world.exercise(identity, "read", Route::Api));
    let before = state(world)?;
    world.reopen(FaultEngine::fresh())?;
    assert_eq!(
        state(world)?,
        before,
        "the answered state is the replayed state"
    );
    let replayed = everyone(world).map(|identity| world.exercise(identity, "read", Route::Api));
    let strip = |decision: &Result<lys_identity::grants::Permit, GrantError>| {
        decision
            .as_ref()
            .map(|permit| permit.path.clone())
            .map_err(Clone::clone)
    };
    assert_eq!(replayed.map(|d| strip(&d)), decisions.map(|d| strip(&d)));
    world.reopen(live.clone())?;
    Ok(())
}

/// A grant ending after its source, signed and appended without admission.
fn past_its_source(world: &FaultWorld, source: GrantId) -> Result<Vec<u8>, Box<dyn Error>> {
    let operation = OperationId::generate()?;
    let dana = IdentityId::Person(world.dana);
    let grant = Grant::new(GrantParts {
        id: GrantId::generate()?,
        issuer: dana,
        holder: IdentityId::Person(world.tom),
        responsible: world.tom,
        resource: alpha()?,
        relation: Relation::new("tern")?,
        actions: actions(&["read"])?,
        pass_on: PassOn::UseOnly,
        source: Source::Grant(source),
        window: Window::new(T0, Some(T0 + 5_000))?,
        model_version: 1,
        operation,
    })?;
    let event = GrantEvent::new(
        operation,
        dana,
        world.now,
        GrantChange::Issue(Box::new(grant)),
    )?;
    let key = Ed25519Identity::load(&world.dir.path().join("service.key"))?;
    Ok(sign_grant_event(event, &key)?.bytes().to_vec())
}

const LEGS: [&str; 8] = [
    "the leaf write fails before any byte is stored",
    "the leaf is stored and its write is reported failed",
    "the leaf is stored and the pin write fails",
    "the leaf is stored, the pin write fails and the log is unreadable until cleared",
    "the permission write is refused, then held",
    "the permission write is stored and reported failed",
    "the service restarts between commit and projection",
    "a committed event past its source's end is refused by the projector",
];

#[test]
fn row_2_6_grant_durability_every_boundary_answers_replay_or_names_what_is_unresolved() -> TestResult
{
    let mut exercised = 0;
    for (leg, name) in LEGS.iter().enumerate() {
        let plan = Arc::new(Mutex::new(LogFault::None));
        let engine = FaultEngine::fresh();
        let mut world = world(&plan, &engine)?;
        let (dana, tom, lee) = (
            IdentityId::Person(world.dana),
            IdentityId::Person(world.tom),
            IdentityId::Person(world.lee),
        );
        let root = world.root(
            world.dana,
            "kite",
            pass(&["read"], &BOTH)?,
            Some(T0 + 1_000),
        )?;
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let request = world.request(dana, root, tom, "tern", PassOn::UseOnly, Some(T0 + 1_000))?;
        let control = |world: &mut FaultWorld| world.exercise(lee, "read", Route::Api).map(|_| ());
        if leg <= 3 {
            boundaries::assert_log_boundary(&mut world, &plan, &request, leg, name)?;
        } else {
            boundaries::assert_projection_boundary(&mut world, &engine, root, &request, leg, name)?;
        }
        control(&mut world)?;
        equals_replay(&mut world, &engine)?;
        exercised += 1;
    }
    assert_eq!(exercised, LEGS.len(), "every boundary was exercised");
    Ok(())
}

#[test]
fn grant_idempotence_a_lost_acknowledgement_retries_to_one_grant() -> TestResult {
    let plan = Arc::new(Mutex::new(LogFault::None));
    let engine = FaultEngine::fresh();
    let mut world = world(&plan, &engine)?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let request = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    set(&plan, LogFault::AfterLeaf);
    let first = world.delegate(&request)?;
    let retried = world.delegate(&request)?;
    world.reopen(engine)?;
    let reopened = world.delegate(&request)?;
    assert_eq!(retried, first, "the retry answers the first receipt");
    assert_eq!(reopened, first, "so does a retry after a restart");
    assert_eq!(world.events(), 2, "one root and one grant");
    let issued = world
        .grants
        .events()?
        .iter()
        .filter(|(signed, _)| signed.event().operation() == request.operation)
        .count();
    assert_eq!(issued, 1, "one authorising event");
    let before = state(&world)?;
    let mut changed = request.clone();
    changed.relation = Relation::new("kite")?;
    assert_eq!(
        world.delegate(&changed),
        Err(GrantError::OperationReused {
            operation: request.operation.to_string()
        })
    );
    assert_eq!(
        state(&world)?,
        before,
        "a changed payload changes neither projection"
    );
    assert_eq!(
        world.exercise(tom, "read", Route::Api)?.grant,
        first.event.grant()
    );
    Ok(())
}

#[test]
fn grant_durability_the_fault_free_path_equals_replay() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let request = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    let recorded = world.delegate(&request)?;
    let permit = world.exercise(tom, "read", Route::Browser)?;
    let before = (
        world.grants.book().clone(),
        world.grants.relationships().read()?,
    );
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        (
            world.grants.book().clone(),
            world.grants.relationships().read()?
        ),
        before
    );
    let again = world.exercise(tom, "read", Route::Browser)?;
    assert_eq!((again.grant, again.path), (permit.grant, permit.path));
    assert_eq!(world.delegate(&request)?, recorded);
    Ok(())
}
