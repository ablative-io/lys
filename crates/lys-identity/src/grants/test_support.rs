//! A permission engine whose outage a test switches on and off.
//!
//! Built only for tests, and for other crates' tests through the
//! `test-support` feature. While it is down, every read and write is refused
//! `PermissionEngineUnavailable`, and nothing is written.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::error::GrantError;
use super::permission::{MemoryRelationships, Relationship, RelationshipStore};
use super::types::Resource;

/// An in-process permission engine that can be taken down. Clones share one
/// engine and one switch.
#[derive(Debug, Clone, Default)]
pub struct FailingRelationships {
    inner: MemoryRelationships,
    down: Arc<AtomicBool>,
}

impl FailingRelationships {
    /// Take the engine down, or bring it back.
    pub fn set_down(&self, down: bool) {
        self.down.store(down, Ordering::SeqCst);
    }

    fn up(&self) -> Result<(), GrantError> {
        if self.down.load(Ordering::SeqCst) {
            Err(GrantError::PermissionEngineUnavailable {
                reason: "the permission engine is down".to_owned(),
            })
        } else {
            Ok(())
        }
    }
}

impl RelationshipStore for FailingRelationships {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        self.inner.admit_resource(resource)
    }

    fn revision(&self) -> Result<u64, GrantError> {
        self.up()?;
        self.inner.revision()
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        self.up()?;
        self.inner.write(revision, touch, delete)
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        self.up()?;
        self.inner.read()
    }
}
