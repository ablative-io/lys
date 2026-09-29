//! The native reader and explicit reconciliation reuse the session table's
//! operation owner. Neither path can write a pipe, start a turn or mint an id.
use super::{Sessions, Table, unknown};
use crate::error::RunnerError;
use crate::harness_control::events::Event;
use crate::operations::OperationOutcome;

impl Sessions {
    /// Keep a validated observation from the sole managed dispatcher. The
    /// process generation must still be owned by this session; the operation's
    /// journal independently checks its full source and native identities.
    /// This method confers no authority to launch or dispatch an operation.
    pub fn control_observed(
        &self,
        operation: &str,
        event: &Event,
    ) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock();
        let session = table
            .sessions
            .get(&event.source.binding.session)
            .ok_or_else(|| unknown(&event.source.binding.session))?;
        if session.generation != event.source.generation
            || session.guard.leader.as_ref() != Some(&event.source.leader)
            || session.ended.is_some()
        {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "native reader does not own this session's live process generation",
            ));
        }
        let Table {
            operations, feed, ..
        } = &mut *table;
        let result = operations.keep_control(feed, operation, event);
        drop(table);
        self.wake();
        result
    }

    /// Reconcile an original operation from one already committed native fact.
    /// The source comes from its journal, not from the caller. This works after
    /// restart without claiming that the prior process or pipe is still alive.
    pub fn reconcile_control(
        &self,
        operation: &str,
        source_id: &str,
    ) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock();
        let held = table.operations.get(operation).ok_or_else(|| {
            RunnerError::refused("operation_unknown", "no operation has this identity")
        })?;
        let delivery = held.control.as_ref().ok_or_else(|| {
            RunnerError::refused(
                "control_delivery_unprepared",
                "no managed write was journalled",
            )
        })?;
        let event = table.feed.control_event(&delivery.source, source_id)?;
        let Table {
            operations, feed, ..
        } = &mut *table;
        let result = operations.keep_control(feed, operation, &event);
        drop(table);
        self.wake();
        result
    }
}
