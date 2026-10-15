//! A settled reading can cross an unlocked network call before it is judged.

use std::collections::BTreeSet;

use lys_log_store::LeafStore;

use super::frame::Settled;
use super::{Frame, GrantError, Grants, Relationship, RelationshipStore, Resource};
use crate::IdentityId;
use crate::projection::Projection;

/// The authority revision and settlement metadata before a scoped network read.
#[derive(Debug, Clone)]
pub struct ReadStamp {
    pub(super) folded: u64,
    pub(super) settled: Settled,
}

/// Relationships read for a decision at one settled revision, without a directory borrow.
#[derive(Debug, Clone)]
pub struct Reading {
    pub(super) stamp: ReadStamp,
    pub(super) held: BTreeSet<Relationship>,
}

impl Reading {
    /// The scoped relationships of this settled reading.
    #[must_use]
    pub fn relationships(&self) -> &BTreeSet<Relationship> {
        &self.held
    }
}

impl ReadStamp {
    /// The relationship revision every question in this reading requires.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.settled.projected
    }

    /// The original failure when this settlement chose a degraded revision.
    #[must_use]
    pub fn degradation(&self) -> Option<&std::sync::Arc<super::ProjectionDegraded>> {
        self.settled.degraded.as_ref()
    }

    /// Read one subject and resource at the already settled revision.
    ///
    /// # Errors
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    pub fn read_about(
        &self,
        engine: &impl RelationshipStore,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<Reading, GrantError> {
        Ok(Reading {
            stamp: self.clone(),
            held: engine.read_about_at(subject, resource, self.revision())?,
        })
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Settle once before releasing the owner lock for relationship I/O.
    ///
    /// # Errors
    /// - `GrantError::LogUnavailable` if the store or checkpoint cannot be read.
    /// - `GrantError::LeafNotAnEvent` if a leaf is missing or is not a valid event signed by this service.
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::StaleDecision` if the prepared or actual revision does not satisfy the requested revision.
    pub fn read_stamp(&mut self, at_least: Option<u64>) -> Result<ReadStamp, GrantError> {
        let settled = self.settle(at_least)?;
        Ok(ReadStamp {
            folded: self.folded,
            settled,
        })
    }

    /// Bind an unlocked reading to the current directory, refusing an authority that moved.
    ///
    /// # Errors
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::StaleDecision` if the grants moved since the reading or its degraded revision cannot be used now.
    pub fn frame_from<'d>(
        &self,
        directory: &'d Projection,
        reading: Reading,
    ) -> Result<Frame<'d>, GrantError> {
        Frame::from_reading(self, directory, reading)
    }
}

struct Update {
    revision: u64,
    touch: Vec<Relationship>,
    delete: Vec<Relationship>,
}

/// Log settlement and projection deltas prepared under the owner, applied after it releases.
pub struct ReadPlan {
    folded: u64,
    from: u64,
    at_least: Option<u64>,
    unresolved: Option<(crate::OperationId, super::GrantId)>,
    updates: Vec<Update>,
}

impl ReadPlan {
    /// Apply projection deltas outside the owner lock and select the actual readable revision.
    ///
    /// # Errors
    /// - `GrantError::PermissionEngineUnavailable` if the permission revision or relationships cannot be read.
    /// - A custom `RelationshipStore` may return another `GrantError` from its read or revision methods; that exact variant is propagated.
    /// - `GrantError::StaleDecision` if the actual readable revision is below the requested minimum; write failures with a readable revision are retained as degradation.
    pub fn execute(self, engine: &mut impl RelationshipStore) -> Result<ReadStamp, GrantError> {
        let mut projected = self.from;
        let mut failed = None;
        for update in self.updates {
            if let Err(error) = engine.write(update.revision, &update.touch, &update.delete) {
                failed = Some(error);
                break;
            }
            let moved = engine.revision()?;
            if moved <= projected {
                failed = Some(GrantError::PermissionEngineUnavailable {
                    reason: format!(
                        "the permission engine took the write for revision {} and did not move from revision {projected}",
                        update.revision
                    ),
                });
                break;
            }
            projected = moved;
        }
        let degraded = if let Some(error) = failed {
            projected = engine.revision()?;
            tracing::warn!(step = "project", error = %error, revision = projected, "grant projection degraded");
            Some(std::sync::Arc::new(super::ProjectionDegraded::at(
                error, projected,
            )))
        } else {
            None
        };
        if let Some(required) = self.at_least
            && projected < required
        {
            return Err(GrantError::StaleDecision {
                required,
                projected,
            });
        }
        Ok(ReadStamp {
            folded: self.folded,
            settled: Settled {
                projected,
                unresolved: self.unresolved,
                degraded,
            },
        })
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Reconcile the log and prepare every missing projection delta, without engine I/O.
    ///
    /// # Errors
    /// - `GrantError::LogUnavailable` if the store or checkpoint cannot be read.
    /// - `GrantError::LeafNotAnEvent` if a leaf is missing or is not a valid event signed by this service.
    /// - `GrantError::GrantUnknown` if an issuance delta names no recorded grant.
    pub fn read_plan(&mut self, from: u64, at_least: Option<u64>) -> Result<ReadPlan, GrantError> {
        self.settle_log()?;
        let changes = self.book.changes_from(from);
        let mut updates = Vec::new();
        for index in from..self.folded {
            let (touch, delete) = self.delta(index, &changes)?;
            updates.push(Update {
                revision: index + 1,
                touch,
                delete,
            });
        }
        Ok(ReadPlan {
            folded: self.folded,
            from,
            at_least,
            unresolved: self.unresolved_revocation(),
            updates,
        })
    }
}
