//! A proved peer restarts its held launch only after the old process ends.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::launch::valid_id;
use super::{Sessions, lifecycle, now_ms, unknown};
use crate::error::RunnerError;
use crate::operations::{OperationOutcome, begin_restart, finish_restart};
use crate::peer::Leader;
use crate::rotation::RotationState;

impl Sessions {
    pub(crate) fn restart_proved(
        self: &Arc<Self>,
        id: &str,
        operation: &str,
        leader: &Leader,
    ) -> Result<OperationOutcome, RunnerError> {
        if !valid_id(operation) {
            return Err(RunnerError::refused(
                "operation_invalid",
                "a restart operation id holds letters, digits, '-' or '_'",
            ));
        }
        let (outcome, fresh) = {
            let mut table = self.lock()?;
            if table.stopping {
                return Err(RunnerError::refused(
                    "runner_stopping",
                    "the runner is stopping and restarts nothing",
                ));
            }
            let session = table.sessions.get(id).ok_or_else(|| unknown(id))?;
            if session.ended.is_some() || session.guard.leader.as_ref() != Some(leader) {
                return Err(RunnerError::refused(
                    "not_a_session",
                    "the proved session's leader changed before restart",
                ));
            }
            if session.launch.is_none() {
                return Err(RunnerError::refused(
                    "session_launch_missing",
                    format!("session {id} has no held launch to restart"),
                ));
            }
            begin_restart(&mut table, id, operation)?
        };
        if !fresh {
            return Ok(outcome);
        }
        // Acceptance survives the caller's exit, which ending its own
        // session causes; only the old process's exit releases the restart.
        let result = self.end(id, &AtomicBool::new(false)).and_then(|ended| {
            if ended.status.is_none() && ended.signal.is_none() {
                return Err(RunnerError::refused(
                    "restart_exit_unconfirmed",
                    "the old process's exit has no status or signal; its launch was not restarted",
                ));
            }
            self.relaunch(id)?;
            Ok(ended)
        });
        let mut table = self.lock()?;
        let outcome = finish_restart(&mut table, operation, result);
        drop(table);
        self.writer.barrier()?;
        self.wake();
        outcome
    }

    fn relaunch(self: &Arc<Self>, id: &str) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        if table.stopping {
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner stopped before the session could restart",
            ));
        }
        let native = table
            .feed
            .source(id)
            .and_then(|source| source.native_conversation.clone());
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        let launch = session.launch.as_mut().ok_or_else(|| {
            RunnerError::refused(
                "session_launch_missing",
                format!("session {id} has no held launch"),
            )
        })?;
        if session.managed.is_none() {
            crate::pty_conversation::prepare_restart(launch, native.as_deref())?;
        }
        session.rotation = launch
            .rotation
            .clone()
            .map(RotationState::new)
            .transpose()?;
        session.columns = launch.columns;
        session.rows = launch.rows;
        let plan = lifecycle::plan(session, false)?;
        let generation = session.generation;
        drop(table);
        if !self.replace_generation(id, generation, &plan, Some(now_ms()), true)? {
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner stopped before the prepared restart was installed",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{Leader, Sessions};
    use crate::peer::StartIdentity;

    #[test]
    fn restart_refuses_invalid_operation_ids_before_finding_a_session() -> Result<(), Box<dyn Error>>
    {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 4096)?;
        let leader = Leader {
            pid: std::process::id(),
            start: StartIdentity("unproved".to_owned()),
        };
        for operation in ["", "../restart", "restart name", "restart-é"] {
            let error = sessions
                .restart_proved("missing", operation, &leader)
                .err()
                .ok_or("invalid restart operation id was accepted")?;
            assert_eq!(error.name(), "operation_invalid", "{operation:?}: {error}");
        }
        assert!(sessions.read_lock()?.sessions.is_empty());
        Ok(())
    }
}
