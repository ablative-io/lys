#![cfg(test)]

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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

use support::{World, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;
const WRITE_FAILURE: &str = "settlement-fixture-write-refused";
const READ_FAILURE: &str = "settlement-fixture-revision-unavailable";
const LOG_FAILURE: &str = "settlement-fixture-reopen-unavailable";

#[derive(Default)]
struct Faults {
    writes: AtomicU64,
    write: AtomicBool,
    read_after_write: AtomicBool,
    revision: AtomicBool,
    lost_acknowledgement: AtomicBool,
    log_unreadable: AtomicBool,
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
        self.inner.read()
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
        self.inner.leaf(index)
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
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
        self.inner.pin(pin)
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
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
        let mut fields = Fields(BTreeMap::new());
        event.record(&mut fields);
        match self.events.lock() {
            Ok(mut events) => events.push(fields.0),
            Err(error) => panic!("observation_fixture_lock_poisoned: {error}"),
        }
    }

    fn enter(&self, span: &Id) {
        assert_ne!(span.into_u64(), 0);
    }

    fn exit(&self, span: &Id) {
        assert_ne!(span.into_u64(), 0);
    }
}

impl Observations {
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
    let events = capture.read()?;
    opened?;
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
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
