//! Pulling the cord: every running session ended, each recorded with who
//! asked and why.
//!
//! The first ask hangs every session up, as its own end does, so a harness
//! can keep its last output, and answers once every exit is seen: nothing
//! ends the wait but the exits, or the caller leaving. `kill`, or an ask
//! while sessions an earlier one stopped still run, ends each session's
//! proved process group at once. A session not ended when the act answers
//! is named running, never stopped.

use std::sync::atomic::AtomicBool;

use super::Sessions;
use crate::error::RunnerError;
use crate::protocol::Stopped;

/// What a stop of everything did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoppedEverything {
    /// Every session it ended.
    pub sessions: Vec<String>,
    /// Every session asked to end that had not when it answered.
    pub running: Vec<String>,
}

impl Sessions {
    /// End every running session for `by`, because `reason`.
    pub fn stop_everything(
        &self,
        by: &str,
        reason: &str,
        kill: bool,
        left: &AtomicBool,
    ) -> Result<StoppedEverything, RunnerError> {
        let (by, reason) = (by.trim(), reason.trim());
        if by.is_empty() || reason.is_empty() {
            return Err(RunnerError::refused(
                "stop_words_missing",
                "a stop of everything names who asked and why",
            ));
        }
        let asked = self.cord(
            &Stopped {
                by: by.to_owned(),
                reason: reason.to_owned(),
                at: super::now_ms(),
            },
            kill,
        )?;
        let waited = self.until_any(left, |table| {
            asked
                .iter()
                .all(|id| {
                    table
                        .sessions
                        .get(id)
                        .is_none_or(|session| session.ended.is_some())
                })
                .then_some(())
        });
        let table = self.lock()?;
        let (sessions, running): (Vec<String>, Vec<String>) = asked.into_iter().partition(|id| {
            table
                .sessions
                .get(id)
                .is_none_or(|session| session.ended.is_some())
        });
        drop(table);
        self.writer.barrier()?;
        match waited {
            Ok(()) => {}
            Err(error) if error.name() == "caller_left" => {}
            Err(error) => return Err(error),
        }
        Ok(StoppedEverything { sessions, running })
    }

    /// Ask every running session to end as `stopped` says, answering their
    /// ids. A session already stopped by an earlier ask is ended at once.
    fn cord(&self, stopped: &Stopped, kill: bool) -> Result<Vec<String>, RunnerError> {
        let mut table = self.lock()?;
        let mut asked = Vec::new();
        for (id, session) in &mut table.sessions {
            if session.ended.is_some() {
                continue;
            }
            let again = session.stopped.is_some();
            session.ending = true;
            if session.stopped.is_none() {
                session.stopped = Some(stopped.clone());
            }
            if let Some(live) = &session.live {
                let sent = if kill || again {
                    live.kill_proved()
                } else {
                    live.end()
                };
                if let Err(error) = sent {
                    crate::error::said(&format!("session {id}: stop_signal_failed: {error}"));
                }
            }
            asked.push(id.clone());
        }
        drop(table);
        self.wake();
        Ok(asked)
    }

    /// Mark every running session stopped by the runner's own stop, for
    /// `reason`, before [`Sessions::stop_all`] ends them.
    pub fn stopping_because(&self, reason: &str) -> Result<(), RunnerError> {
        let stopped = Stopped {
            by: "the runner".to_owned(),
            reason: reason.to_owned(),
            at: super::now_ms(),
        };
        let mut table = self.lock()?;
        for session in table.sessions.values_mut() {
            if session.ended.is_none() && session.stopped.is_none() {
                session.stopped = Some(stopped.clone());
            }
        }
        Ok(())
    }
}

impl super::Live {
    /// End every member of the session's proved process group at once.
    pub(crate) fn kill_proved(&self) -> Result<(), RunnerError> {
        let leader = self.leader.as_ref().ok_or_else(|| {
            RunnerError::refused(
                "leader_unproved",
                "the process's start identity was not recorded",
            )
        })?;
        match crate::pty::end_left_group(leader)? {
            crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None } => Ok(()),
            crate::pty::Left::Ended {
                reason: Some(reason),
            } => {
                crate::error::said(&format!("process group {}: {reason}", leader.pid));
                Ok(())
            }
            crate::pty::Left::Reused => Err(RunnerError::refused(
                "process_start_mismatch",
                "the session leader was reused; its group was not signalled",
            )),
            crate::pty::Left::Unended { reason } => Err(RunnerError::refused("end_failed", reason)),
        }
    }
}
