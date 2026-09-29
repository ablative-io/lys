//! One hold of the grants for many questions.
//!
//! A [`Frame`] settles the log, projects the relationships and reads them
//! once, and every question put through [`Grants::explain_in`] is decided
//! against that one reading. Asking who holds what across many resources is
//! then one read of the permission engine rather than one read per holder,
//! action and resource. Each answer is the one [`Grants::explain`] gives.

use std::collections::BTreeSet;

use lys_log_store::LeafStore;

use super::admission::effective;
use super::authority::{ExerciseRequest, Grants, Permit};
use super::error::GrantError;
use super::events::GrantChange;
use super::permission::{Relationship, RelationshipStore, confirm};
use super::types::GrantId;
use crate::operation::OperationId;
use crate::projection::Projection;

/// The relationships at one revision, read once, and what the log held
/// unresolved when they were read.
#[derive(Debug)]
pub struct Frame {
    projected: u64,
    held: Result<BTreeSet<Relationship>, GrantError>,
    unresolved: Option<(OperationId, GrantId)>,
}

impl Frame {
    /// The revision every decision in this frame is made at.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.projected
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Settle the log, project the relationships and read them, once, for
    /// every decision [`Grants::explain_in`] makes in the frame. `at_least`
    /// is a revision every decision must reflect.
    pub fn frame(&mut self, at_least: Option<u64>) -> Result<Frame, GrantError> {
        self.settle_log().ok();
        let projected = match self.project() {
            Ok(projected) => projected,
            Err(_) => self.relationships.revision()?,
        };
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
        Ok(Frame {
            projected,
            held: self.relationships.read(),
            unresolved,
        })
    }

    /// The decision [`Grants::explain`] makes, against `frame`'s reading.
    pub fn explain_in(
        &self,
        frame: &Frame,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
    ) -> Result<Permit, GrantError> {
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
        let held = frame.held.as_ref().map_err(Clone::clone)?;
        let mut refusal = None;
        let candidates = self.book.on_resource(&request.resource).filter(|record| {
            record.grant().holder() == request.caller
                && record.grant().actions().contains(&request.action)
        });
        for record in candidates {
            let grant = record.grant();
            let decided = effective(&self.book, directory, grant.id(), at).and_then(|lineage| {
                if let Some((operation, revoked)) = frame.unresolved
                    && lineage.path.contains(&revoked)
                {
                    return Err(GrantError::OperationUnresolved {
                        operation: operation.to_string(),
                        grant: revoked.to_string(),
                    });
                }
                self.fresh(&lineage.path, frame.projected)?;
                confirm(held, &lineage.path, at)?;
                Ok(lineage)
            });
            match decided {
                Ok(lineage) => {
                    return Ok(Permit {
                        grant: grant.id(),
                        path: lineage.path,
                        root_person: lineage.root_person,
                        actions: grant.actions().clone(),
                        model_version: grant.parts().model_version,
                        revision: frame.projected,
                        use_event: None,
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
