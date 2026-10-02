//! An accepted operation the server re-judged before its boundary: withdrawn
//! while it still waits, so it is never typed; one already being typed, or
//! typed, is refused by name and stands as it was.

use crate::error::RunnerError;
use crate::session::Sessions;

use super::{OperationOutcome, OperationState};

impl Sessions {
    /// Withdraw `operation` before its boundary, saying `why`.
    ///
    /// # Errors
    /// Returns an unknown operation, one already typed, or a recording
    /// failure by name.
    pub fn withdraw(&self, operation: &str, why: &str) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock()?;
        let held = table.operations.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused(
                "operation_unknown",
                format!("no operation {operation} is held"),
            )
        })?;
        if held.state != OperationState::Accepted {
            return Err(RunnerError::refused(
                "operation_past_its_boundary",
                format!(
                    "operation {operation} is no longer waiting: it stands {:?}",
                    held.state
                ),
            ));
        }
        if let Some(waiting) = table.operations.accepted.get_mut(&held.session) {
            waiting.retain(|queued| queued != operation);
        }
        table.operations.texts.remove(operation);
        let outcome = table.operations.set(
            operation,
            OperationState::Refused,
            format!("withdrawn before its boundary: {why}"),
        )?;
        super::delivery::feed(&mut table, &outcome);
        drop(table);
        self.writer.barrier()?;
        Ok(outcome)
    }
}
