//! A session generation's terminal: its output kept as it comes, and its
//! process's exit seen, moved to the next account or recorded as its end.

use std::io::Read;
use std::sync::Arc;

use portable_pty::Child;

use super::super::{Sessions, now_ms};
use super::plan;
use crate::protocol::{Ended, EndedHow};

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
                    // A hang-up asks the harness to flush and exit. Further
                    // output may be that flush, so it is not evidence that a
                    // second signal or a kill is safe.
                    if !tripped || told {
                        continue;
                    }
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
                        match crate::pty::end(&leader) {
                            Ok(()) => crate::error::said(&format!(
                                "session {id}: rotation_exit_pending: hang-up sent; waiting for the process exit before changing accounts; explicitly end the session if its program does not exit"
                            )),
                            Err(error) => crate::error::said(&format!(
                                "session {id}: rotation_signal_failed: {error}"
                            )),
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
        if !self.finish_terminal_output(id, generation, pump) {
            return;
        }
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
            signal: signal.clone(),
            reason: seat_exit_reason(&table, id, status, signal.as_deref()),
            stopped,
        };
        self.publish_terminal_exit(id, generation, output, ended, table);
    }
}

impl Sessions {
    fn finish_terminal_output(
        &self,
        id: &str,
        generation: u64,
        pump: std::thread::JoinHandle<()>,
    ) -> bool {
        let Some(table) = self.lock_logged() else {
            return false;
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
        true
    }
    fn publish_terminal_exit(
        &self,
        id: &str,
        generation: u64,
        output: &super::super::output::OutputHandle,
        ended: Ended,
        mut table: std::sync::MutexGuard<'_, crate::session::Table>,
    ) {
        let drains = table
            .sessions
            .get(id)
            .zip(table.feed.source(id))
            .is_some_and(|(session, source)| session.guard.proxy_source(source).is_some());
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        else {
            return;
        };
        session.ended = Some(ended.clone());
        super::drain::end_follower(id, session, drains);
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

fn seat_exit_reason(
    table: &crate::session::Table,
    id: &str,
    status: Option<u32>,
    signal: Option<&str>,
) -> Option<String> {
    table
        .sessions
        .get(id)
        .filter(|session| {
            session.launch.as_ref().is_some_and(|launch| {
                launch
                    .environment
                    .contains_key(crate::harness_control::seat_program::SPEC_VARIABLE)
            })
        })
        .map(|_| format!("seat_program_exited: status {status:?}; signal {signal:?}"))
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
