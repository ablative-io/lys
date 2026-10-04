//! The old build's half of a handover: quiet every session at a boundary,
//! keep the handover record, answer, and replace this process's program
//! with the new build. See [`crate::handover`] for the whole of it.

use std::collections::BTreeMap;
use std::os::fd::AsFd;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::{Sessions, Table};
use crate::error::RunnerError;
use crate::handover::{Handed, Holding, Manifest};
use crate::protocol::Answer;

/// Where the answer goes once the runner is quiet and its record kept, and
/// whether the program is then replaced: `true` once the answer is written.
pub(crate) struct Handshake<'a> {
    /// Takes `handing_over`, or the refusal when the runner never got there.
    pub(crate) ready: Option<tokio::sync::oneshot::Sender<Answer>>,
    /// Answers whether the answer reached the caller.
    pub(crate) go: &'a std::sync::mpsc::Receiver<bool>,
}

impl Table {
    /// Whether a handover is under way: nothing is typed or started.
    pub(crate) fn is_handing_over(&self) -> bool {
        self.handing_over
    }

    /// Deliver what waits for a boundary in every running session that is
    /// already between turns: a handover held it back.
    pub(crate) fn deliver_idle(&mut self) {
        let idle: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.ended.is_none() && session.guard.idle)
            .map(|(id, _)| id.clone())
            .collect();
        for id in idle {
            crate::operations::deliver(self, &id);
        }
    }

    /// Refused `runner_stopping` or `runner_handing_over` when no start may
    /// begin.
    pub(crate) fn open_for_starts(&self) -> Result<(), RunnerError> {
        if self.stopping {
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner is stopping and starts nothing",
            ));
        }
        if self.handing_over {
            return Err(RunnerError::refused(
                "runner_handing_over",
                "the runner is handing its sessions to a new build and starts nothing until it has",
            ));
        }
        Ok(())
    }
}

impl Sessions {
    /// Hold back recording an exit while a handover is under way, and say
    /// one is being recorded; `false` when the table cannot be read.
    pub(super) fn settle_begins(&self, id: &str, generation: u64) -> bool {
        let Some(mut table) = self.lock_logged() else {
            return false;
        };
        while table.handing_over {
            table = match self.wait(table) {
                Ok(table) => table,
                Err(error) => {
                    crate::error::said(&format!("session {id}: {error}"));
                    return false;
                }
            };
        }
        table.settling.insert((id.to_owned(), generation));
        true
    }

    /// The exit [`Sessions::settle_begins`] held is recorded.
    pub(super) fn settle_ends(&self, id: &str, generation: u64) {
        if let Some(mut table) = self.lock_logged() {
            table.settling.remove(&(id.to_owned(), generation));
            drop(table);
        }
        self.wake();
    }

    /// Wake every session's input path, so a handover waiting on it sees
    /// its caller left.
    pub(crate) fn wake_inputs(&self) {
        if let Ok(table) = self.lock() {
            for session in table.sessions.values() {
                if let Some(live) = &session.live {
                    live.writer.wake();
                }
            }
        }
        self.quiesce.wake();
        self.wake();
    }

    /// Hand every running session to `binary`. Answers only when the
    /// program was not replaced: refused before anything changed, given up
    /// because the caller left, or failed at the replacement, each by name,
    /// with every session running on in this build.
    pub(crate) fn hand_over(
        self: &Arc<Self>,
        holding: &Holding,
        (binary, environment): (&str, &BTreeMap<String, String>),
        left: &AtomicBool,
        handshake: &mut Handshake<'_>,
    ) -> Result<(), RunnerError> {
        crate::handover::check_format(binary, environment)?;
        {
            let mut table = self.lock()?;
            table.open_for_starts()?;
            table.handing_over = true;
        }
        let result = self.quiet_and_replace(holding, (binary, environment), left, handshake);
        let resumed = self.resume_after_handover();
        result.and(resumed)
    }

    fn quiet_and_replace(
        self: &Arc<Self>,
        holding: &Holding,
        (binary, environment): (&str, &BTreeMap<String, String>),
        left: &AtomicBool,
        handshake: &mut Handshake<'_>,
    ) -> Result<(), RunnerError> {
        self.until_any(left, |table| {
            (table.starting.is_empty() && table.settling.is_empty()).then_some(())
        })?;
        let inputs: Vec<_> = self
            .lock()?
            .sessions
            .values()
            .filter_map(|session| session.live.as_ref().map(|live| live.writer.clone()))
            .collect();
        for input in &inputs {
            input.close(left)?;
        }
        self.quiesce.ask(left)?;
        let mut table = self.lock()?;
        let mut manifest = self.manifest(&mut table, holding, binary)?;
        let path = crate::handover::path(&holding.state);
        let kept = self
            .keep_descriptors(&table, holding, &mut manifest)
            .and_then(|()| manifest.write(&path));
        if let Err(error) = kept {
            let released = self.release_descriptors(&table, holding);
            drop(table);
            released.and(remove_record(&path))?;
            return Err(error);
        }
        let answer = Answer::HandingOver {
            sessions: manifest
                .sessions
                .iter()
                .map(|handed| handed.session.clone())
                .collect(),
            binary: binary.to_owned(),
        };
        let reached = handshake
            .ready
            .take()
            .is_some_and(|ready| ready.send(answer).is_ok())
            && handshake.go.recv().unwrap_or(false);
        let failure = if reached {
            crate::error::said(&format!(
                "handing {} sessions to {binary}",
                manifest.sessions.len()
            ));
            let error = replace(binary, environment, &path);
            RunnerError::refused(
                "handover_exec_failed",
                format!("{binary} could not replace this runner's program: {error}"),
            )
        } else {
            RunnerError::refused(
                "handover_abandoned",
                "the caller left before the handover was answered",
            )
        };
        let released = self.release_descriptors(&table, holding);
        drop(table);
        released.and(remove_record(&path))?;
        Err(failure)
    }

    /// Take input and output again and record exits again.
    fn resume_after_handover(&self) -> Result<(), RunnerError> {
        let resumed = self.quiesce.resume();
        let mut table = self.lock()?;
        table.handing_over = false;
        for session in table.sessions.values() {
            if let Some(live) = &session.live {
                live.writer.reopen()?;
            }
        }
        table.deliver_idle();
        drop(table);
        self.wake();
        resumed
    }

    /// The record of every running session, each flushed and durable first.
    fn manifest(
        &self,
        table: &mut Table,
        holding: &Holding,
        binary: &str,
    ) -> Result<Manifest, RunnerError> {
        let running: Vec<String> = table
            .sessions
            .iter()
            .filter(|(_, session)| session.ended.is_none() && session.live.is_some())
            .map(|(id, _)| id.clone())
            .collect();
        for id in &running {
            crate::collector::status::flush_status(table, id)?;
        }
        self.persist(table)?;
        self.writer.barrier()?;
        let mut sessions = Vec::new();
        for id in &running {
            let Some(session) = table.sessions.get(id) else {
                continue;
            };
            let Some(live) = &session.live else {
                continue;
            };
            let leader = live.leader.clone().ok_or_else(|| {
                RunnerError::refused(
                    "handover_leader_unproved",
                    format!("session {id}'s leader start identity was not recorded"),
                )
            })?;
            let output = session.output.lock()?;
            sessions.push(Handed {
                session: id.clone(),
                generation: session.generation,
                leader,
                terminal: -1,
                tty: live.terminal.name()?,
                started_at: session.started_at,
                columns: session.columns,
                rows: session.rows,
                launch: session.launch.clone(),
                rotation: session.rotation.clone(),
                policy: session.guard.policy.clone(),
                tracking: session.guard.tracking.clone(),
                proxy: session.guard.proxy.clone(),
                cwd: session.guard.cwd.clone(),
                idle: session.guard.idle,
                ending: session.ending,
                stopped: session.stopped.clone(),
                output: crate::protocol::hex(&output.scrollback().kept()),
                oldest: output.scrollback().oldest(),
                tripped: output.tripped(),
            });
        }
        Ok(Manifest {
            format: crate::handover::FORMAT.to_owned(),
            binary: binary.to_owned(),
            socket: holding.socket.clone(),
            state: holding.state.clone(),
            server_key: crate::protocol::hex(&holding.server_key),
            scrollback: self.scrollback,
            listener: -1,
            state_lock: -1,
            sessions,
            gaps: table.gaps.clone(),
        })
    }
}

impl Sessions {
    /// Keep the listener, the state lock and each running session's
    /// terminal open across the replacement, writing their numbers into
    /// `manifest`.
    fn keep_descriptors(
        &self,
        table: &Table,
        holding: &Holding,
        manifest: &mut Manifest,
    ) -> Result<(), RunnerError> {
        manifest.listener = crate::fd_own::keep(holding.listener.as_fd())?;
        manifest.state_lock = crate::fd_own::keep(self.state.lock_fd())?;
        for handed in &mut manifest.sessions {
            let live = table
                .sessions
                .get(&handed.session)
                .and_then(|session| session.live.as_ref())
                .ok_or_else(|| {
                    RunnerError::refused(
                        "handover_session_gone",
                        format!("session {} left the table while the runner was quiet", handed.session),
                    )
                })?;
            handed.terminal = crate::fd_own::keep(live.terminal.as_fd())?;
        }
        Ok(())
    }

    /// Close every kept descriptor on a replacement again: the runner goes
    /// on in this build.
    fn release_descriptors(&self, table: &Table, holding: &Holding) -> Result<(), RunnerError> {
        let mut result = crate::fd_own::unkeep(holding.listener.as_fd())
            .and(crate::fd_own::unkeep(self.state.lock_fd()));
        for session in table.sessions.values() {
            if let Some(live) = &session.live {
                result = result.and(crate::fd_own::unkeep(live.terminal.as_fd()));
            }
        }
        result
    }
}

/// Replace this process's program with `binary`'s take-over of `record`;
/// answers only when the replacement failed.
fn replace(binary: &str, environment: &BTreeMap<String, String>, record: &Path) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    std::process::Command::new(binary)
        .env_clear()
        .envs(environment)
        .args(["runner", "take-over", "--manifest"])
        .arg(record)
        .exec()
}

/// Remove the handover record at `path`; one never written is no failure.
fn remove_record(path: &Path) -> Result<(), RunnerError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(RunnerError::refused(
            "handover_record_unremoved",
            format!("{}: {error}", path.display()),
        )),
    }
}
