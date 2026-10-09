//! Product drafts as their events leave them: each draft with its creation
//! hash, its approvals in order, and at most one refusal and one close.

use super::Projection;
use crate::IdentityError;
use crate::OperationId;
use crate::encoding::payload_commitment;
use crate::grants::Mode;
use crate::product_draft_event::{
    self, Approved, Created, Executed, ProductDraftEvent, Refused, RefusedOnExecution,
};
use std::sync::Arc;

#[path = "projection_product_draft_state.rs"]
pub(super) mod state;

/// How a product draft was closed by its app's connector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closed {
    /// The product executed it.
    Executed(Arc<Executed>, u64),
    /// The product refused it when it came to execute it.
    RefusedOnExecution(Arc<RefusedOnExecution>, u64),
}

/// One product draft and every decision on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDraftRecord {
    /// The held act.
    pub created: Arc<Created>,
    /// SHA-256 of its canonical creation bytes.
    pub hash: [u8; 32],
    /// The creation's directory position.
    pub created_index: u64,
    /// Each approval and its position, in the order recorded.
    pub approvals: Vec<(Arc<Approved>, u64)>,
    /// The refusal and its position, if refused.
    pub refused: Option<(Arc<Refused>, u64)>,
    /// The close, if closed.
    pub closed: Option<Closed>,
}

impl ProductDraftRecord {
    /// How many distinct approvers its mode needs: one by draft, two by two.
    pub fn needed(&self) -> usize {
        if self.created.mode == Mode::ByTwo {
            2
        } else {
            1
        }
    }

    /// Whether it has as many distinct approvals as its mode needs.
    pub fn is_approved(&self) -> bool {
        self.refused.is_none() && self.approvals.len() >= self.needed()
    }

    /// Whether it still awaits approvals: neither approved nor refused.
    pub fn is_pending(&self) -> bool {
        self.refused.is_none() && self.approvals.len() < self.needed()
    }

    /// Whether it still waits on someone: not refused, and not closed by
    /// its product, so an approved draft left unexecuted stays open.
    pub fn is_open(&self) -> bool {
        self.closed.is_none() && self.refused.is_none()
    }

    fn events(&self) -> Vec<(ProductDraftEvent, u64)> {
        let mut events = vec![(
            ProductDraftEvent::Created(Arc::clone(&self.created)),
            self.created_index,
        )];
        for (approval, index) in &self.approvals {
            events.push((ProductDraftEvent::Approved(Arc::clone(approval)), *index));
        }
        if let Some((refused, index)) = &self.refused {
            events.push((ProductDraftEvent::Refused(Arc::clone(refused)), *index));
        }
        match &self.closed {
            Some(Closed::Executed(event, index)) => {
                events.push((ProductDraftEvent::Executed(Arc::clone(event)), *index));
            }
            Some(Closed::RefusedOnExecution(event, index)) => events.push((
                ProductDraftEvent::RefusedOnExecution(Arc::clone(event)),
                *index,
            )),
            None => {}
        }
        events
    }
}

impl Projection {
    /// The product draft by its id, without scanning leaves.
    pub fn product_draft(&self, draft: OperationId) -> Option<&ProductDraftRecord> {
        self.product_drafts.get(&draft).map(Arc::as_ref)
    }

    /// Every product draft, in draft id order.
    pub fn product_drafts(&self) -> impl Iterator<Item = &ProductDraftRecord> {
        self.product_drafts.values().map(Arc::as_ref)
    }

    /// Validate a product draft event against the drafts held, before it is
    /// signed: a decision names a held draft by its exact hash; an approval
    /// or refusal finds it pending, and an approver approves once; a close
    /// finds it approved and not yet closed.
    pub fn check_product_draft(&self, event: &ProductDraftEvent) -> Result<(), IdentityError> {
        event.validate()?;
        if self.operation(event.operation()).is_some() {
            return Err(IdentityError::OperationReused {
                operation: event.operation().to_string(),
            });
        }
        let Some((draft, hash)) = event.decision() else {
            return Ok(());
        };
        let held = self
            .product_draft(draft)
            .ok_or_else(|| IdentityError::DraftNotFound {
                draft: draft.to_string(),
            })?;
        if held.hash != hash {
            return Err(IdentityError::DraftHashMismatch);
        }
        let not_pending = || IdentityError::DraftNotPending {
            draft: draft.to_string(),
        };
        match event {
            ProductDraftEvent::Created(_) => Ok(()),
            ProductDraftEvent::Approved(approval) => {
                if !held.is_pending() {
                    return Err(not_pending());
                }
                if held
                    .approvals
                    .iter()
                    .any(|(earlier, _)| earlier.approver == approval.approver)
                {
                    return Err(IdentityError::DraftChangeInvalid {
                        reason: "two approvals of one draft are by two distinct people",
                    });
                }
                Ok(())
            }
            ProductDraftEvent::Refused(_) => {
                if held.is_pending() {
                    Ok(())
                } else {
                    Err(not_pending())
                }
            }
            ProductDraftEvent::Executed(_) | ProductDraftEvent::RefusedOnExecution(_) => {
                let app = match event {
                    ProductDraftEvent::Executed(close) => close.app.as_str(),
                    ProductDraftEvent::RefusedOnExecution(close) => close.app.as_str(),
                    _ => "",
                };
                if app != held.created.app {
                    return Err(IdentityError::DraftChangeInvalid {
                        reason: "only the draft's own app closes it",
                    });
                }
                if held.is_approved() && held.closed.is_none() {
                    Ok(())
                } else {
                    Err(not_pending())
                }
            }
        }
    }

    /// Fold a validated committed product draft entry at its directory index.
    pub fn apply_product_draft(
        &mut self,
        event: &ProductDraftEvent,
        index: u64,
    ) -> Result<(), IdentityError> {
        self.check_product_draft(event)?;
        self.fold_product_draft(event, index)?;
        Arc::make_mut(&mut self.operations).insert(event.operation(), index);
        Ok(())
    }

    /// Fold `event` into the drafts without the operation index, as a
    /// snapshot's records are read back.
    fn fold_product_draft(
        &mut self,
        event: &ProductDraftEvent,
        index: u64,
    ) -> Result<(), IdentityError> {
        let drafts = Arc::make_mut(&mut self.product_drafts);
        if let ProductDraftEvent::Created(created) = event {
            drafts.insert(
                created.operation,
                Arc::new(ProductDraftRecord {
                    created: Arc::clone(created),
                    hash: payload_commitment(&product_draft_event::encode(event)),
                    created_index: index,
                    approvals: Vec::new(),
                    refused: None,
                    closed: None,
                }),
            );
            return Ok(());
        }
        let Some((draft, _)) = event.decision() else {
            return Ok(());
        };
        let held = drafts
            .get_mut(&draft)
            .ok_or_else(|| IdentityError::DraftNotFound {
                draft: draft.to_string(),
            })?;
        let held = Arc::make_mut(held);
        match event {
            ProductDraftEvent::Created(_) => {}
            ProductDraftEvent::Approved(approval) => {
                held.approvals.push((Arc::clone(approval), index));
            }
            ProductDraftEvent::Refused(refused) => {
                held.refused = Some((Arc::clone(refused), index));
            }
            ProductDraftEvent::Executed(executed) => {
                held.closed = Some(Closed::Executed(Arc::clone(executed), index));
            }
            ProductDraftEvent::RefusedOnExecution(refused) => {
                held.closed = Some(Closed::RefusedOnExecution(Arc::clone(refused), index));
            }
        }
        Ok(())
    }
}
