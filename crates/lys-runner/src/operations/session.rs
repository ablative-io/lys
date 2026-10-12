//! Session entry points for durable operations.
use super::*;
use crate::session::Sessions;
use std::sync::atomic::AtomicBool;

impl Sessions {
    /// Accept `operation` under its stable id, answering how it stands.
    ///
    /// # Errors
    /// Refuses an unknown session, reused identity or missing control authority,
    /// and propagates an unavailable operation record or durable writer.
    pub fn operate(&self, operation: Operation) -> Result<OperationOutcome, RunnerError> {
        if matches!(operation.request, OperationRequest::BoundaryReply { .. }) {
            return Err(RunnerError::refused(
                "control_boundary_service_required",
                "only a verified service request may decide a boundary",
            ));
        }
        if matches!(
            operation.request,
            OperationRequest::Compact { .. } | OperationRequest::ContextCompact { .. }
        ) && self.lock()?.responsible.contains_key(&operation.session)
        {
            return Err(RunnerError::refused(
                "SessionInputContextMissing",
                "owned compaction requires a verified caller and live grant judge",
            ));
        }
        self.operate_admitted(operation)
    }

    pub(crate) fn operate_admitted(
        &self,
        operation: Operation,
    ) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock()?;
        let mut outcome = accept(&mut table, operation)?;
        drop(table);
        self.writer.barrier()?;
        if outcome.request == "stop" && outcome.state == OperationState::Accepted {
            let mut table = self.lock()?;
            outcome = stop(&mut table, &outcome.session, &outcome.operation)?;
            drop(table);
            self.writer.barrier()?;
        }
        self.wake();
        if outcome.state != OperationState::Delivering {
            return Ok(outcome);
        }
        let outcome = self.until_any(&AtomicBool::new(false), |table| {
            table
                .operations
                .get(&outcome.operation)
                .filter(|held| held.state != OperationState::Delivering)
                .cloned()
        })?;
        self.writer.barrier()?;
        Ok(outcome)
    }

    /// How operation `operation` stands, as the record keeps it.
    ///
    /// # Errors
    /// Refuses an unknown operation and propagates an unavailable record or writer.
    pub fn outcome(&self, operation: &str) -> Result<OperationOutcome, RunnerError> {
        let outcome = self
            .lock()?
            .operations
            .get(operation)
            .cloned()
            .ok_or_else(|| {
                RunnerError::refused(
                    "operation_unknown",
                    format!("no operation {operation} is held"),
                )
            })?;
        self.writer.barrier()?;
        Ok(outcome)
    }
}

impl Sessions {
    /// Confirm a delivered Codex compaction when its caller observed a turn end.
    ///
    /// # Errors
    /// Returns an unavailable operation record or durable writer.
    pub fn confirm_on_turn_end(&self, id: &str) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        super::confirm_on_turn_end(&mut table, id);
        drop(table);
        self.writer.barrier()?;
        self.wake();
        Ok(())
    }
}
