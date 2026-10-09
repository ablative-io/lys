//! A product draft event is checked against the drafts held, signed and
//! appended; the same act asked again is answered with its first position.

use super::Directory;
use crate::IdentityError;
use crate::log::Coordinate;
use crate::product_draft_event::ProductDraftEvent;
use crate::signer::{Entry, sign_product_draft_event};
use lys_log_store::LeafStore;

impl<S: LeafStore> Directory<S> {
    /// Record a product draft's creation, a decision on it, or its close.
    /// The same act under the same operation, asked again at another time,
    /// returns its original coordinate; another act under that operation is
    /// refused `OperationReused`, and nothing is appended.
    pub fn record_product_draft(
        &mut self,
        event: ProductDraftEvent,
    ) -> Result<Coordinate, IdentityError> {
        self.settle()?;
        if let Some(coordinate) = self.product_draft_retry(&event)? {
            return Ok(coordinate);
        }
        self.projection.check_product_draft(&event)?;
        let signed = sign_product_draft_event(event, &self.key)?;
        let failure = match self.log.append(&signed) {
            Ok(coordinate) => {
                self.record_committed(&signed, coordinate)?;
                self.snapshot();
                return Ok(coordinate);
            }
            Err(failure) => failure,
        };
        if !self.log.is_uncertain() {
            return Err(failure);
        }
        self.settle()?;
        let Entry::ProductDraft(event) = signed.entry() else {
            return Err(IdentityError::AppendRefused {
                reason: failure.to_string(),
            });
        };
        if let Some(coordinate) = self.product_draft_retry(event)? {
            return Ok(coordinate);
        }
        Err(IdentityError::AppendRefused {
            reason: failure.to_string(),
        })
    }

    /// The coordinate of the act `event` repeats, or none when its operation
    /// is unrecorded; refused `OperationReused` when the operation recorded
    /// another act.
    fn product_draft_retry(
        &self,
        event: &ProductDraftEvent,
    ) -> Result<Option<Coordinate>, IdentityError> {
        let operation = event.operation();
        let Some(index) = self.projection.operation(operation) else {
            return Ok(None);
        };
        let (held, coordinate) =
            self.log
                .entry(index)?
                .ok_or_else(|| IdentityError::LogUnavailable {
                    reason: format!("operation {operation} names missing leaf {index}"),
                })?;
        if matches!(held.entry(), Entry::ProductDraft(draft) if draft.same_act(event)) {
            return Ok(Some(coordinate));
        }
        Err(IdentityError::OperationReused {
            operation: operation.to_string(),
        })
    }
}
