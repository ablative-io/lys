//! Draft records retain immutable creation hashes and hash-bound decisions.

use super::Projection;
use crate::draft_event::{Approved, Correction, Created, DraftEvent, Refused};
use crate::encoding::payload_commitment;
use crate::{IdentityError, OperationId};
use std::sync::Arc;

#[path = "projection_draft_state.rs"]
pub(super) mod state;

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
    /// A refusal and its directory position.
    pub refused: Option<(Arc<Refused>, u64)>,
    /// The correction that refuses this original and links its replacement.
    pub correction: Option<(Arc<Correction>, u64)>,
    /// The original when this record is the corrector's own change.
    pub replacement_of: Option<OperationId>,
}

impl DraftRecord {
    /// Whether this original still awaits a decision.
    pub fn is_pending(&self) -> bool {
        self.approved.is_none()
            && self.refused.is_none()
            && self.correction.is_none()
            && self.replacement_of.is_none()
    }

    /// Whether the original was refused, directly or by correction.
    pub fn is_refused(&self) -> bool {
        self.refused.is_some() || self.correction.is_some()
    }

    pub(super) fn decision(&self) -> Option<(DraftEvent, u64)> {
        if let Some((event, index)) = &self.approved {
            return Some((DraftEvent::Approved(Arc::clone(event)), *index));
        }
        if let Some((event, index)) = &self.refused {
            return Some((DraftEvent::Refused(Arc::clone(event)), *index));
        }
        self.correction
            .as_ref()
            .map(|(event, index)| (DraftEvent::Correction(Arc::clone(event)), *index))
    }
}

impl Projection {
    /// The draft by its creation operation, without scanning leaves.
    pub fn draft(&self, operation: OperationId) -> Option<&DraftRecord> {
        self.drafts.get(&operation).map(Arc::as_ref)
    }

    /// Every draft held, originals and corrections' own changes alike, in
    /// operation id order (the order of their text form), without reading leaves.
    pub fn drafts(&self) -> impl Iterator<Item = &DraftRecord> {
        self.drafts.values().map(Arc::as_ref)
    }

    /// Validate a draft act before signing or appending it.
    pub fn check_draft(&self, event: &DraftEvent) -> Result<(), IdentityError> {
        event.validate()?;
        if let Some((draft, hash)) = event.decision() {
            let held = self
                .draft(draft)
                .ok_or_else(|| IdentityError::DraftNotFound {
                    draft: draft.to_string(),
                })?;
            if held.hash != hash {
                return Err(IdentityError::DraftHashMismatch);
            }
            if !held.is_pending() {
                return Err(IdentityError::DraftNotPending {
                    draft: draft.to_string(),
                });
            }
        }
        if let DraftEvent::Approved(approval) = event {
            if self.operation(approval.application).is_some() {
                return Err(IdentityError::OperationReused {
                    operation: approval.application.to_string(),
                });
            }
        }
        if let DraftEvent::Correction(correction) = event {
            if self.operation(correction.corrected.operation).is_some() {
                return Err(IdentityError::OperationReused {
                    operation: correction.corrected.operation.to_string(),
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
                    refused: None,
                    correction: None,
                    replacement_of: None,
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
            DraftEvent::Refused(refused) => {
                let held = Arc::make_mut(&mut self.drafts)
                    .get_mut(&refused.draft)
                    .ok_or_else(|| IdentityError::DraftNotFound {
                        draft: refused.draft.to_string(),
                    })?;
                Arc::make_mut(held).refused = Some((Arc::clone(refused), index));
            }
            DraftEvent::Correction(correction) => {
                let held = Arc::make_mut(&mut self.drafts)
                    .get_mut(&correction.draft)
                    .ok_or_else(|| IdentityError::DraftNotFound {
                        draft: correction.draft.to_string(),
                    })?;
                Arc::make_mut(held).correction = Some((Arc::clone(correction), index));
                Arc::make_mut(&mut self.drafts).insert(
                    correction.corrected.operation,
                    Arc::new(DraftRecord {
                        created: Arc::clone(&correction.corrected),
                        hash: correction.corrected_hash,
                        created_index: index,
                        approved: None,
                        refused: None,
                        correction: None,
                        replacement_of: Some(correction.draft),
                    }),
                );
                Arc::make_mut(&mut self.operations).insert(correction.corrected.operation, index);
            }
        }
        Arc::make_mut(&mut self.operations).insert(event.operation(), index);
        Ok(())
    }
}
