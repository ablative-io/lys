//! Draft validation precedes signing and the log's durable append.

use super::Directory;
use crate::IdentityError;
use crate::draft_event::DraftEvent;
use crate::log::Coordinate;
use crate::signer::{Entry, sign_draft_event};
use lys_log_store::LeafStore;

impl<S: LeafStore> Directory<S> {
    /// Record a creation or approval, refusing a hash mismatch before any append.
    /// A retry of the same exact payload returns its original coordinate.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::DraftChangeInvalid` if the draft payload fails validation.
    /// - `IdentityError::DraftHashMismatch` if a decision or correction names a different payload.
    /// - `IdentityError::DraftNotFound` if a decision names an absent draft.
    /// - `IdentityError::DraftNotPending` if a decision or close cannot be applied in the current state.
    /// - `IdentityError::OperationReused` if an operation is already recorded for another act.
    /// - `IdentityError::AppendRefused` if reconciliation proves the event was not committed.
    /// - `IdentityError::AppendUncertain` if a pending log append has not been resolved.
    pub fn record_draft(&mut self, event: DraftEvent) -> Result<Coordinate, IdentityError> {
        self.settle()?;
        let operation = event.operation();
        if let Some(index) = self.projection.operation(operation) {
            let (held, coordinate) =
                self.log
                    .entry(index)?
                    .ok_or_else(|| IdentityError::LogUnavailable {
                        reason: format!("operation {operation} names missing leaf {index}"),
                    })?;
            if matches!(held.entry(), Entry::Draft(draft) if draft.as_ref() == &event) {
                return Ok(coordinate);
            }
            return Err(IdentityError::OperationReused {
                operation: operation.to_string(),
            });
        }
        self.projection.check_draft(&event)?;
        let signed = sign_draft_event(event, &self.key)?;
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
        if let Some(index) = self.projection.operation(operation) {
            let (held, coordinate) =
                self.log
                    .entry(index)?
                    .ok_or_else(|| IdentityError::LogUnavailable {
                        reason: format!("operation {operation} names missing leaf {index}"),
                    })?;
            if held.bytes() == signed.bytes() {
                return Ok(coordinate);
            }
        }
        Err(IdentityError::AppendRefused {
            reason: failure.to_string(),
        })
    }
}
