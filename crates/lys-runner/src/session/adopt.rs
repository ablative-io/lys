//! The new build's half of a handover: take up every session the record
//! names, as the same process its sessions' processes are children of.
//!
//! Each session's terminal is found by the number the record kept and
//! checked against the name the record gave its other end; its scrollback,
//! cursors, rotation, policy and tracking go on from where the old build
//! stopped; its output and its exit are watched again, its stream followed
//! again from its saved cursor. A session that cannot be taken up is hung
//! up, its exit waited on with its real status, and recorded ended
//! `handover_failed` with the reason; it is never left running unowned.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::{Guard, Live, Session, Sessions, now_ms, output};
use crate::error::RunnerError;
use crate::handover::{Handed, Manifest};
use crate::protocol::{Ended, EndedHow};
use crate::state::StateFile;
use crate::tracking::Harness;

impl Sessions {
    /// The sessions of `state`, those `manifest` hands over taken up, its
    /// record removed once they are recorded.
    pub(crate) fn take_over(
        state: StateFile,
        manifest: &Manifest,
        record: &std::path::Path,
    ) -> Result<Arc<Self>, RunnerError> {
        let handed: BTreeSet<String> = manifest
            .sessions
            .iter()
            .map(|handed| handed.session.clone())
            .collect();
        let sessions = Self::open_held(
            state,
            &manifest.state,
            manifest.scrollback,
            &handed,
            BTreeMap::new(),
        )?;
        sessions.lock()?.gaps.extend(manifest.gaps.clone());
        for handed in &manifest.sessions {
            if let Err(error) = sessions.adopt(handed) {
                crate::error::said(&format!(
                    "session {}: handover_session_refused: {error}",
                    handed.session
                ));
                sessions.end_unadopted(handed, error.to_string());
            }
        }
        let mut table = sessions.lock()?;
        table.deliver_idle();
        sessions.persist(&table)?;
        drop(table);
        sessions.writer.barrier()?;
        std::fs::remove_file(record).map_err(|error| {
            RunnerError::refused(
                "handover_record_unremoved",
                format!("{}: {error}", record.display()),
            )
        })?;
        sessions.wake();
        Ok(sessions)
    }

    fn restored_output(&self, handed: &Handed) -> Result<Arc<output::OutputHandle>, RunnerError> {
        let bytes = crate::protocol::unhex(&handed.output).ok_or_else(|| {
            RunnerError::refused(
                "handover_record_unreadable",
                format!("session {}'s scrollback is not lowercase hex", handed.session),
            )
        })?;
        let scrollback =
            crate::scrollback::Scrollback::restored(self.scrollback, handed.oldest, &bytes);
        let claude = handed
            .tracking
            .as_ref()
            .is_some_and(|tracking| tracking.harness == Harness::ClaudeCode);
        Ok(Arc::new(output::OutputHandle::restored(
            scrollback,
            handed.generation,
            handed.rotation.as_ref(),
            claude,
            handed.tripped,
        )?))
    }

    fn held(handed: &Handed, output: Arc<output::OutputHandle>, live: Option<Live>) -> Session {
        Session {
            started_at: handed.started_at,
            pid: Some(handed.leader.pid),
            leader_start: Some(handed.leader.clone()),
            columns: handed.columns,
            rows: handed.rows,
            output,
            ended: None,
            ending: handed.ending || live.is_none(),
            live,
            generation: handed.generation,
            launch: handed.launch.clone(),
            rotation: handed.rotation.clone(),
            guard: Guard {
                policy: handed.policy.clone(),
                tracking: handed.tracking.clone(),
                leader: Some(handed.leader.clone()),
                cwd: handed.cwd.clone(),
                idle: handed.idle,
            },
            follower: None,
            pending_status: None,
            stopped: handed.stopped.clone(),
        }
    }

    /// Take `handed` up: its terminal, its output and its exit.
    fn adopt(self: &Arc<Self>, handed: &Handed) -> Result<(), RunnerError> {
        let fd = crate::fd_own::take(handed.terminal)?;
        let terminal = crate::pty_master::Master::checked(fd)?;
        let name = terminal.name()?;
        if name != handed.tty {
            return Err(RunnerError::refused(
                "handover_terminal_mismatch",
                format!(
                    "descriptor {} is terminal {name}, and the record names {}",
                    handed.terminal, handed.tty
                ),
            ));
        }
        let reader = terminal.reader()?;
        let writer = terminal.writer()?;
        let output = self.restored_output(handed)?;
        let live = Live {
            writer: crate::input::Input::new(Box::new(writer)),
            terminal,
            pid: handed.leader.pid,
            leader: Some(handed.leader.clone()),
        };
        let id = handed.session.clone();
        {
            let mut table = self.lock()?;
            table.sessions.insert(
                id.clone(),
                Self::held(handed, Arc::clone(&output), Some(live)),
            );
            self.follow(&mut table, &id)?;
        }
        self.watch_adopted(&id, handed.generation, output, reader, handed.leader.pid)
    }

    /// Read the terminal and watch the exit of an adopted generation.
    fn watch_adopted(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        output: Arc<output::OutputHandle>,
        reader: crate::pty_master::Reader,
        pid: u32,
    ) -> Result<(), RunnerError> {
        let pumped = Arc::clone(self);
        let (owned, pumped_output) = (id.to_owned(), Arc::clone(&output));
        self.quiesce.enter(id, generation);
        let pump = std::thread::Builder::new()
            .name("runner-output".to_owned())
            .spawn(move || {
                pumped.pump(&owned, generation, &pumped_output, reader);
                pumped.quiesce.leave(&owned, generation);
            })
            .map_err(|error| {
                self.quiesce.leave(id, generation);
                RunnerError::refused("session_output_worker_failed", error.to_string())
            })?;
        let observer = Arc::clone(self);
        let owned = id.to_owned();
        std::thread::Builder::new()
            .name("runner-process-exit".to_owned())
            .spawn(move || observer.watch(&owned, generation, &output, pid, pump))
            .map(drop)
            .map_err(|error| {
                // The reader runs on with nobody to record the exit; the
                // session is ended here instead, by its caller.
                RunnerError::refused("session_exit_worker_failed", error.to_string())
            })
    }

    /// End `handed`, which could not be taken up for `why`: hang it up,
    /// wait for its exit with its status, end what it left, record it.
    fn end_unadopted(self: &Arc<Self>, handed: &Handed, why: String) {
        let output = Arc::new(output::OutputHandle::new(self.scrollback, None));
        let generation = handed.generation;
        let begun = output.begin(generation, None, false);
        if let Some(mut table) = self.lock_logged() {
            if let Some(session) = table.sessions.get_mut(&handed.session)
                && let Some(live) = session.live.take()
            {
                // A session whose reader started keeps it: only its exit
                // watch failed, so its terminal is closed with it here.
                drop(live);
            }
            table.sessions.insert(
                handed.session.clone(),
                Self::held(handed, Arc::clone(&output), None),
            );
        }
        if let Err(error) = begun {
            crate::error::said(&format!("session {}: {error}", handed.session));
        }
        let ending = Arc::clone(self);
        let (id, leader) = (handed.session.clone(), handed.leader.clone());
        let spawned = std::thread::Builder::new()
            .name("runner-unadopted-exit".to_owned())
            .spawn(move || {
                if let Err(error) = crate::pty::end(&leader) {
                    crate::error::said(&format!("session {id}: {error}"));
                }
                let exit = crate::process_exit::observe(leader.pid);
                let left = crate::pty::end_left_group(&leader);
                ending.record_unadopted(&id, generation, &output, exit, &why, left);
                if let Err(error) = crate::process_exit::reap(leader.pid) {
                    crate::error::said(&format!("session {id}: {error}"));
                }
            });
        if let Err(error) = spawned {
            crate::error::said(&format!(
                "session {}: session_exit_worker_failed: {error}",
                handed.session
            ));
            if let Err(error) = crate::pty::end_group(handed.leader.pid) {
                crate::error::said(&format!("session {}: {error}", handed.session));
            }
        }
    }

    fn record_unadopted(
        &self,
        id: &str,
        generation: u64,
        output: &output::OutputHandle,
        exit: Result<crate::process_exit::Exit, String>,
        why: &str,
        left: Result<crate::pty::Left, RunnerError>,
    ) {
        let (status, signal, seen) = match exit {
            Ok(exit) => (exit.status, exit.signal, None),
            Err(error) => (None, None, Some(error)),
        };
        let left = match left {
            Ok(crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None }) => None,
            Ok(other) => Some(format!("what it left: {other:?}")),
            Err(error) => Some(format!("what it left: {error}")),
        };
        let reason = [Some(why.to_owned()), seen, left]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("; ");
        let Some(mut table) = self.lock_logged() else {
            return;
        };
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        else {
            return;
        };
        let ended = Ended {
            how: EndedHow::HandoverFailed,
            at: now_ms(),
            status,
            signal,
            reason: Some(reason),
            stopped: session.stopped.clone(),
        };
        session.ended = Some(ended.clone());
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
