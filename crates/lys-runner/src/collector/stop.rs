//! The managed reader owns turn boundaries; passive hooks cannot release them.

use std::sync::Arc;

use crate::error::RunnerError;
use crate::session::Sessions;

impl Sessions {
    pub(super) fn hook_stop(
        self: &Arc<Self>,
        id: &str,
        event: &str,
    ) -> Result<String, RunnerError> {
        self.read_source(id, None);
        let mut table = self.lock()?;
        let managed = table
            .sessions
            .get(id)
            .is_some_and(|session| session.managed.is_some());
        let name = if managed {
            "managed_hook_observed"
        } else if event == "Stop" {
            "turn_end"
        } else {
            "session_end"
        };
        super::flushed(&mut table, self.runner(), id, name)?;
        if event == "Stop" && !managed {
            if let Some(session) = table.sessions.get_mut(id) {
                session.guard.idle = true;
            }
            crate::operations::deliver(&mut table, id);
        }
        drop(table);
        self.wake();
        Ok(format!("{name} kept"))
    }
}
