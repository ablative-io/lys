//! Harness admission proves delivery even before the pipe completion callback.
use super::{Sessions, runtime};
use crate::error::RunnerError;
use crate::operations::OperationState;
use crate::session::Table;

pub(super) fn admitted(table: &mut Table, operation: &str) -> Result<(), RunnerError> {
    if table
        .operations
        .get(operation)
        .is_some_and(|held| held.request == "notice" && held.state == OperationState::Delivering)
    {
        crate::operations::managed_state(
            table,
            operation,
            OperationState::Delivered,
            "managed user turn admitted by harness".to_owned(),
        )?;
    }
    Ok(())
}

impl Sessions {
    pub(super) fn managed_written(
        &self,
        id: &str,
        generation: u64,
        operation: &str,
        result: Result<(), RunnerError>,
    ) -> Result<(), RunnerError> {
        if let Err(error) = result {
            return self.managed_lost(id, generation, &error.to_string());
        }
        let mut table = self.lock()?;
        runtime(&mut table, id, generation)?;
        if table
            .operations
            .get(operation)
            .is_some_and(|held| held.state == OperationState::Delivering)
        {
            let notice = table
                .operations
                .get(operation)
                .is_some_and(|held| held.request == "notice");
            crate::operations::managed_state(
                &mut table,
                operation,
                if notice {
                    OperationState::Delivered
                } else {
                    OperationState::Accepted
                },
                if notice {
                    "managed user turn written"
                } else {
                    "managed request written; harness evidence pending"
                }
                .to_owned(),
            )?;
        }
        drop(table);
        self.writer.barrier()?;
        self.wake();
        Ok(())
    }
}
