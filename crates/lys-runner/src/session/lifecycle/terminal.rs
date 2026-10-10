//! A session generation's terminal: its output kept as it comes, and its
//! process's exit seen, moved to the next account or recorded as its end.

use std::io::Read;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use portable_pty::Child;

use super::super::{Sessions, now_ms};
use super::plan;
use crate::protocol::{Ended, EndedHow};

/// How long a program has, after the first hang-up at a usage limit, before
/// its process group is killed. A program that exits cleanly on the hang-up
/// is still writing its own session file, which is user data, and a short
/// window is not safe on a loaded machine (load 59 on Tom's Mac that day), so
/// it is ten seconds (Waffles, 10 Oct 2026, 13:18).
const FIRST_HANGUP_GRACE: Duration = Duration::from_secs(10);

/// How long a program has, after the repeated hang-up, before its process
/// group is killed. A program that printed after the hang-up has shown it is
/// still running, not exiting, so the rotation tests' own cadence is enough
/// (Waffles, 10 Oct 2026, 12:09, kept at 13:18).
const REPEAT_GRACE: Duration = Duration::from_secs(2);

/// Which hang-up's grace a generation's escalation is counting down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grace {
    /// [`FIRST_HANGUP_GRACE`] from the first hang-up.
    FirstHangUp,
    /// [`REPEAT_GRACE`] from the repeated hang-up.
    Repeat,
}

impl Grace {
    fn length(self) -> Duration {
        match self {
            Self::FirstHangUp => FIRST_HANGUP_GRACE,
            Self::Repeat => REPEAT_GRACE,
        }
    }

    /// The hang-up it counts from and the grace's name, as the escalated
    /// line says them.
    fn words(self) -> &'static str {
        match self {
            Self::FirstHangUp => "the first hang-up (FIRST_HANGUP_GRACE)",
            Self::Repeat => "the repeated hang-up (REPEAT_GRACE)",
        }
    }
}

impl Sessions {
    /// Keep the output of generation `generation` of session `id` until its
    /// terminal closes.
    pub(super) fn pump(
        &self,
        id: &str,
        generation: u64,
        output: &super::super::output::OutputHandle,
        mut reader: Box<dyn Read + Send>,
    ) {
        let mut buffer = [0_u8; 8192];
        // Whether this generation has been sent its hang-up at a usage limit.
        let mut told = false;
        // Whether the repeat has been said; it is said once, however often it is sent.
        let mut said_again = false;
        // The generation's one escalation, armed at the first hang-up that
        // was sent; a repeat brings its deadline forward.
        let mut escalation: Option<mpsc::Sender<Instant>> = None;
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    crate::error::said(&format!("session {id}: the terminal closed: {error}"));
                    break;
                }
            };
            match output.push(generation, &buffer[..read]) {
                Ok(tripped) => {
                    // A program can drop one hang-up: a shell that is running a
                    // command when it lands only notes it and goes back to
                    // reading. Output from a generation already told to hang up
                    // means its program is still running, so it is told again,
                    // and the runner says so, once. Nothing is sent to a generation
                    // that has gone quiet; this reader never waits on a clock. A program
                    // that took the first hang-up and prints while it shuts down
                    // is hung up again as it prints: one whose handler stands
                    // loses nothing; one whose handler was for a single signal is
                    // ended there. Its output to that point is kept; shutdown work
                    // that is not output, such as writing its own session file, is
                    // lost with it. A program that cannot receive the hang-up (its
                    // SIGHUP blocked) is killed with its group by a thread of its
                    // own (`escalate`): FIRST_HANGUP_GRACE after the first
                    // hang-up, or REPEAT_GRACE after a repeat if that is sooner.
                    if !tripped && !told {
                        continue;
                    }
                    let again = told;
                    let leader = {
                        let Some(mut table) = self.lock_logged() else {
                            return;
                        };
                        table
                            .sessions
                            .get_mut(id)
                            .filter(|session| session.generation == generation && !session.ending)
                            .and_then(|session| {
                                let rotation = session.rotation.as_mut()?;
                                if tripped {
                                    if rotation.tripped() {
                                        return None;
                                    }
                                    rotation.trip();
                                }
                                session.guard.leader.clone()
                            })
                    };
                    if let Some(leader) = leader {
                        told = true;
                        if again && !said_again {
                            said_again = true;
                            crate::error::said(&format!(
                                "session {id}: rotation_signal_repeated: output came after the hang-up"
                            ));
                        }
                        let sent = crate::pty::end(&leader);
                        if sent.is_ok() {
                            let grace = if again {
                                Grace::Repeat
                            } else {
                                Grace::FirstHangUp
                            };
                            if escalation.is_none() {
                                escalation = escalate(id, leader.clone(), grace);
                            } else if let (true, Some(armed)) = (again, &escalation) {
                                // A send that fails finds the escalation
                                // already past its deadline and done.
                                if armed.send(Instant::now()).is_err() {
                                    crate::error::said(&format!(
                                        "session {id}: rotation_escalation_done: the repeat came after the escalation acted"
                                    ));
                                }
                            }
                        }
                        if let Err(error) = sent {
                            // On macOS a hang-up to a group whose leader has
                            // exited and is not yet reaped is refused, so a
                            // repeat that races the exit fails for a session
                            // that ended properly. It is said for what it is.
                            let word = if again {
                                "rotation_repeat_undelivered"
                            } else {
                                "rotation_signal_failed"
                            };
                            crate::error::said(&format!("session {id}: {word}: {error}"));
                        }
                    }
                }
                Err(error) => {
                    crate::error::said(&format!("session {id}: output_record_failed: {error}"));
                    break;
                }
            }
        }
    }

    /// See the exit of generation `generation` of session `id`: move it to
    /// its next account at a usage limit, or record its end. The end is
    /// recorded once the exit is seen and every byte its terminal gave is
    /// kept, so a read that answers an end has read everything before it.
    pub(super) fn watch(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        output: &super::super::output::OutputHandle,
        mut child: Box<dyn Child + Send + Sync>,
        pump: std::thread::JoinHandle<()>,
    ) {
        let waited = child.wait();
        let at = now_ms();
        let Some(table) = self.lock_logged() else {
            return;
        };
        let cleanup = table
            .sessions
            .get(id)
            .filter(|session| session.generation == generation)
            .filter(|session| {
                session.ending
                    || session
                        .rotation
                        .as_ref()
                        .is_some_and(crate::rotation::RotationState::tripped)
            })
            .and_then(|session| session.guard.leader.clone());
        drop(table);
        if let Some(leader) = cleanup {
            if let Some(diagnostic) = terminal_cleanup(crate::pty::end_left_group(&leader)) {
                crate::error::said(&format!("session {id}: {diagnostic}"));
            }
        }
        if pump.join().is_err() {
            crate::error::said(&format!(
                "session {id}: the thread keeping its output ended abnormally"
            ));
        }
        self.read_source(id, None);
        let Some(mut table) = self.lock_logged() else {
            return;
        };
        let status_flushed = crate::collector::status::flush_status(&mut table, id);
        if let Err(error) = &status_flushed {
            crate::error::said(&format!(
                "session {id}: final_status_record_failed: {error}"
            ));
        }
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|s| s.generation == generation)
        else {
            return;
        };
        session.live = None;
        let (status, signal) = match &waited {
            Ok(exit) => match exit.signal() {
                Some(signal) => (None, Some(signal.to_owned())),
                None => (Some(exit.exit_code()), None),
            },
            Err(error) => {
                crate::error::said(&format!(
                    "session {id}: the process's exit could not be read: {error}"
                ));
                (None, None)
            }
        };
        let limit = status_flushed.is_ok()
            && !session.ending
            && session
                .rotation
                .as_ref()
                .is_some_and(|rotation| rotation.limit_at_exit(status));
        let mut how = EndedHow::Exited;
        if limit {
            let next = session
                .rotation
                .as_mut()
                .and_then(|rotation| rotation.advance(at));
            if let Some(moved) = next {
                crate::error::said(&format!(
                    "session {id} reached its usage limit and moves to account {moved}"
                ));
                let next_plan = plan(session, true);
                drop(table);
                match next_plan
                    .and_then(|plan| self.replace_generation(id, generation, &plan, None, false))
                {
                    Ok(true) => return,
                    Ok(false) => {}
                    Err(error) => {
                        crate::error::said(&format!(
                            "session {id}: rotation_spawn_failed: {error}"
                        ));
                    }
                }
                let Some(next_table) = self.lock_logged() else {
                    return;
                };
                table = next_table;
            } else {
                crate::error::said(&format!(
                    "session {id} reached its usage limit on the last of its accounts"
                ));
                how = EndedHow::AccountsExhausted;
            }
        }
        let stopped = table
            .sessions
            .get(id)
            .filter(|session| session.generation == generation)
            .and_then(|session| session.stopped.clone());
        if stopped.is_some() && how == EndedHow::Exited {
            how = EndedHow::Stopped;
        }
        let ended = Ended {
            how,
            at,
            status,
            signal,
            reason: None,
            stopped,
        };
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        else {
            return;
        };
        session.ended = Some(ended.clone());
        super::drain::end_follower(id, session);
        if table
            .sessions
            .get(id)
            .is_some_and(|session| session.guard.tracking.is_some())
        {
            if let Err(error) =
                crate::collector::flushed(&mut table, self.runner(), id, "session_end")
            {
                crate::error::said(&format!("session {id}: final_usage_record_failed: {error}"));
            }
        }
        crate::operations::ended(&mut table, id, &ended);
        self.persist_logged(&table);
        drop(table);
        if let Err(error) = self.writer.barrier() {
            crate::error::said(&format!("session {id}: exit_record_failed: {error}"));
        }
        if let Err(error) = output.finish(generation, ended) {
            crate::error::said(&format!("session {id}: output_exit_failed: {error}"));
        }
        self.wake();
    }
}

fn terminal_cleanup(left: Result<crate::pty::Left, crate::error::RunnerError>) -> Option<String> {
    match left {
        Ok(crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None }) => None,
        Ok(crate::pty::Left::Ended {
            reason: Some(reason),
        }) => Some(format!("group_cleanup_proved: {reason}")),
        Ok(left) => Some(format!("group_cleanup_incomplete: {left:?}")),
        Err(error) => Some(format!("group_cleanup_failed: {error}")),
    }
}

/// After a hang-up of session `id`, give its program `grace`; if the same
/// leader is still running at the deadline, kill its process group and say
/// which grace expired. Each instant sent on the answered channel is a
/// repeated hang-up, and brings the deadline forward to [`REPEAT_GRACE`] after
/// it when that is sooner. A program can hold a hang-up blocked forever (a
/// shell's `exec` from inside its SIGHUP trap passes the block on) and print
/// nothing, and a runner that only answers output would wait on it forever.
/// `None` when the thread could not be started, which is said.
fn escalate(id: &str, leader: crate::peer::Leader, grace: Grace) -> Option<mpsc::Sender<Instant>> {
    let (repeats, heard) = mpsc::channel::<Instant>();
    let session = id.to_owned();
    let spawned = std::thread::Builder::new()
        .name(format!("lys-runner-escalate-{id}"))
        .spawn(move || {
            let id = session;
            let mut grace = grace;
            let mut deadline = Instant::now() + grace.length();
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                match heard.recv_timeout(left) {
                    Ok(repeated) => {
                        let sooner = repeated + REPEAT_GRACE;
                        if sooner < deadline {
                            deadline = sooner;
                            grace = Grace::Repeat;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => break,
                    // The terminal's reader has ended; no repeat can come,
                    // and the deadline still stands.
                    Err(RecvTimeoutError::Disconnected) => {
                        std::thread::sleep(left);
                        break;
                    }
                }
            }
            match crate::pty::kill_if_still(&leader) {
                Ok(true) => crate::error::said(&format!(
                    "session {id}: rotation_signal_escalated: still running {} s after {}; its process group was killed",
                    grace.length().as_secs(),
                    grace.words()
                )),
                Ok(false) => {}
                Err(error) => crate::error::said(&format!(
                    "session {id}: rotation_escalation_failed: {error}"
                )),
            }
        });
    match spawned {
        Ok(_) => Some(repeats),
        Err(error) => {
            crate::error::said(&format!(
                "session {id}: rotation_escalation_failed: the escalation could not be started: {error}"
            ));
            None
        }
    }
}

#[cfg(test)]
mod grace_tests {
    use std::time::Duration;

    use super::{FIRST_HANGUP_GRACE, Grace, REPEAT_GRACE};

    #[test]
    fn the_first_hang_up_grace_is_ten_seconds() {
        assert_eq!(FIRST_HANGUP_GRACE, Duration::from_secs(10));
        assert_eq!(Grace::FirstHangUp.length(), FIRST_HANGUP_GRACE);
        assert_eq!(
            Grace::FirstHangUp.words(),
            "the first hang-up (FIRST_HANGUP_GRACE)"
        );
    }

    #[test]
    fn the_repeat_grace_is_two_seconds() {
        assert_eq!(REPEAT_GRACE, Duration::from_secs(2));
        assert_eq!(Grace::Repeat.length(), REPEAT_GRACE);
        assert_eq!(Grace::Repeat.words(), "the repeated hang-up (REPEAT_GRACE)");
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::terminal_cleanup;
    use crate::error::RunnerError;
    use crate::pty::Left;
    #[test]
    fn successful_cleanup_diagnostics_are_kept_as_a_terminal_group_proof() {
        let diagnostic = "EPERM errno 1; member exit observed";
        assert_eq!(
            terminal_cleanup(Ok(Left::Ended {
                reason: Some(diagnostic.to_owned())
            })),
            Some(format!("group_cleanup_proved: {diagnostic}"))
        );
        for left in [
            Left::Unended {
                reason: "still live".to_owned(),
            },
            Left::Reused,
        ] {
            assert!(
                terminal_cleanup(Ok(left))
                    .is_some_and(|words| words.starts_with("group_cleanup_incomplete:"))
            );
        }
        assert!(
            terminal_cleanup(Err(RunnerError::refused(
                "process_group_unreadable",
                "denied"
            )))
            .is_some_and(|words| words.starts_with("group_cleanup_failed:")
                && words.contains("process_group_unreadable"))
        );
    }
}
