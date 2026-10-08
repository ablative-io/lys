//! Original settlement failures, selected revisions and retained opening state.

#![cfg(test)]

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use lys_identity::IdentityId;
use lys_identity::grants::{
    ExerciseRequest, GrantError, MemoryRelationships, PassOn, RecipientKind, Relationship,
    RelationshipStore, Resource, Route,
};
use lys_identity::log::Reopen;
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreError, StoreResult};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;
const WRITE_FAILURE: &str = "settlement-fixture-write-refused";
const READ_FAILURE: &str = "settlement-fixture-revision-unavailable";
const LOG_FAILURE: &str = "settlement-fixture-reopen-unavailable";

fn finish_world<S: LeafStore, R: RelationshipStore, T>(
    world: World<S, R>,
    result: Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    let dir = Arc::clone(&world.dir);
    drop(world);
    let cleanup = Arc::try_unwrap(dir)
        .map_err(|shared| {
            format!("fixture directory still has {} owners", Arc::strong_count(&shared))
        })
        .and_then(|dir| dir.close().map_err(|error| error.to_string()));
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(error), Err(cleanup)) => {
            Err(format!("fixture act failed: {error}; fixture cleanup failed: {cleanup}").into())
        }
    }
}

#[derive(Default)]
struct Faults {
    writes: AtomicU64,
    write: AtomicBool,
    read_after_write: AtomicBool,
    revision: AtomicBool,
    lost_acknowledgement: AtomicBool,
    log_unreadable: AtomicBool,
    advance_on_read: AtomicBool,
    write_then_fail: AtomicBool,
    revisions: AtomicU64,
    reads: AtomicU64,
    appends: AtomicU64,
    pins: AtomicU64,
    snapshots: AtomicU64,
    leaves: AtomicU64,
    snapshot_reads: AtomicU64,
    reopens: AtomicU64,
}

#[derive(Clone, Default)]
struct Engine {
    inner: MemoryRelationships,
    faults: Arc<Faults>,
}

impl RelationshipStore for Engine {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        self.inner.admit_resource(resource)
    }

    fn revision(&self) -> Result<u64, GrantError> {
        self.faults.revisions.fetch_add(1, Ordering::Relaxed);
        if self.faults.revision.load(Ordering::Relaxed) {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: READ_FAILURE.to_owned(),
            });
        }
        self.inner.revision()
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        self.faults.writes.fetch_add(1, Ordering::Relaxed);
        if self.faults.write.load(Ordering::Relaxed) {
            if self.faults.write_then_fail.load(Ordering::Relaxed) {
                self.inner.write(revision, touch, delete)?;
            }
            if self.faults.read_after_write.load(Ordering::Relaxed) {
                self.faults.revision.store(true, Ordering::Relaxed);
            }
            return Err(GrantError::PermissionEngineUnavailable {
                reason: WRITE_FAILURE.to_owned(),
            });
        }
        self.inner.write(revision, touch, delete)
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        self.faults.reads.fetch_add(1, Ordering::Relaxed);
        let held = self.inner.read()?;
        if self.faults.advance_on_read.swap(false, Ordering::Relaxed) {
            let mut writer = self.inner.clone();
            let revision = writer.revision()?.checked_add(1)
                .ok_or_else(|| GrantError::PermissionEngineUnavailable {
                    reason: "fixture relationship revision overflowed".to_owned(),
                })?;
            writer.write(revision, &[], &[])?;
        }
        Ok(held)
    }
}

struct Store {
    inner: FileLeafStore,
    faults: Arc<Faults>,
}

fn log_failure() -> StoreError {
    StoreError::Io {
        context: LOG_FAILURE.to_owned(),
        source: std::io::Error::other(LOG_FAILURE),
    }
}

impl LeafStore for Store {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn extent(&self) -> u64 {
        self.inner.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.faults.leaves.fetch_add(1, Ordering::Relaxed);
        self.inner.leaf(index)
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        self.faults.appends.fetch_add(1, Ordering::Relaxed);
        self.inner.append(index, leaves, pin)?;
        if self
            .faults
            .lost_acknowledgement
            .swap(false, Ordering::Relaxed)
        {
            self.faults.log_unreadable.store(true, Ordering::Relaxed);
            return Err(log_failure());
        }
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.faults.pins.fetch_add(1, Ordering::Relaxed);
        self.inner.pin(pin)
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.faults.snapshot_reads.fetch_add(1, Ordering::Relaxed);
        self.inner.snapshot()
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.faults.snapshots.fetch_add(1, Ordering::Relaxed);
        self.inner.put_snapshot(bytes)
    }
}

fn world(engine: &Engine) -> Result<World<Store, Engine>, Box<dyn Error>> {
    let faults = Arc::clone(&engine.faults);
    World::with(
        Box::new(move |path: &Path| -> Reopen<Store> {
            let path = path.to_owned();
            let faults = Arc::clone(&faults);
            Box::new(move || {
                faults.reopens.fetch_add(1, Ordering::Relaxed);
                if faults.log_unreadable.load(Ordering::Relaxed) {
                    return Err(log_failure());
                }
                Ok(Store {
                    inner: FileLeafStore::open(&path)?,
                    faults: Arc::clone(&faults),
                })
            })
        }),
        engine.clone(),
    )
}

#[derive(Clone, Default)]
struct Observations {
    events: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
    spans: Arc<AtomicU64>,
    callbacks: Arc<AtomicU64>,
    locks: Arc<AtomicU64>,
    fields: Arc<AtomicU64>,
    bytes: Arc<AtomicU64>,
    nanos: Arc<AtomicU64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ObservationCosts {
    callbacks: u64,
    locks: u64,
    fields: u64,
    bytes: u64,
    nanos: u64,
}

fn observation_count(value: usize) -> u64 {
    match u64::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("observation_fixture_count_overflowed: {error}"),
    }
}

struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }
}

impl Subscriber for Observations {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= tracing::Level::WARN
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut fields = Fields(BTreeMap::new());
        attributes.record(&mut fields);
        Id::from_u64(self.spans.fetch_add(1, Ordering::Relaxed) + 1)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut fields = Fields(BTreeMap::new());
        values.record(&mut fields);
        assert_ne!(span.into_u64(), 0);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        assert_ne!(span.into_u64(), 0);
        assert_ne!(follows.into_u64(), 0);
    }

    fn event(&self, event: &Event<'_>) {
        let started = Instant::now();
        self.callbacks.fetch_add(1, Ordering::Relaxed);
        let mut fields = Fields(BTreeMap::new());
        event.record(&mut fields);
        self.fields.fetch_add(observation_count(fields.0.len()), Ordering::Relaxed);
        for (name, value) in &fields.0 {
            self.bytes.fetch_add(observation_count(name.len()), Ordering::Relaxed);
            self.bytes.fetch_add(observation_count(value.len()), Ordering::Relaxed);
        }
        self.locks.fetch_add(1, Ordering::Relaxed);
        match self.events.lock() {
            Ok(mut events) => events.push(fields.0),
            Err(error) => panic!("observation_fixture_lock_poisoned: {error}"),
        }
        let nanos = match u64::try_from(started.elapsed().as_nanos()) {
            Ok(nanos) => nanos,
            Err(error) => panic!("observation_fixture_duration_overflowed: {error}"),
        };
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    fn enter(&self, span: &Id) {
        assert_ne!(span.into_u64(), 0);
    }

    fn exit(&self, span: &Id) {
        assert_ne!(span.into_u64(), 0);
    }
}

impl Observations {
    fn costs(&self) -> ObservationCosts {
        ObservationCosts {
            callbacks: self.callbacks.load(Ordering::Relaxed),
            locks: self.locks.load(Ordering::Relaxed),
            fields: self.fields.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            nanos: self.nanos.load(Ordering::Relaxed),
        }
    }

    fn read(&self) -> Result<Vec<BTreeMap<String, String>>, Box<dyn Error>> {
        match self.events.lock() {
            Ok(events) => Ok(events.clone()),
            Err(error) => Err(format!("observation_fixture_lock_poisoned: {error}").into()),
        }
    }
}

fn observed(
    events: &[BTreeMap<String, String>],
    step: &str,
    cause: &str,
    revision: u64,
) -> bool {
    events.iter().any(|fields| {
        fields.get("step").is_some_and(|value| value == step)
            && fields
                .values()
                .any(|value| value.contains(cause) && value.contains("PermissionEngineUnavailable"))
            && fields
                .get("revision")
                .is_some_and(|value| value == &revision.to_string())
    })
}

fn explain(world: &mut World<Store, Engine>) -> Result<lys_identity::grants::Permit, GrantError> {
    let request = ExerciseRequest {
        caller: IdentityId::Person(world.lee),
        route: Route::Api,
        resource: alpha()?,
        action: lys_identity::grants::Action::new("read")?,
    };
    let directory = world.directory.projection()?;
    world.grants.explain(directory, &request, world.now, None)
}

#[test]
fn projection_failure_keeps_unrelated_authority_and_names_its_actual_revision() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let healthy = explain(&mut world)?;
    engine.faults.write.store(true, Ordering::Relaxed);
    let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
    let revision = engine.inner.revision()?;
    let writes = engine.faults.writes.load(Ordering::Relaxed);
    let capture = Observations::default();
    let (result, repeated) = tracing::subscriber::with_default(capture.clone(), || {
        (explain(&mut world), explain(&mut world))
    });
    engine.faults.write.store(false, Ordering::Relaxed);
    let events = capture.read()?;
    assert!(matches!(
        pending.as_ref().err().and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })
    ));
    assert_eq!(result?.grant, healthy.grant);
    assert_eq!(repeated?.grant, healthy.grant);
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    let failures = engine.faults.writes.load(Ordering::Relaxed) - writes;
    assert_eq!(failures, 2);
    assert_eq!(
        u64::try_from(events.iter().filter(|fields| {
            fields.get("step").is_some_and(|step| step == "project")
                && fields.values().any(|value| value.contains(WRITE_FAILURE))
        }).count())?,
        failures,
        "every failed attempt keeps its own observation"
    );
    Ok(())
}

#[test]
fn unreadable_reconciliation_refuses_unrelated_authority_with_the_original_cause() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let kinds = [RecipientKind::Person, RecipientKind::Agent];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    explain(&mut world)?;
    let request = world.request(
        IdentityId::Person(world.dana),
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    engine.faults.lost_acknowledgement.store(true, Ordering::Relaxed);
    let pending = world.delegate(&request);
    let before = FileLeafStore::open(&world.dir.path().join("grants"))?.extent();
    let result = explain(&mut world);
    engine.faults.log_unreadable.store(false, Ordering::Relaxed);
    let after = FileLeafStore::open(&world.dir.path().join("grants"))?.extent();
    assert!(matches!(pending, Err(GrantError::OperationUnresolved { .. })));
    assert!(matches!(result, Err(GrantError::LogUnavailable { reason }) if reason.contains(LOG_FAILURE)));
    assert_eq!(after, before, "a refused explanation appends no use event");
    Ok(())
}

#[test]
fn startup_retains_the_original_projection_failure_and_selected_revision() -> TestResult {
    let original = Engine::default();
    let mut world = world(&original)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let replacement = Engine::default();
    replacement.faults.write.store(true, Ordering::Relaxed);
    let revision = replacement.inner.revision()?;
    let capture = Observations::default();
    let opened = tracing::subscriber::with_default(capture.clone(), || world.reopen(replacement.clone()));
    replacement.faults.write.store(false, Ordering::Relaxed);
    let reading = (|| -> Result<_, Box<dyn Error>> {
        opened?;
        let writes = replacement.faults.writes.load(Ordering::Relaxed);
        let degraded = world.grants.take_startup_degradation()
            .ok_or("opening failure was not retained")?;
        let consumed = world.grants.take_startup_degradation().is_none();
        Ok((capture.read()?, degraded, consumed, writes,
            replacement.faults.writes.load(Ordering::Relaxed)))
    })();
    let (events, degraded, consumed, before, after) = finish_world(world, reading)?;
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    assert_eq!(degraded.revision(), revision);
    assert_eq!(degraded.error(), &GrantError::PermissionEngineUnavailable {
        reason: WRITE_FAILURE.to_owned(),
    });
    assert!(consumed);
    assert_eq!(before, after, "consuming the opening failure retries no projection");
    Ok(())
}

#[test]
fn healthy_settlement_keeps_no_degradation_and_emits_no_failure() -> TestResult {
    let capture = Observations::default();
    let mut world = tracing::subscriber::with_default(capture.clone(), World::new)?;
    let readings = tracing::subscriber::with_default(capture.clone(), || -> Result<_, Box<dyn Error>> {
        let grant = world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let before = world.events();
        let healthy = world.grants.frame(world.directory.projection()?, None)?
            .degradation().is_none();
        let permit = world.exercise(IdentityId::Person(world.lee), "read", Route::Api)?;
        Ok((grant, permit, healthy, world.grants.take_startup_degradation().is_none(),
            world.now, before, world.events(), capture.read()?))
    });
    let (grant, permit, healthy, opening, now, before, after, events) = finish_world(world, readings)?;
    assert_eq!(permit.grant, grant);
    assert_eq!(permit.actions, actions(&["read"])?);
    assert_eq!(now, T0);
    assert_eq!(after, before + 1);
    assert!(healthy);
    assert!(opening);
    assert!(events.is_empty(), "{events:?}");
    Ok(())
}

#[test]
fn degraded_frame_retains_the_selected_cause_and_recovers_without_a_marker() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let capture = Observations::default();
    let readings = tracing::subscriber::with_default(capture.clone(), || -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let healthy = world.grants.frame(world.directory.projection()?, None)?
            .degradation().is_none();
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        let revision = engine.inner.revision()?;
        let before = world.events();
        let degraded = {
            let frame = world.grants.frame(world.directory.projection()?, None)?;
            Arc::clone(frame.degradation().ok_or("projection failure lost its reading marker")?)
        };
        engine.faults.write.store(false, Ordering::Relaxed);
        let recovered = world.grants.frame(world.directory.projection()?, None)?
            .degradation().is_none();
        Ok((pending, healthy, degraded, recovered, revision, before, world.events(), capture.read()?))
    });
    engine.faults.write.store(false, Ordering::Relaxed);
    let (pending, healthy, degraded, recovered, revision, before, after, events) = finish_world(world, readings)?;
    assert!(matches!(pending.as_ref().err().and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })));
    assert!(healthy);
    assert!(recovered);
    assert_eq!(degraded.revision(), revision);
    assert_eq!(degraded.error(), &GrantError::PermissionEngineUnavailable {
        reason: WRITE_FAILURE.to_owned(),
    });
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn failed_frame_revision_read_returns_the_read_cause_and_observes_both_failures() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let capture = Observations::default();
    let readings = (|| -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        let before = world.events();
        engine.faults.read_after_write.store(true, Ordering::Relaxed);
        let directory = world.directory.projection()?;
        let frame = tracing::subscriber::with_default(capture.clone(), || {
            world.grants.frame(directory, None)
                .map(|frame| frame.revision())
        });
        Ok((pending, frame, before, world.events(), capture.read()?))
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    engine.faults.revision.store(false, Ordering::Relaxed);
    engine.faults.read_after_write.store(false, Ordering::Relaxed);
    let (pending, frame, before, after, events) = finish_world(world, readings)?;
    assert!(matches!(pending.as_ref().err().and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })));
    assert!(matches!(frame, Err(GrantError::PermissionEngineUnavailable { reason }) if reason == READ_FAILURE));
    assert_eq!(before, after);
    for (step, cause) in [("project", WRITE_FAILURE), ("revision", READ_FAILURE)] {
        assert!(events.iter().any(|fields| fields.get("step").is_some_and(|value| value == step)
            && fields.values().any(|value| value.contains(cause))), "{events:?}");
    }
    Ok(())
}

#[test]
fn mutation_acknowledgement_retains_the_projection_cause_and_real_revision() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let kinds = [RecipientKind::Person, RecipientKind::Agent];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    let request = world.request(IdentityId::Person(world.dana), root, IdentityId::Person(world.tom), "tern", PassOn::UseOnly, None)?;
    let revision = engine.inner.revision()?;
    let capture = Observations::default();
    engine.faults.write.store(true, Ordering::Relaxed);
    let answer = tracing::subscriber::with_default(capture.clone(), || world.delegate(&request));
    engine.faults.write.store(false, Ordering::Relaxed);
    let events = capture.read()?;
    assert!(matches!(answer, Err(GrantError::ProjectionPending { index: 1, .. })));
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    Ok(())
}

#[test]
fn failed_mutation_revision_read_is_named_instead_of_inventing_revision_zero() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let kinds = [RecipientKind::Person, RecipientKind::Agent];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    let request = world.request(IdentityId::Person(world.dana), root, IdentityId::Person(world.tom), "tern", PassOn::UseOnly, None)?;
    let capture = Observations::default();
    engine.faults.write.store(true, Ordering::Relaxed);
    engine.faults.read_after_write.store(true, Ordering::Relaxed);
    let answer = tracing::subscriber::with_default(capture.clone(), || world.delegate(&request));
    engine.faults.write.store(false, Ordering::Relaxed);
    engine.faults.read_after_write.store(false, Ordering::Relaxed);
    engine.faults.revision.store(false, Ordering::Relaxed);
    let events = capture.read()?;
    assert!(matches!(answer, Err(GrantError::PermissionEngineUnavailable { reason }) if reason == READ_FAILURE));
    for cause in [WRITE_FAILURE, READ_FAILURE] {
        assert!(events.iter().any(|fields| fields.values().any(|value| value.contains(cause))));
    }
    assert_eq!(FileLeafStore::open(&world.dir.path().join("grants"))?.extent(), 2);
    Ok(())
}

#[test]
fn mutation_acknowledgement_uses_the_real_frontier_and_preserves_the_original_receipt() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let capture = Observations::default();
    let reading = tracing::subscriber::with_default(capture.clone(), || -> Result<_, Box<dyn Error>> {
        let kinds = [RecipientKind::Person, RecipientKind::Agent];
        let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
        let request = world.request(IdentityId::Person(world.dana), root,
            IdentityId::Person(world.tom), "tern", PassOn::UseOnly, None)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.delegate(&request);
        let at_index = engine.inner.revision()?;
        let before = world.events();
        engine.faults.write_then_fail.store(true, Ordering::Relaxed);
        let answered = world.delegate(&request)?;
        let projected = engine.inner.revision()?;
        let repeated = world.delegate(&request)?;
        world.reopen(engine.clone())?;
        let reopened = world.delegate(&request)?;
        let mut changed = request.clone();
        changed.relation = lys_identity::grants::Relation::new("kite")?;
        let refused = world.delegate(&changed);
        Ok((pending, at_index, answered, projected, repeated, reopened, refused,
            before, world.events(), capture.read()?))
    });
    engine.faults.write.store(false, Ordering::Relaxed);
    engine.faults.write_then_fail.store(false, Ordering::Relaxed);
    let (pending, at_index, answered, projected, repeated, reopened, refused,
        before, after, observations) = finish_world(world, reading)?;
    assert_eq!(at_index, answered.index);
    assert!(matches!(pending, Err(GrantError::ProjectionPending { index, operation, grant })
        if index == answered.index && operation == answered.event.operation().to_string()
            && grant == answered.event.grant().to_string()));
    assert_eq!(projected, answered.index + 1);
    let marker = answered.degraded.as_deref().ok_or("acknowledgement lost its projection cause")?;
    assert_eq!(marker.revision(), projected);
    assert_eq!(marker.error(), &GrantError::PermissionEngineUnavailable {
        reason: WRITE_FAILURE.to_owned(),
    });
    assert!(observed(&observations, "project", WRITE_FAILURE, at_index));
    assert!(observed(&observations, "project", WRITE_FAILURE, projected));
    for retry in [repeated, reopened] {
        assert_eq!(retry.event, answered.event);
        assert_eq!(retry.index, answered.index);
        assert_eq!(retry.receipt, answered.receipt);
        assert!(retry.degraded.is_none());
    }
    assert!(matches!(refused, Err(GrantError::OperationReused { .. })));
    assert_eq!(before, after);
    Ok(())
}

struct ChangedReading {
    result: Result<(), GrantError>,
    projected: u64,
    actual: u64,
    before: u64,
    after: u64,
}

fn changed_reading(change_on_read: bool) -> Result<ChangedReading, Box<dyn Error>> {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let reading = (|| -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        if !matches!(pending.as_ref().err().and_then(|error| error.downcast_ref::<GrantError>()),
            Some(GrantError::ProjectionPending { .. })) {
            return Err(format!("the reading fixture did not leave projection pending: {pending:?}").into());
        }
        let projected = engine.inner.revision()?;
        let before = world.events();
        let directory = world.directory.projection()?;
        let result = if change_on_read {
            engine.faults.advance_on_read.store(true, Ordering::Relaxed);
            world.grants.frame(directory, None).map(drop)
        } else {
            let frame = world.grants.frame(directory, None)?;
            let mut writer = engine.inner.clone();
            writer.write(projected + 1, &[], &[])?;
            let request = ExerciseRequest {
                caller: IdentityId::Person(world.lee), route: Route::Api,
                resource: alpha()?, action: lys_identity::grants::Action::new("read")?,
            };
            world.grants.explain_in(&frame, &request, world.now).map(drop)
        };
        Ok(ChangedReading { result, projected, actual: engine.inner.revision()?,
            before, after: world.events() })
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    engine.faults.advance_on_read.store(false, Ordering::Relaxed);
    finish_world(world, reading)
}

#[test]
fn degraded_relationships_that_move_during_the_read_make_no_frame() -> TestResult {
    let reading = changed_reading(true)?;
    assert_eq!(reading.actual, reading.projected + 1);
    assert_eq!(reading.result, Err(GrantError::StaleDecision {
        required: reading.actual, projected: reading.projected,
    }));
    assert_eq!(reading.before, reading.after);
    Ok(())
}

#[test]
fn degraded_frame_cannot_be_explained_after_the_relationship_revision_moves() -> TestResult {
    let reading = changed_reading(false)?;
    assert_eq!(reading.actual, reading.projected + 1);
    assert_eq!(reading.result, Err(GrantError::StaleDecision {
        required: reading.actual, projected: reading.projected,
    }));
    assert_eq!(reading.before, reading.after);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct OperationCounts {
    revisions: u64,
    reads: u64,
    writes: u64,
    appends: u64,
    pins: u64,
    snapshots: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct LogCounts {
    leaves: u64,
    snapshots: u64,
    reopens: u64,
    appends: u64,
    pins: u64,
    snapshot_writes: u64,
}

impl LogCounts {
    fn since(self, before: Self) -> Result<Self, Box<dyn Error>> {
        let delta = |after: u64, before: u64| after.checked_sub(before)
            .ok_or_else(|| "fixture log counter decreased".to_owned());
        Ok(Self {
            leaves: delta(self.leaves, before.leaves)?,
            snapshots: delta(self.snapshots, before.snapshots)?,
            reopens: delta(self.reopens, before.reopens)?,
            appends: delta(self.appends, before.appends)?,
            pins: delta(self.pins, before.pins)?,
            snapshot_writes: delta(self.snapshot_writes, before.snapshot_writes)?,
        })
    }
}

impl Faults {
    fn log_counts(&self) -> LogCounts {
        LogCounts {
            leaves: self.leaves.load(Ordering::Relaxed),
            snapshots: self.snapshot_reads.load(Ordering::Relaxed),
            reopens: self.reopens.load(Ordering::Relaxed),
            appends: self.appends.load(Ordering::Relaxed),
            pins: self.pins.load(Ordering::Relaxed),
            snapshot_writes: self.snapshots.load(Ordering::Relaxed),
        }
    }

    fn counts(&self) -> OperationCounts {
        OperationCounts {
            revisions: self.revisions.load(Ordering::Relaxed),
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            appends: self.appends.load(Ordering::Relaxed),
            pins: self.pins.load(Ordering::Relaxed),
            snapshots: self.snapshots.load(Ordering::Relaxed),
        }
    }
}

impl OperationCounts {
    fn since(self, before: Self) -> Result<Self, Box<dyn Error>> {
        let delta = |after: u64, before: u64| after.checked_sub(before)
            .ok_or_else(|| "fixture operation counter decreased".to_owned());
        Ok(Self {
            revisions: delta(self.revisions, before.revisions)?,
            reads: delta(self.reads, before.reads)?,
            writes: delta(self.writes, before.writes)?,
            appends: delta(self.appends, before.appends)?,
            pins: delta(self.pins, before.pins)?,
            snapshots: delta(self.snapshots, before.snapshots)?,
        })
    }
}

#[test]
fn frame_metadata_shares_one_failure_and_keeps_healthy_and_degraded_work_bounded() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let reading = (|| -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let request = ExerciseRequest {
            caller: IdentityId::Person(world.lee), route: Route::Api,
            resource: alpha()?, action: lys_identity::grants::Action::new("read")?,
        };
        let before = engine.faults.counts();
        let healthy = world.grants.frame(world.directory.projection()?, None)?;
        let permitted = world.grants.explain_in(&healthy, &request, world.now)?;
        let omitted = healthy.degradation().is_none() && permitted.degraded.is_none();
        let healthy_counts = engine.faults.counts().since(before)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        if !matches!(pending.as_ref().err().and_then(|error| error.downcast_ref::<GrantError>()),
            Some(GrantError::ProjectionPending { .. })) {
            return Err(format!("counter fixture did not leave projection pending: {pending:?}").into());
        }
        let before = engine.faults.counts();
        let frame = world.grants.frame(world.directory.projection()?, None)?;
        let first = world.grants.explain_in(&frame, &request, world.now)?;
        let second = world.grants.explain_in(&frame, &request, world.now)?;
        let shared = frame.degradation().zip(first.degraded.as_ref()).zip(second.degraded.as_ref())
            .is_some_and(|((frame, first), second)|
                Arc::ptr_eq(frame, first) && Arc::ptr_eq(frame, second) && Arc::strong_count(frame) == 3);
        Ok((omitted, healthy_counts, shared, engine.faults.counts().since(before)?))
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    let (omitted, healthy, shared, degraded) = finish_world(world, reading)?;
    assert!(omitted);
    assert!(shared);
    assert_eq!(healthy, OperationCounts {
        revisions: 1, reads: 1, writes: 0, appends: 0, pins: 0, snapshots: 0,
    });
    assert_eq!(degraded, OperationCounts {
        revisions: 5, reads: 1, writes: 1, appends: 0, pins: 0, snapshots: 0,
    });
    Ok(())
}

#[test]
fn observation_and_log_reads_are_counted_for_allowed_stale_and_revoked_answers() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let reading = (|| -> Result<_, Box<dyn Error>> {
        let root = world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let request = ExerciseRequest {
            caller: IdentityId::Person(world.lee),
            route: Route::Api,
            resource: alpha()?,
            action: lys_identity::grants::Action::new("read")?,
        };
        let healthy = Observations::default();
        let before = engine.faults.log_counts();
        let permitted = tracing::subscriber::with_default(healthy.clone(), || {
            world.grants.explain(world.directory.projection()?, &request, world.now, None)
        })?;
        let healthy_log = engine.faults.log_counts().since(before)?;
        let healthy_cost = healthy.costs();
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        if !matches!(pending.as_ref().err().and_then(|error| error.downcast_ref::<GrantError>()),
            Some(GrantError::ProjectionPending { .. })) {
            return Err(format!("cost fixture did not leave projection pending: {pending:?}").into());
        }
        let capture = Observations::default();
        let before = engine.faults.log_counts();
        let allowed = tracing::subscriber::with_default(capture.clone(), || {
            world.grants.explain(world.directory.projection()?, &request, world.now, None)
        })?;
        let allowed_log = engine.faults.log_counts().since(before)?;
        let before = engine.faults.log_counts();
        let stale = tracing::subscriber::with_default(capture.clone(), || {
            world.grants.explain(world.directory.projection()?, &request, world.now,
                Some(world.grants.revision()))
        });
        let stale_log = engine.faults.log_counts().since(before)?;
        let revoke = world.grants.revoke(&lys_identity::grants::RevokeRequest {
            operation: lys_identity::OperationId::generate()?,
            caller: IdentityId::Person(world.admin),
            route: Route::Api,
            grant: root,
            reason: "cost fixture revocation".to_owned(),
        }, world.now);
        if !matches!(revoke, Err(GrantError::ProjectionPending { .. })) {
            return Err(format!("cost fixture revocation was not pending: {revoke:?}").into());
        }
        let before = engine.faults.log_counts();
        let revoked = tracing::subscriber::with_default(capture.clone(), || {
            world.grants.explain(world.directory.projection()?, &request, world.now, None)
        });
        let revoked_log = engine.faults.log_counts().since(before)?;
        let events = capture.read()?;
        Ok((root, permitted, allowed, stale, revoked, healthy_log, allowed_log,
            stale_log, revoked_log, healthy_cost, capture.costs(), events))
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    let (root, healthy, allowed, stale, revoked, healthy_log, allowed_log,
        stale_log, revoked_log, healthy_cost, observed_cost, events) = finish_world(world, reading)?;
    assert_eq!(healthy.grant, root);
    assert_eq!(allowed.grant, root);
    assert_eq!(healthy.path, [root]);
    assert_eq!(allowed.path, [root]);
    assert!(matches!(stale, Err(GrantError::StaleDecision { .. })));
    assert!(matches!(revoked, Err(GrantError::Revoked { grant }) if grant == root.to_string()));
    for counts in [healthy_log, allowed_log, stale_log, revoked_log] {
        assert_eq!(counts, LogCounts::default());
    }
    assert_eq!(healthy_cost, ObservationCosts::default());
    assert_eq!(observed_cost.callbacks, 3);
    assert_eq!(observed_cost.locks, observed_cost.callbacks);
    assert_eq!(events.len(), 3);
    assert!(observed_cost.fields >= 3 * 3);
    assert!(observed_cost.bytes > 0);
    assert!(events.iter().all(|fields| fields.get("step").is_some_and(|step| step == "project")));
    eprintln!("grant observation fixture costs: {observed_cost:?}; physical syncs not measured");
    Ok(())
}
