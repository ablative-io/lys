//! Failed activation is reaped and recorded before its refusal is returned.
use super::{Ended, EndedHow, FailedActivation, Sessions, now_ms, stop_follower};
use crate::error::RunnerError;

impl Sessions {
    pub(super) fn activation_failed(
        &self,
        id: &str,
        generation: u64,
        output: &crate::session::output::OutputHandle,
        failed: FailedActivation,
    ) -> Result<(), RunnerError> {
        let FailedActivation {
            pid,
            mut child,
            pump,
            failure,
        } = failed;
        let signalled = crate::pty::end_group(pid);
        if let Err(error) = &signalled {
            crate::error::said(&format!(
                "session {id}: cancelled_spawn_cleanup_failed: {error}"
            ));
        }
        let waited = child.wait();
        if let Err(error) = &waited {
            crate::error::said(&format!(
                "session {id}: cancelled_spawn_exit_unconfirmed: {error}"
            ));
        }
        if pump.is_some_and(|pump| pump.join().is_err()) {
            crate::error::said("session_output_worker_failed: the output worker panicked");
        }
        let ended = Ended {
            how: EndedHow::Exited,
            at: now_ms(),
            status: waited
                .as_ref()
                .ok()
                .filter(|exit| exit.signal().is_none())
                .map(portable_pty::ExitStatus::exit_code),
            signal: waited
                .as_ref()
                .ok()
                .and_then(|exit| exit.signal().map(str::to_owned)),
            reason: Some(match (&signalled, &waited) {
                (Ok(()), Ok(_)) => failure.to_string(),
                (Err(error), Ok(_)) => {
                    format!("{failure}; cancelled_spawn_cleanup_failed: {error}")
                }
                (Ok(()), Err(error)) => {
                    format!("{failure}; cancelled_spawn_exit_unconfirmed: {error}")
                }
                (Err(signal), Err(wait)) => format!(
                    "{failure}; cancelled_spawn_cleanup_failed: {signal}; cancelled_spawn_exit_unconfirmed: {wait}"
                ),
            }),
            stopped: None,
        };
        let mut table = match self.lock() {
            Ok(table) => table,
            Err(error) => {
                if let Err(finish) = output.finish(generation, ended) {
                    crate::error::said(&format!("session {id}: output_exit_failed: {finish}"));
                }
                self.wake();
                return Err(error);
            }
        };
        let mut recorded = Ok(());
        if let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        {
            session.live = None;
            session.ended = Some(ended.clone());
            if let Some(follower) = session.follower.take() {
                stop_follower(id, &follower);
            }
            crate::operations::ended(&mut table, id, &ended);
            recorded = self.persist(&table);
        }
        drop(table);
        let fenced = recorded.and_then(|()| self.writer.barrier());
        let finished = output.finish(generation, ended);
        self.wake();
        fenced?;
        finished?;
        signalled?;
        Err(failure)
    }
}
