//! Draft records retain immutable creation hashes and hash-bound decisions.

use super::Projection;
use crate::draft_event::{Approved, Created, DraftEvent};
use crate::encoding::payload_commitment;
use crate::{IdentityError, OperationId};
use std::sync::Arc;

/// The immutable draft and its current decision, indexed independently of history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRecord {
    /// The original creation payload.
    pub created: Arc<Created>,
    /// SHA-256 of its exact canonical bytes.
    pub hash: [u8; 32],
    /// The creation's directory position.
    pub created_index: u64,
    /// The recorded approval and its position, if any.
    pub approved: Option<(Arc<Approved>, u64)>,
}

impl Projection {
    /// The draft by its creation operation, without scanning leaves.
    pub fn draft(&self, operation: OperationId) -> Option<&DraftRecord> {
        self.drafts.get(&operation).map(Arc::as_ref)
    }

    /// Validate a draft act before signing or appending it.
    pub fn check_draft(&self, event: &DraftEvent) -> Result<(), IdentityError> {
        event.validate()?;
        if let DraftEvent::Approved(approval) = event {
            let held = self
                .draft(approval.draft)
                .ok_or_else(|| IdentityError::DraftNotFound {
                    draft: approval.draft.to_string(),
                })?;
            if held.hash != approval.draft_hash {
                return Err(IdentityError::DraftHashMismatch);
            }
            if held.approved.is_some() {
                return Err(IdentityError::DraftNotPending {
                    draft: approval.draft.to_string(),
                });
            }
            if self.operation(approval.application).is_some() {
                return Err(IdentityError::OperationReused {
                    operation: approval.application.to_string(),
                });
            }
        }
        if self.operation(event.operation()).is_some() {
            return Err(IdentityError::OperationReused {
                operation: event.operation().to_string(),
            });
        }
        Ok(())
    }

    /// Fold a validated committed draft entry at its directory index.
    pub fn apply_draft(&mut self, event: &DraftEvent, index: u64) -> Result<(), IdentityError> {
        self.check_draft(event)?;
        match event {
            DraftEvent::Created(created) => {
                let record = DraftRecord {
                    created: Arc::clone(created),
                    hash: payload_commitment(&crate::draft_event::encode(event)),
                    created_index: index,
                    approved: None,
                };
                Arc::make_mut(&mut self.drafts).insert(created.operation, Arc::new(record));
            }
            DraftEvent::Approved(approved) => {
                let held = Arc::make_mut(&mut self.drafts)
                    .get_mut(&approved.draft)
                    .ok_or_else(|| IdentityError::DraftNotFound {
                        draft: approved.draft.to_string(),
                    })?;
                Arc::make_mut(held).approved = Some((Arc::clone(approved), index));
            }
        }
        Arc::make_mut(&mut self.operations).insert(event.operation(), index);
        Ok(())
    }
}
