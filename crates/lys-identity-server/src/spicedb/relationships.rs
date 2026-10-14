//! The service permission engine and its test fault injection.

use super::SpiceDb;
#[cfg(test)]
use super::unavailable;
use lys_identity::IdentityId;
use lys_identity::grants::{
    GrantError, MemoryRelationships, Relationship, RelationshipStore, Resource,
};
use std::collections::BTreeSet;
#[cfg(test)]
use std::sync::{Arc, Mutex};

/// The permission engine the service runs the grants on.
#[derive(Debug)]
pub enum Relationships {
    /// Relationships held in this process, gone when it stops.
    Memory(MemoryRelationships),
    /// Relationships held in `SpiceDB`.
    SpiceDb(SpiceDb),
    #[cfg(test)]
    /// An instance-private failure of the in-process permission engine.
    Fixture(FaultRelationships),
}

#[cfg(test)]
#[derive(Debug, Clone)]
/// Fault state used only by the service's grant-settlement tests.
pub struct FaultRelationships {
    held: MemoryRelationships,
    hook: Arc<Mutex<Option<Arc<ReadHook>>>>,
    unavailable: std::sync::Arc<std::sync::atomic::AtomicBool>,
    projection_only: bool,
    writes: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(test)]
impl Relationships {
    pub(crate) fn faulted(
        held: MemoryRelationships,
        unavailable: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self::Fixture(FaultRelationships {
            held,
            hook: Arc::default(),
            unavailable,
            projection_only: false,
            writes: std::sync::Arc::default(),
        })
    }

    pub(crate) fn faulted_projection(
        held: MemoryRelationships,
        unavailable: std::sync::Arc<std::sync::atomic::AtomicBool>,
        writes: std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        Self::Fixture(FaultRelationships {
            held,
            hook: Arc::default(),
            unavailable,
            projection_only: true,
            writes,
        })
    }
}

#[cfg(test)]
type ReadEffect = dyn Fn(IdentityId, &Resource) -> Result<(), GrantError> + Send + Sync;
#[cfg(test)]
pub(crate) struct ReadHook(pub(crate) Box<ReadEffect>);
#[cfg(test)]
impl std::fmt::Debug for ReadHook {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReadHook")
    }
}
#[cfg(test)]
impl Relationships {
    pub(crate) fn on_read(&self, hook: ReadHook) -> Result<(), GrantError> {
        match self {
            Self::Fixture(engine) => {
                *engine
                    .hook
                    .lock()
                    .map_err(|error| unavailable(format!("fixture hook poisoned: {error}")))? =
                    Some(Arc::new(hook));
                Ok(())
            }
            _ => Err(unavailable("the fixture has no read hook")),
        }
    }
}

#[cfg(test)]
impl FaultRelationships {
    fn readable(&self) -> Result<(), GrantError> {
        if self.projection_only {
            Ok(())
        } else {
            self.ready()
        }
    }

    fn ready(&self) -> Result<(), GrantError> {
        if self.unavailable.load(std::sync::atomic::Ordering::Acquire) {
            Err(GrantError::PermissionEngineUnavailable {
                reason: "instance-private settlement refusal".to_owned(),
            })
        } else {
            Ok(())
        }
    }
}

impl Relationships {
    pub(crate) fn read_batch(
        &self,
        revision: u64,
        covered: BTreeSet<super::context::Question>,
        questions: &BTreeSet<super::context::Permission>,
        at: u64,
    ) -> Result<super::batch::Batch, GrantError> {
        if let Self::SpiceDb(engine) = self {
            return engine.read_batch(revision, covered, questions, at);
        }
        let rows = self.read()?;
        if self.revision()? != revision {
            return Err(super::unavailable(
                "the relationship batch moved before it was read",
            ));
        }
        #[cfg(test)]
        if let Self::Fixture(engine) = self {
            let hook = engine
                .hook
                .lock()
                .map_err(|error| unavailable(format!("fixture hook poisoned: {error}")))?
                .clone();
            if let (Some(hook), Some((subject, resource))) = (hook, covered.first()) {
                (hook.0)(*subject, resource)?;
            }
        }
        Ok(super::batch::Batch {
            snapshot: super::snapshot::Snapshot::new(revision, covered, rows),
            checks: questions
                .iter()
                .cloned()
                .map(|key| (key, Ok(true)))
                .collect(),
        })
    }

    pub(crate) fn certify(&self, revision: u64) -> Result<(), GrantError> {
        match self {
            Self::SpiceDb(engine) => engine.certify(revision).map(drop),
            #[cfg(not(test))]
            Self::Memory(_) => Ok(()),
            #[cfg(test)]
            Self::Memory(_) | Self::Fixture(_) => Ok(()),
        }
    }

    pub(crate) fn reader(&self) -> Result<Self, GrantError> {
        match self {
            Self::Memory(held) => Ok(Self::Memory(held.clone())),
            Self::SpiceDb(engine) => Ok(Self::SpiceDb(engine.schema_writer()?)),
            #[cfg(test)]
            Self::Fixture(engine) => Ok(Self::Fixture(engine.clone())),
        }
    }
}

impl RelationshipStore for Relationships {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        match self {
            Self::Memory(held) => held.admit_resource(resource),
            Self::SpiceDb(engine) => engine.admit_resource(resource),
            #[cfg(test)]
            Self::Fixture(engine) => {
                engine.readable()?;
                engine.held.admit_resource(resource)
            }
        }
    }

    fn revision(&self) -> Result<u64, GrantError> {
        match self {
            Self::Memory(held) => held.revision(),
            Self::SpiceDb(engine) => engine.revision(),
            #[cfg(test)]
            Self::Fixture(engine) => {
                engine.readable()?;
                engine.held.revision()
            }
        }
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        match self {
            Self::Memory(held) => held.write(revision, touch, delete),
            Self::SpiceDb(engine) => engine.write(revision, touch, delete),
            #[cfg(test)]
            Self::Fixture(engine) => {
                engine
                    .writes
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                engine.ready()?;
                engine.held.write(revision, touch, delete)
            }
        }
    }

    fn prepared_stamp(&self) -> Option<lys_identity::grants::ReadStamp> {
        super::context::stamp()
    }

    fn read_about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        match self {
            Self::Memory(held) => held.read_about(subject, resource),
            Self::SpiceDb(engine) => engine.read_about(subject, resource),
            #[cfg(test)]
            Self::Fixture(engine) => {
                engine.readable()?;
                let hook = engine
                    .hook
                    .lock()
                    .map_err(|error| unavailable(format!("fixture hook poisoned: {error}")))?
                    .clone();
                if let Some(hook) = hook {
                    (hook.0)(subject, resource)?;
                }
                engine.held.read_about(subject, resource)
            }
        }
    }

    fn read_about_at(
        &self,
        subject: IdentityId,
        resource: &Resource,
        revision: u64,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        if let Some(held) = super::context::about(subject, resource, revision) {
            return held;
        }
        match self {
            Self::SpiceDb(engine) => engine.read_about_at(subject, resource, revision),
            #[cfg(not(test))]
            Self::Memory(_) => self.read_about(subject, resource),
            #[cfg(test)]
            Self::Memory(_) | Self::Fixture(_) => self.read_about(subject, resource),
        }
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        if let Some(held) = super::context::whole() {
            return held;
        }
        match self {
            Self::Memory(held) => held.read(),
            Self::SpiceDb(engine) => engine.read(),
            #[cfg(test)]
            Self::Fixture(engine) => {
                engine.readable()?;
                engine.held.read()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Arc, GrantError, IdentityId, MemoryRelationships, RelationshipStore, Relationships,
        Resource,
    };
    use crate::spicedb::ReadHook;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

    #[test]
    fn an_unlocked_reader_shares_the_engine_and_retains_its_refusals()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut held = MemoryRelationships::default();
        let fault = Arc::new(AtomicBool::new(false));
        let owner = Relationships::faulted(held.clone(), Arc::clone(&fault));
        let reader = owner.reader()?;
        let reads = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&reads);
        reader.on_read(ReadHook(Box::new(move |_, _| {
            counted.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })))?;
        let subject = IdentityId::Person("person-11111111111111111111111111111111".parse()?);
        let resource = Resource::new("project", "asked")?;
        assert!(reader.read_about(subject, &resource)?.is_empty());
        assert_eq!(reads.load(Ordering::Relaxed), 1);
        held.write(1, &[], &[])?;
        assert_eq!(reader.revision()?, 1);
        reader.certify(1)?;
        fault.store(true, Ordering::Release);
        assert!(matches!(
            reader.read_about(subject, &resource),
            Err(GrantError::PermissionEngineUnavailable { .. })
        ));
        let writes = Arc::new(AtomicU64::new(0));
        let mut projected = Relationships::faulted_projection(held, fault, Arc::clone(&writes));
        assert!(projected.read_about(subject, &resource)?.is_empty());
        assert!(projected.write(2, &[], &[]).is_err());
        assert_eq!(writes.load(Ordering::Relaxed), 1);
        Ok(())
    }
}
