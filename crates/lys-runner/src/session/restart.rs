//! A proved peer restarts its held launch only after the old process ends.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::{Sessions, now_ms, unknown, valid_id};
use crate::error::RunnerError;
use crate::operations::{OperationOutcome, begin_restart, finish_restart};
use crate::peer::Leader;
use crate::protocol::Ended;
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
            let mut table = self.lock();
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
            self.relaunch(id, &ended)?;
            Ok(ended)
        });
        let mut table = self.lock();
        let outcome = finish_restart(&mut table, operation, result);
        drop(table);
        self.wake();
        outcome
    }

    fn relaunch(self: &Arc<Self>, id: &str, ended: &Ended) -> Result<(), RunnerError> {
        let mut table = self.lock();
        if table.stopping {
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner stopped before the session could restart",
            ));
        }
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        let launch = session.launch.as_ref().ok_or_else(|| {
            RunnerError::refused(
                "session_launch_missing",
                format!("session {id} has no held launch"),
            )
        })?;
        session.rotation = launch
            .rotation
            .clone()
            .map(RotationState::new)
            .transpose()?;
        session.columns = launch.columns;
        session.rows = launch.rows;
        session.ending = false;
        session.guard.idle = true;
        session.ended = None;
        if let Err(error) = self.run(id, session, false) {
            session.ended = Some(ended.clone());
            return Err(error);
        }
        session.started_at = now_ms();
        self.follow(&mut table, id);
        self.persist(&table)?;
        drop(table);
        self.wake();
        Ok(())
    }
}
