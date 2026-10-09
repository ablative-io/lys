//! One hold of the grants for many questions.
//!
//! A [`Frame`] settles the log, projects the relationships and reads them
//! once, and every question put through [`Grants::explain_in`] is decided
//! against that one reading and the directory the frame was taken with.
//! Asking who holds what across many resources is then one read of the
//! permission engine rather than one read per holder, action and resource.
//! Each answer is the one [`Grants::explain`] gives. A frame whose read
//! failed is not made: the failure is the answer. A frame asked after the
//! grants have moved on is refused `StaleDecision`, never mixed with them.

use std::collections::BTreeSet;
use std::sync::Arc;

use lys_log_store::LeafStore;

use super::admission::effective;
use super::authority::{ExerciseRequest, Grants, Permit};
use super::error::GrantError;
use super::events::GrantChange;
use super::permission::{Relationship, RelationshipStore, confirm};
use super::settlement::ProjectionDegraded;
use super::types::GrantId;
use crate::operation::OperationId;
use crate::projection::Projection;

/// The relationships at one revision, read once, and what the log held
/// unresolved when they were read.
#[derive(Debug)]
pub struct Frame<'d> {
    directory: &'d Projection,
    folded: u64,
    projected: u64,
    held: BTreeSet<Relationship>,
    unresolved: Option<(OperationId, GrantId)>,
    degraded: Option<Arc<ProjectionDegraded>>,
}

/// A frame's log settled and relationships projected, before they are read.
#[derive(Debug, Clone)]
pub(super) struct Settled {
    pub(super) projected: u64,
    pub(super) unresolved: Option<(OperationId, GrantId)>,
    pub(super) degraded: Option<Arc<ProjectionDegraded>>,
}

impl<'d> Frame<'d> {
    /// Read `grants`' relationships once, after `settled`, with `directory`.
    pub(super) fn read<S: LeafStore, R: RelationshipStore>(
        grants: &Grants<S, R>,
        directory: &'d Projection,
        settled: Settled,
    ) -> Result<Self, GrantError> {
        let frame = Frame {
            directory,
            folded: grants.folded,
            projected: settled.projected,
            held: grants.relationships.read()?,
            unresolved: settled.unresolved,
            degraded: settled.degraded,
        };
        frame.current_degraded_revision(grants)?;
        Ok(frame)
    }

    /// The revision every decision in this frame is made at.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.projected
    }

    /// The original projection failure shared by this reading, when degraded.
    #[must_use]
    pub fn degradation(&self) -> Option<&Arc<ProjectionDegraded>> {
        self.degraded.as_ref()
    }

    fn current_degraded_revision<S: LeafStore, R: RelationshipStore>(
        &self,
        grants: &Grants<S, R>,
    ) -> Result<(), GrantError> {
        if self.degraded.is_none() {
            return Ok(());
        }
        let actual = grants.relationships.revision().inspect_err(|error| {
            tracing::warn!(step = "revision", error = %error, "degraded reading could not be certified");
        })?;
        if actual > self.projected {
            return Err(GrantError::StaleDecision {
                required: actual,
                projected: self.projected,
            });
        }
        if actual < self.projected {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: format!(
                    "the relationship revision decreased from {} to {actual}",
                    self.projected
                ),
            });
        }
        Ok(())
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Settle the log, project the relationships and read them, once, for
    /// every decision [`Grants::explain_in`] makes in the frame. `at_least`
    /// is a revision every decision must reflect.
    pub fn frame<'d>(
        &mut self,
        directory: &'d Projection,
        at_least: Option<u64>,
    ) -> Result<Frame<'d>, GrantError> {
        let settled = self.settle(at_least)?;
        Frame::read(self, directory, settled)
    }

    /// Settle the log and project the relationships, refusing a projection
    /// older than `at_least`, and name the revocation held unresolved.
    pub(super) fn settle(&mut self, at_least: Option<u64>) -> Result<Settled, GrantError> {
        self.settle_log().inspect_err(|error| {
            tracing::warn!(step = "reconcile", error = %error, "grant log settlement refused");
        })?;
        let reading = self.project_reading()?;
        let projected = reading.revision;
        if let Some(required) = at_least
            && projected < required
        {
            return Err(GrantError::StaleDecision {
                required,
                projected,
            });
        }
        let unresolved = self
            .ledger
            .uncertain()
            .and_then(|held| match held.event.change() {
                GrantChange::Revoke { grant, .. } => Some((held.operation, *grant)),
                GrantChange::Issue(_) | GrantChange::Use { .. } => None,
            });
        Ok(Settled {
            projected,
            unresolved,
            degraded: reading.degraded,
        })
    }

    /// The decision [`Grants::explain`] makes, against `frame`'s reading,
    /// refused `StaleDecision` when a grant event was recorded after it.
    pub fn explain_in(
        &self,
        frame: &Frame<'_>,
        request: &ExerciseRequest,
        at: u64,
    ) -> Result<Permit, GrantError> {
        self.explain_only(frame, request, None, at)
    }

    /// Explain from one reading while restricting authority to the named grant.
    ///
    /// # Errors
    /// Returns the original refusal when the reading or selected lineage is invalid.
    pub fn explain_in_by(
        &self,
        frame: &Frame<'_>,
        request: &ExerciseRequest,
        only: Option<GrantId>,
        at: u64,
    ) -> Result<Permit, GrantError> {
        self.explain_only(frame, request, only, at)
    }

    /// [`Grants::explain_in`] resting only on `only` when it names a grant.
    pub(super) fn explain_only(
        &self,
        frame: &Frame<'_>,
        request: &ExerciseRequest,
        only: Option<GrantId>,
        at: u64,
    ) -> Result<Permit, GrantError> {
        if frame.folded != self.folded {
            return Err(GrantError::StaleDecision {
                required: self.folded,
                projected: frame.projected,
            });
        }
        frame.current_degraded_revision(self)?;
        self.unresolved_issue(request)?;
        self.decide_in(frame, request, only, at)
    }

    /// Refuse by the held operation's name while the issue of the grant the
    /// question rests on is unresolved.
    pub(super) fn unresolved_issue(&self, request: &ExerciseRequest) -> Result<(), GrantError> {
        if let Some(held) = self.ledger.uncertain()
            && let GrantChange::Issue(grant) = held.event.change()
            && grant.holder() == request.caller
            && grant.resource() == &request.resource
        {
            return Err(GrantError::OperationUnresolved {
                operation: held.operation.to_string(),
                grant: held.grant.to_string(),
            });
        }
        Ok(())
    }

    fn decide_in(
        &self,
        frame: &Frame<'_>,
        request: &ExerciseRequest,
        only: Option<GrantId>,
        at: u64,
    ) -> Result<Permit, GrantError> {
        let mut refusal = None;
        let candidates = self.book.on_resource(&request.resource).filter(|record| {
            record.grant().holder() == request.caller
                && self
                    .model
                    .actions_of(record.grant())
                    .contains(&request.action)
                && only.is_none_or(|grant| record.grant().id() == grant)
        });
        for record in candidates {
            let grant = record.grant();
            let decided =
                effective(&self.book, frame.directory, grant.id(), at).and_then(|lineage| {
                    if let Some((operation, revoked)) = frame.unresolved
                        && lineage.path.contains(&revoked)
                    {
                        return Err(GrantError::OperationUnresolved {
                            operation: operation.to_string(),
                            grant: revoked.to_string(),
                        });
                    }
                    self.fresh(&lineage.path, frame.projected)?;
                    confirm(&frame.held, &lineage.path, at)?;
                    Ok(lineage)
                });
            match decided {
                Ok(lineage) => {
                    return Ok(Permit {
                        grant: grant.id(),
                        path: lineage.path,
                        root_person: lineage.root_person,
                        actions: self.model.actions_of(grant).clone(),
                        // A role is judged under the version current now.
                        model_version: if grant.names_role() {
                            self.model.version_on(grant.resource().kind())
                        } else {
                            grant.parts().model_version
                        },
                        revision: frame.projected,
                        use_event: None,
                        degraded: frame.degraded.as_ref().map(Arc::clone),
                    });
                }
                Err(error) => {
                    refusal.get_or_insert(error);
                }
            }
        }
        Err(refusal.unwrap_or_else(|| GrantError::NotHeld {
            identity: request.caller.to_string(),
            resource: request.resource.to_string(),
            action: request.action.to_string(),
        }))
    }
}
