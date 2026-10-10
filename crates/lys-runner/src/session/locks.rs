//! The table lock, its waits and its wakes.

use std::sync::{Arc, MutexGuard};

#[cfg(test)]
use super::lifecycle;
use super::{Sessions, Table, now_ms, table_poisoned, unknown};
use crate::error::RunnerError;

impl Sessions {
    pub(crate) fn read_lock(&self) -> Result<MutexGuard<'_, Table>, RunnerError> {
        #[cfg(test)]
        lifecycle::output_tests::table_locked();
        self.table.lock().map_err(table_poisoned)
    }

    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Table>, RunnerError> {
        let mut table = self.read_lock()?;
        table.operations.prune(now_ms());
        Ok(table)
    }

    pub(crate) fn lock_logged(&self) -> Option<MutexGuard<'_, Table>> {
        match self.lock() {
            Ok(table) => Some(table),
            Err(error) => {
                crate::error::said(&error.to_string());
                self.wake();
                None
            }
        }
    }

    pub(super) fn wait<'a>(
        &self,
        table: MutexGuard<'a, Table>,
    ) -> Result<MutexGuard<'a, Table>, RunnerError> {
        self.changed.wait(table).map_err(table_poisoned)
    }

    /// Wake everything waiting on the table: a caller left, or a thing changed.
    pub fn wake(&self) {
        #[cfg(test)]
        lifecycle::output_tests::table_woken();
        self.changed.notify_all();
    }

    /// Wake a cancelled output request and the shared control waiters.
    pub fn wake_session(&self, id: &str) -> Result<(), RunnerError> {
        let table = self.lock()?;
        let output = Arc::clone(&table.sessions.get(id).ok_or_else(|| unknown(id))?.output);
        self.changed.notify_all();
        drop(table);
        output.wake()
    }
}
