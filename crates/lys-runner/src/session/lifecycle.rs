//! A session's process: started in its terminal, its output kept, its exit
//! seen, and at a usage limit moved to its next account. One thread reads
//! the terminal and one waits on the process; neither runs on a clock.
//!
//! A tracked session's harness is checked before it runs: the executable
//! the launch names is found and asked its version, and one that reports a
//! version its adapter was not measured against is refused
//! `tracking_contract_unsupported`. Its leader's start identity is read at
//! spawn, for the peer proof. Its stream is followed by one more thread,
//! woken by the filesystem's change notices on the stream's directory and
//! never on a timer; it reads from the saved offset to the last whole line,
//! so a restart never reads the stream again from its start, a partial last
//! line is left for the next read, and a truncated or replaced file is a new
//! generation, said by name.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use portable_pty::Child;

use super::{Live, Session, Sessions, Starting, now_ms, unknown};
use crate::error::RunnerError;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow};
use crate::tracking::Harness;

pub use crate::peer::Collected;

mod launch;
mod stream;
mod terminal;

pub(crate) use launch::{
    bound_directory, launched, tracking_started, trust_recorded, window_limit,
};
use stream::stop_follower;
pub(crate) use stream::{accounts, append};

#[cfg(test)]
#[path = "../../tests/lifecycle/cases.rs"]
mod io_tests;

#[cfg(test)]
#[path = "../../tests/output/cases.rs"]
pub(super) mod output_tests;

pub(crate) fn transcript_parent(path: &Path) -> Result<PathBuf, RunnerError> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            RunnerError::refused(
                "transcript_parent_missing",
                format!("{} has no containing directory", path.display()),
            )
        })
}

/// What wakes a session's stream follower.
#[derive(Debug)]
pub(crate) enum Wake {
    /// The stream's directory changed.
    Changed,
    /// Change notices were lost; the stream is read from its cursor again.
    Lost(String),
    /// The session ended, or its stream was bound to another file.
    Stop,
}

pub(super) struct SpawnPlan {
    program: String,
    arguments: Vec<String>,
    directory: String,
    environment: std::collections::BTreeMap<String, String>,
    columns: u16,
    rows: u16,
}

pub(super) fn plan(session: &Session, resumed: bool) -> Result<SpawnPlan, RunnerError> {
    let launch = session.launch.as_ref().ok_or_else(|| {
        RunnerError::refused("session_launch_missing", "the session has no held launch")
    })?;
    let mut environment = launch.environment.clone();
    let mut arguments = launch.arguments.clone();
    if let Some(rotation) = &session.rotation {
        environment.insert(
            rotation.variable().to_owned(),
            rotation.account().to_owned(),
        );
        if let (true, Some(resume)) = (resumed, rotation.resume_arguments()) {
            arguments = resume.to_vec();
        }
    }
    Ok(SpawnPlan {
        program: launch.program.clone(),
        arguments,
        directory: launch.directory.clone(),
        environment,
        columns: session.columns,
        rows: session.rows,
    })
}

pub(super) struct Prepared {
    spawned: Option<crate::pty::Spawned>,
    leader: Leader,
}

impl Drop for Prepared {
    fn drop(&mut self) {
        if let Some(mut spawned) = self.spawned.take() {
            match crate::pty::end_group(spawned.pid) {
                Ok(()) => {
                    if let Err(error) = spawned.child.wait() {
                        crate::error::said(&format!("cancelled_spawn_exit_unconfirmed: {error}"));
                    }
                }
                Err(error) => {
                    crate::error::said(&format!("cancelled_spawn_cleanup_failed: {error}"));
                }
            }
        }
    }
}

pub(super) struct Pending {
    pid: u32,
    generation: u64,
    reader: Box<dyn Read + Send>,
    child: Box<dyn Child + Send + Sync>,
    output: Arc<super::output::OutputHandle>,
}

struct FailedActivation {
    pid: u32,
    child: Box<dyn Child + Send + Sync>,
    pump: Option<std::thread::JoinHandle<()>>,
    failure: RunnerError,
}

impl Pending {
    pub(super) fn cancel(mut self) -> Result<(), RunnerError> {
        crate::pty::end_group(self.pid)?;
        self.child.wait().map_err(|error| {
            RunnerError::refused("cancelled_spawn_exit_unconfirmed", error.to_string())
        })?;
        Ok(())
    }
}

impl Sessions {
    /// Prepare a verified child without owning the session table.
    pub(super) fn run(&self, plan: &SpawnPlan) -> Result<Prepared, RunnerError> {
        self.writer.barrier()?;
        #[cfg(test)]
        if let Some(probe) = self
            .spawn_probe
            .lock()
            .map_err(|error| RunnerError::refused("spawn_probe_failed", error.to_string()))?
            .take()
        {
            probe();
        }
        let mut spawned = crate::pty::spawn(&crate::pty::Spawn {
            program: &plan.program,
            arguments: &plan.arguments,
            directory: &plan.directory,
            environment: &plan.environment,
            columns: plan.columns,
            rows: plan.rows,
        })?;
        let start = match crate::peer::start_identity(spawned.pid) {
            Ok(start) => start,
            Err(error) => {
                crate::pty::end_group(spawned.pid)?;
                spawned.child.wait().map_err(|waited| {
                    RunnerError::refused("cancelled_spawn_exit_unconfirmed", waited.to_string())
                })?;
                return Err(error);
            }
        };
        let leader = Leader {
            pid: spawned.pid,
            start,
        };
        Ok(Prepared {
            spawned: Some(spawned),
            leader,
        })
    }

    pub(super) fn install(
        session: &mut Session,
        mut prepared: Prepared,
    ) -> Result<Pending, RunnerError> {
        let generation = session.generation + 1;
        session.output.begin(
            generation,
            session.rotation.as_ref(),
            session
                .guard
                .tracking
                .as_ref()
                .is_some_and(|tracking| tracking.harness == Harness::ClaudeCode),
        )?;
        let spawned = prepared.spawned.take().ok_or_else(|| {
            RunnerError::refused(
                "spawn_install_failed",
                "the prepared child was already installed",
            )
        })?;
        session.generation = generation;
        session.pid = Some(spawned.pid);
        session.guard.leader = Some(prepared.leader.clone());
        session.leader_start = Some(prepared.leader.clone());
        session.live = Some(Live {
            writer: crate::input::Input::new(spawned.writer),
            master: spawned.master,
            pid: spawned.pid,
            leader: Some(prepared.leader.clone()),
        });
        Ok(Pending {
            pid: spawned.pid,
            generation: session.generation,
            reader: spawned.reader,
            child: spawned.child,
            output: Arc::clone(&session.output),
        })
    }

    /// Activate only after the generation is visible in the table.
    pub(super) fn activate(
        self: &Arc<Self>,
        id: &str,
        pending: Pending,
    ) -> Result<(), RunnerError> {
        let Pending {
            pid,
            generation,
            reader,
            child,
            output,
        } = pending;
        let pumped = Arc::clone(self);
        let owned = id.to_owned();
        let pumped_output = Arc::clone(&output);
        let pump = match std::thread::Builder::new()
            .name("runner-output".to_owned())
            .spawn(move || pumped.pump(&owned, generation, &pumped_output, reader))
        {
            Ok(pump) => pump,
            Err(error) => {
                let failure =
                    RunnerError::refused("session_output_worker_failed", error.to_string());
                return self.activation_failed(
                    id,
                    generation,
                    &output,
                    FailedActivation {
                        pid,
                        child,
                        pump: None,
                        failure,
                    },
                );
            }
        };
        let observer = Arc::clone(self);
        let owned = id.to_owned();
        let watched_output = Arc::clone(&output);
        let (deliver, delivered) = mpsc::channel();
        let watcher = match std::thread::Builder::new()
            .name("runner-process-exit".to_owned())
            .spawn(move || match delivered.recv() {
                Ok((child, pump)) => {
                    observer.watch(&owned, generation, &watched_output, child, pump);
                }
                Err(error) => {
                    crate::error::said(&format!("session_exit_worker_failed: {error}"));
                }
            }) {
            Ok(watcher) => watcher,
            Err(error) => {
                let failure = RunnerError::refused("session_exit_worker_failed", error.to_string());
                return self.activation_failed(
                    id,
                    generation,
                    &output,
                    FailedActivation {
                        pid,
                        child,
                        pump: Some(pump),
                        failure,
                    },
                );
            }
        };
        if let Err(error) = deliver.send((child, pump)) {
            let (child, pump) = error.0;
            if watcher.join().is_err() {
                crate::error::said("session_exit_worker_failed: the exit worker panicked");
            }
            let failure = RunnerError::refused(
                "session_exit_worker_failed",
                "the exit worker ended before it took its child",
            );
            return self.activation_failed(
                id,
                generation,
                &output,
                FailedActivation {
                    pid,
                    child,
                    pump: Some(pump),
                    failure,
                },
            );
        }
        drop(watcher);
        Ok(())
    }

    fn activation_failed(
        &self,
        id: &str,
        generation: u64,
        output: &super::output::OutputHandle,
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

    pub(super) fn replace_generation(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        plan: &SpawnPlan,
        started_at: Option<u64>,
        follow: bool,
    ) -> Result<bool, RunnerError> {
        let mut table = self.lock()?;
        let session = table.sessions.get(id).ok_or_else(|| unknown(id))?;
        if table.stopping || session.generation != generation {
            return Ok(false);
        }
        if !table.starting.insert(id.to_owned()) {
            return Err(RunnerError::refused(
                "session_starting",
                "the session already has a launch in progress",
            ));
        }
        drop(table);
        let reservation = Starting {
            sessions: Arc::clone(self),
            id: id.to_owned(),
        };
        let prepared = self.run(plan)?;
        let mut table = self.lock()?;
        let session = table.sessions.get(id).ok_or_else(|| unknown(id))?;
        if table.stopping || session.generation != generation {
            drop(table);
            drop(prepared);
            return Ok(false);
        }
        crate::collector::status::flush_status(&mut table, id)?;
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        let pending = Self::install(session, prepared)?;
        session.ended = None;
        session.ending = false;
        session.guard.idle = true;
        if let Some(started_at) = started_at {
            session.started_at = started_at;
        }
        let followed = if follow {
            self.follow(&mut table, id)
        } else {
            Ok(())
        };
        let recorded = self.persist(&table);
        drop(table);
        self.activate(id, pending)?;
        drop(reservation);
        recorded?;
        self.writer.barrier()?;
        if let Err(error) = followed {
            self.end(id, &std::sync::atomic::AtomicBool::new(false))?;
            return Err(error);
        }
        self.wake();
        Ok(true)
    }
}
