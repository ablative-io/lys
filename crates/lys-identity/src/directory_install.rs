//! Recording a change to the install as a whole: the directory service
//! appends it once, under the change's own operation id, so a start that
//! records it again finds it already in the log.

use lys_log_store::LeafStore;

use super::Directory;
use crate::error::IdentityError;
use crate::install_event::InstallEvent;
use crate::signer::sign_install_event;

impl<S: LeafStore> Directory<S> {
    /// Record that the issuer moved from `from` to `to`, as the service
    /// running `build` at `recorded_at`. Answers whether this call appended
    /// it: `false` when the log already holds the same move.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::BindingMalformed` if either issuer cannot form a login binding.
    /// - `IdentityError::ChangeMismatch` if the build is invalid or the issuers are equal.
    /// - `IdentityError::OperationReused` if the move operation is already recorded when it is checked.
    /// - `IdentityError::BindingTaken` if the move would bind one login to different identities.
    /// - `IdentityError::AppendRefused` if reconciliation proves the move was not committed.
    /// - `IdentityError::AppendUncertain` if a pending log append has not been resolved.
    pub fn record_issuer_move(
        &mut self,
        from: &str,
        to: &str,
        build: &str,
        recorded_at: u64,
    ) -> Result<bool, IdentityError> {
        self.settle()?;
        let event = InstallEvent::issuer_moved(from, to, build, recorded_at)?;
        let operation = event.operation();
        if self.projection.operation(operation).is_some() {
            return Ok(false);
        }
        self.projection.check_install(&event)?;
        let signed = sign_install_event(event, &self.key)?;
        let failure = match self.log.append(&signed) {
            Ok(coordinate) => {
                self.record_committed(&signed, coordinate)?;
                self.snapshot();
                return Ok(true);
            }
            Err(failure) => failure,
        };
        if !self.log.is_uncertain() {
            return Err(failure);
        }
        self.settle()?;
        if self.projection.operation(operation).is_some() {
            Ok(true)
        } else {
            Err(IdentityError::AppendRefused {
                reason: failure.to_string(),
            })
        }
    }
}
