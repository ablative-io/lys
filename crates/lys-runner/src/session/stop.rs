//! Pulling the cord: every running session ended, each recorded with who
//! asked and why.
//!
//! The first ask hangs every session up, as its own end does, so a harness
//! can keep its last output, and answers once every exit is seen: nothing
//! ends the wait but the exits, or the caller leaving. `kill`, or an ask
//! while sessions an earlier one stopped still run, ends each session's
//! proved process group at once. A session not ended when the act answers
//! is named running, never stopped.
//!
//! A settling ask is not a person's: the server sends it again under a pull
//! in force, for a start that crossed it. A session an earlier ask already
//! stopped is left as that ask has it, with no signal sent and no wait on
//! its exit, and is answered ended or running as it stands. Only a person
//! asking again ends at once what a hang-up did not.

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
    /// Every owned seat this runner holds, left with its owner: a seat is
    /// ended at its owner by a signed stop (AGENTS-004), never by the runner
    /// that started it stopping its own sessions.
    pub owned: Vec<String>,
}

/// How a stop of everything asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ask {
    /// Whether each proved process group is ended at once.
    pub kill: bool,
    /// Whether the server is settling a start under a pull in force: a
    /// session an earlier ask already stopped is left as that ask has it.
    pub settling: bool,
}

/// The sessions one ask found running.
#[derive(Default)]
struct Corded {
    /// Every session that had not ended, in the table's order.
    asked: Vec<String>,
    /// Those among them this ask signalled, whose exits it waits on.
    signalled: Vec<String>,
}

impl Sessions {
    /// End every running session for `by`, because `reason`, as `ask` says.
    pub fn stop_everything(
        &self,
        by: &str,
        reason: &str,
        ask: Ask,
        left: &AtomicBool,
    ) -> Result<StoppedEverything, RunnerError> {
        let (by, reason) = (by.trim(), reason.trim());
        if by.is_empty() || reason.is_empty() {
            return Err(RunnerError::refused(
                "stop_words_missing",
                "a stop of everything names who asked and why",
            ));
        }
        let Corded { asked, signalled } = self.cord(
            &Stopped {
                by: by.to_owned(),
                reason: reason.to_owned(),
                at: super::now_ms(),
            },
            ask,
        )?;
        // The wait is on the sessions this ask signalled. One a settling
        // ask left as it was is answered as it stands, never waited on.
        let waited = self.until_any(left, |table| {
            signalled
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
        let owned = self
            .owned_seats()?
            .into_iter()
            .map(|seat| seat.binding.session)
            .collect();
        Ok(StoppedEverything {
            sessions,
            running,
            owned,
        })
    }

    /// Ask every running session to end as `stopped` says, answering their
    /// ids. A session already stopped by an earlier ask is ended at once,
    /// unless this ask is settling: then it is left exactly as it is.
    fn cord(&self, stopped: &Stopped, ask: Ask) -> Result<Corded, RunnerError> {
        let mut table = self.lock()?;
        let mut corded = Corded::default();
        let mut pending = Vec::new();
        for (id, session) in &mut table.sessions {
            if session.ended.is_some() {
                continue;
            }
            corded.asked.push(id.clone());
            let again = session.stopped.is_some();
            if again && ask.settling {
                continue;
            }
            session.ending = true;
            if session.stopped.is_none() {
                session.stopped = Some(stopped.clone());
            }
            if let Some(live) = &session.live {
                let sent = if ask.kill || again {
                    live.leader
                        .as_ref()
                        .ok_or_else(|| {
                            RunnerError::refused(
                                "leader_unproved",
                                "the process's start identity was not recorded",
                            )
                        })
                        .and_then(crate::pty::prepare_left_group)
                        .map(|ending| {
                            pending.push((id.clone(), ending));
                        })
                } else {
                    live.end()
                };
                if let Err(error) = sent {
                    crate::error::said(&format!("session {id}: stop_signal_failed: {error}"));
                }
            }
            corded.signalled.push(id.clone());
        }
        after_table(table, || {
            self.wake();
            for (id, ending) in pending {
                match ending.wait(None) {
                    crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None } => {}
                    crate::pty::Left::Ended {
                        reason: Some(reason),
                    } => {
                        crate::error::said(&format!("session {id}: {reason}"));
                    }
                    left => {
                        crate::error::said(&format!("session {id}: stop_signal_failed: {left:?}"));
                    }
                }
            }
        });
        Ok(corded)
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

/// Report a lost session's cleanup without treating a group number as ownership.
/// Missing ownership and members left running are named beside any signal sent.
pub(super) fn left_behind(
    id: &str,
    leader: Option<&crate::peer::Leader>,
) -> (Option<String>, Option<String>) {
    let Some(leader) = leader else {
        return (
            None,
            Some("start identity not recorded; process group not ended".to_owned()),
        );
    };
    let pid = leader.pid;
    match crate::pty::end_left_group(leader) {
        Ok(crate::pty::Left::Gone) => (None, None),
        Ok(crate::pty::Left::Ended { reason }) => {
            crate::error::said(&format!(
                "session {id}: proved members of process group {pid} were sent SIGKILL"
            ));
            (Some("SIGKILL".to_owned()), reason)
        }
        Ok(crate::pty::Left::Reused) => (None, Some("process group reused, not ended".to_owned())),
        Ok(crate::pty::Left::Unended { reason }) => (None, Some(reason)),
        Err(error) => {
            crate::error::said(&format!("session {id}: {error}"));
            (None, Some(error.to_string()))
        }
    }
}

fn after_table<T, R>(guard: std::sync::MutexGuard<'_, T>, wait: impl FnOnce() -> R) -> R {
    drop(guard);
    wait()
}

#[cfg(test)]
mod cord_tests {
    use super::after_table;
    use std::sync::{Arc, Mutex, mpsc};

    #[test]
    fn a_pending_exit_wait_leaves_the_session_table_readable()
    -> Result<(), Box<dyn std::error::Error>> {
        let table = Arc::new(Mutex::new(()));
        let ending_table = Arc::clone(&table);
        let (waiting, ready) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let ending = std::thread::spawn(move || -> Result<(), String> {
            let guard = ending_table.lock().map_err(|error| error.to_string())?;
            after_table(guard, || {
                waiting.send(()).map_err(|error| error.to_string())?;
                released.recv().map_err(|error| error.to_string())
            })
        });
        ready.recv()?;
        let readable = table.try_lock().is_ok();
        release.send(())?;
        ending
            .join()
            .map_err(|panic| format!("exit waiter panicked: {:?}", panic.as_ref().type_id()))??;
        assert!(readable, "an exit wait held the session table mutex");
        Ok(())
    }
}
