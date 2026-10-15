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

fn witnessed_world(engine: &Engine) -> Result<World<Store, Engine>, Box<dyn Error>> {
    let mut world = world(engine)?;
    let opened = (|| -> Result<_, Box<dyn Error>> {
        let path = world.dir.path().join("grants");
        let provider = Arc::new(lys_log_store::witness::FileTailProvider::new(
            FileLeafStore::open(&path)?,
        ));
        let faults = Arc::clone(&engine.faults);
        let reopen = Box::new(move || {
            faults.reopens.fetch_add(1, Ordering::Relaxed);
            if faults.log_unreadable.load(Ordering::Relaxed) {
                return Err(log_failure());
            }
            Ok(Store {
                inner: FileLeafStore::open(&path)?,
                faults: Arc::clone(&faults),
            })
        });
        Ok(lys_identity::grants::Grants::open_with_tail_provider(
            reopen,
            lys_core::Ed25519Identity::load(&world.dir.path().join("service.key"))?,
            engine.clone(),
            world.grants.model().clone(),
            world.admin,
            lys_identity::restart::SNAPSHOT_EVERY,
            Some(provider),
        )?)
    })();
    match opened {
        Ok(grants) => {
            world.grants = grants;
            Ok(world)
        }
        Err(error) => finish_world(world, Err(error)),
    }
}

fn finish_world<S: LeafStore, R: RelationshipStore, T>(
    world: World<S, R>,
    result: Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    let dir = Arc::clone(&world.dir);
    drop(world);
    let cleanup = Arc::try_unwrap(dir)
        .map_err(|shared| {
            format!(
                "fixture directory still has {} owners",
                Arc::strong_count(&shared)
            )
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
        self.faults.reads.fetch_add(1, Ordering::Relaxed);
        let held = self.inner.read()?;
        if self.faults.advance_on_read.swap(false, Ordering::Relaxed) {
            let mut writer = self.inner.clone();
            let revision = writer.revision()?.checked_add(1).ok_or_else(|| {
                GrantError::PermissionEngineUnavailable {
                    reason: "fixture relationship revision overflowed".to_owned(),
                }
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

include!("grant_settlement/observations.rs");
include!("grant_settlement/tail.rs");
include!("grant_settlement/failures.rs");
include!("grant_settlement/revision.rs");
include!("grant_settlement/costs.rs");
