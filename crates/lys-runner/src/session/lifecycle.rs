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

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use portable_pty::Child;
use serde_json::Value;

use super::{Live, Session, Sessions, Starting, Table, now_ms, unknown};
use crate::error::RunnerError;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Launch};
use crate::tracking::{Accounts, Harness, Reading, Tracking, version_in};
use crate::tracking_store::{Body, Commit, Coverage, SourceState};

pub use crate::peer::Collected;

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
        let mut table = self.lock();
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
        let mut table = self.lock();
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
        let mut table = self.lock();
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

    /// Keep the output of generation `generation` of session `id` until its
    /// terminal closes.
    fn pump(
        &self,
        id: &str,
        generation: u64,
        output: &super::output::OutputHandle,
        mut reader: Box<dyn Read + Send>,
    ) {
        let mut buffer = [0_u8; 8192];
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
                Ok(true) => {
                    let leader = {
                        let mut table = self.lock();
                        table
                            .sessions
                            .get_mut(id)
                            .filter(|session| session.generation == generation && !session.ending)
                            .and_then(|session| {
                                let rotation = session.rotation.as_mut()?;
                                if rotation.tripped() {
                                    return None;
                                }
                                rotation.trip();
                                session.guard.leader.clone()
                            })
                    };
                    if let Some(leader) = leader {
                        if let Err(error) = crate::pty::end(&leader) {
                            crate::error::said(&format!(
                                "session {id}: rotation_signal_failed: {error}"
                            ));
                        }
                    }
                }
                Ok(false) => {}
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
    fn watch(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        output: &super::output::OutputHandle,
        mut child: Box<dyn Child + Send + Sync>,
        pump: std::thread::JoinHandle<()>,
    ) {
        let waited = child.wait();
        let at = now_ms();
        let cleanup = self
            .lock()
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
        if let Some(leader) = cleanup {
            match crate::pty::end_left_group(&leader) {
                Ok(crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None }) => {}
                Ok(left) => {
                    crate::error::said(&format!(
                        "session {id}: group_cleanup_incomplete: {left:?}"
                    ));
                }
                Err(error) => {
                    crate::error::said(&format!("session {id}: group_cleanup_failed: {error}"));
                }
            }
        }
        if pump.join().is_err() {
            crate::error::said(&format!(
                "session {id}: the thread keeping its output ended abnormally"
            ));
        }
        self.read_source(id, None);
        let mut table = self.lock();
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
                table = self.lock();
            } else {
                crate::error::said(&format!(
                    "session {id} reached its usage limit on the last of its accounts"
                ));
                how = EndedHow::AccountsExhausted;
            }
        }
        let ended = Ended {
            how,
            at,
            status,
            signal,
            reason: None,
        };
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        else {
            return;
        };
        session.ended = Some(ended.clone());
        if let Some(follower) = session.follower.take() {
            stop_follower(id, &follower);
        }
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

    /// Follow session `id`'s bound stream on a thread of its own, stopping
    /// any follower it had.
    pub(crate) fn follow(self: &Arc<Self>, table: &mut Table, id: &str) -> Result<(), RunnerError> {
        let Some(path) = table
            .feed
            .source(id)
            .map(|source| PathBuf::from(&source.path))
        else {
            return Ok(());
        };
        let Some(session) = table.sessions.get_mut(id) else {
            return Ok(());
        };
        if let Some(old) = session.follower.take() {
            stop_follower(id, &old);
        }
        let (wake, woken) = mpsc::channel();
        let notices = wake.clone();
        let notifier = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let next = match event {
                Ok(event) if event.need_rescan() => {
                    Wake::Lost("the notifier asked for a rescan".to_owned())
                }
                Ok(_) => Wake::Changed,
                Err(error) => Wake::Lost(error.to_string()),
            };
            if let Err(gone) = notices.send(next) {
                crate::error::said(&format!("a stream follower had ended: {gone}"));
            }
        });
        let dir = match transcript_parent(&path) {
            Ok(dir) => dir,
            Err(error) => {
                crate::error::said(&format!("session {id}: {error}"));
                let source = table.feed.source(id).cloned().unwrap_or_default();
                let coverage = Coverage::of("source_refused", &source, None, error.to_string());
                if let Err(error) = append(table, id, vec![Body::Coverage(coverage)], None) {
                    crate::error::said(&format!("session {id}: coverage_record_failed: {error}"));
                }
                return Err(error);
            }
        };
        let watching = notifier.and_then(|mut each| {
            notify::Watcher::watch(&mut each, &dir, notify::RecursiveMode::NonRecursive)
                .map(|()| each)
        });
        let watcher = match watching {
            Ok(watcher) => watcher,
            Err(error) => {
                let words = format!("{} cannot be watched: {error}", dir.display());
                crate::error::said(&format!("session {id}: coverage_incomplete: {words}"));
                let source = table.feed.source(id).cloned().unwrap_or_default();
                let coverage = Coverage::of("coverage_incomplete", &source, None, words.clone());
                if let Err(error) = append(table, id, vec![Body::Coverage(coverage)], None) {
                    crate::error::said(&format!("session {id}: coverage_record_failed: {error}"));
                }
                return Err(RunnerError::refused("transcript_watch_failed", words));
            }
        };
        let (sessions, owned) = (Arc::clone(self), id.to_owned());
        let follower = std::thread::Builder::new()
            .name("runner-transcript".to_owned())
            .spawn(move || {
                sessions.read_source(&owned, None);
                for next in woken {
                    match next {
                        Wake::Changed => sessions.read_source(&owned, None),
                        Wake::Lost(reason) => sessions.read_source(&owned, Some(&reason)),
                        Wake::Stop => break,
                    }
                }
                drop(watcher);
            })
            .map_err(|error| RunnerError::refused("transcript_worker_failed", error.to_string()))?;
        session.follower = Some(wake);
        drop(follower);
        Ok(())
    }

    /// Read session `id`'s stream from its saved cursor to its last whole
    /// line, and keep what it yields with the new cursor as one unit.
    pub(crate) fn read_source(&self, id: &str, lost: Option<&str>) {
        loop {
            let table = self.lock();
            let Some(session) = table.sessions.get(id) else {
                return;
            };
            let Some(tracking) = session.guard.tracking.clone() else {
                return;
            };
            let Some(mut source) = table.feed.source(id).cloned() else {
                return;
            };
            let evidence = accounts(session, &tracking);
            let current = evidence.current.map(str::to_owned);
            let moves = evidence.moves.to_vec();
            drop(table);
            let reading = Reading {
                runner: self.state.runner(),
                session: id,
                tracking: &tracking,
                accounts: Accounts {
                    current: current.as_deref(),
                    moves: &moves,
                    declared: tracking.account.as_deref(),
                },
                now: now_ms(),
            };
            let mut bodies = Vec::new();
            if let Some(reason) = &lost {
                bodies.push(Body::Coverage(Coverage::of(
                "coverage_incomplete",
                &source,
                Some(source.offset),
                format!("change notices were lost ({reason}): the stream is read again from its saved cursor"),
            )));
            }
            let before = source.clone();
            let more = read_lines(&reading, &mut source, &mut bodies);
            if bodies.is_empty() && source == before {
                return;
            }
            let mut table = self.lock();
            if table.feed.source(id) != Some(&before) {
                drop(table);
                continue;
            }
            if source.generation != before.generation {
                if let Err(error) = crate::collector::status::flush_status(&mut table, id) {
                    crate::error::said(&format!(
                        "session {id}: source_status_record_failed: {error}"
                    ));
                    return;
                }
                if let Some(committed) = table.feed.source(id) {
                    source.snapshot.clone_from(&committed.snapshot);
                    source
                        .snapshot_account
                        .clone_from(&committed.snapshot_account);
                    source.reported_cost_micros = committed.reported_cost_micros;
                }
            }
            let commit = Commit {
                source: Some(source),
                attempt: None,
            };
            let leader = window_limit(&mut table, id, &bodies);
            let appended = table.feed.append(id, now_ms(), bodies, commit);
            drop(table);
            if let Err(error) = appended.and_then(|_| self.writer.barrier()) {
                crate::error::said(&format!(
                    "session {id}: coverage_incomplete: what its stream yielded was not kept, and is read again from the saved cursor: {error}"
                ));
                return;
            }
            if let Some(leader) = leader {
                if let Err(error) = crate::pty::end(&leader) {
                    crate::error::said(&format!("session {id}: rotation_signal_failed: {error}"));
                }
            }
            self.wake();
            if !more {
                return;
            }
        }
    }
}

/// Stop the follower `follower` of session `id`.
fn stop_follower(id: &str, follower: &mpsc::Sender<Wake>) {
    if follower.send(Wake::Stop).is_err() {
        crate::error::said(&format!(
            "session {id}: its stream follower had already ended"
        ));
    }
}

/// Keep `bodies` for session `id` as one unit, with `source` when given.
pub(crate) fn append(
    table: &mut Table,
    id: &str,
    bodies: Vec<Body>,
    source: Option<SourceState>,
) -> Result<(), RunnerError> {
    let commit = Commit {
        source,
        attempt: None,
    };
    if let Err(error) = table.feed.append(id, now_ms(), bodies, commit) {
        let gap = table.gaps.entry(id.to_owned()).or_default();
        gap.lost += 1;
        gap.since.get_or_insert(now_ms());
        gap.words = format!("coverage_incomplete: {error}");
        crate::error::said(&format!("session {id}: coverage_incomplete: {error}"));
        return Err(error);
    }
    Ok(())
}

/// The rotation evidence of `session`.
pub(crate) fn accounts<'a>(session: &'a Session, tracking: &'a Tracking) -> Accounts<'a> {
    Accounts {
        current: session
            .rotation
            .as_ref()
            .map(crate::rotation::RotationState::account),
        moves: session
            .rotation
            .as_ref()
            .map(crate::rotation::RotationState::moves)
            .unwrap_or_default(),
        declared: tracking.account.as_deref(),
    }
}

/// Read `source` from its offset to its last whole line into `bodies`.
fn read_lines(reading: &Reading<'_>, source: &mut SourceState, bodies: &mut Vec<Body>) -> bool {
    let opened = std::fs::File::open(&source.path).and_then(|file| {
        let metadata = file.metadata()?;
        Ok((file, metadata))
    });
    let (mut file, metadata) = match opened {
        Ok(opened) => opened,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return false,
        Err(error) => {
            bodies.push(Body::Coverage(Coverage::of(
                "source_refused",
                source,
                Some(source.offset),
                format!("the stream cannot be read: {error}"),
            )));
            return false;
        }
    };
    let identity = format!("{}:{}", metadata.dev(), metadata.ino());
    let replaced = source
        .identity
        .as_ref()
        .is_some_and(|held| *held != identity);
    if replaced || metadata.len() < source.offset {
        let words = if replaced {
            "the stream's file was replaced"
        } else {
            "the stream's file was truncated"
        };
        source.generation += 1;
        source.offset = 0;
        source.pending = None;
        source.totals = None;
        bodies.push(Body::Coverage(Coverage::of(
            "source_generation",
            source,
            Some(0),
            format!(
                "{words}: generation {} is read from its start",
                source.generation
            ),
        )));
    }
    source.identity = Some(identity);
    if let Err(error) = if source.offset == 0 {
        Ok(0)
    } else {
        file.seek(SeekFrom::Start(source.offset))
    } {
        bodies.push(Body::Coverage(Coverage::of(
            "source_refused",
            source,
            Some(source.offset),
            format!("the stream cannot be read: {error}"),
        )));
        return false;
    }
    let mut reader = BufReader::new(file);
    let beginning = source.offset;
    let mut at = source.offset;
    loop {
        let mut line = Vec::new();
        let read = reader.by_ref().take(1_048_577).read_until(b'\n', &mut line);
        if let Err(error) = read {
            bodies.push(Body::Coverage(Coverage::of(
                "source_refused",
                source,
                Some(at),
                error.to_string(),
            )));
            return false;
        }
        if line.len() > 1_048_576 {
            bodies.push(Body::Coverage(Coverage::of(
                "record_too_large",
                source,
                Some(at),
                "a transcript record exceeds 1048576 bytes".to_owned(),
            )));
            return false;
        }
        if line.last() != Some(&b'\n') {
            return false;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(record) => bodies.extend(match reading.tracking.harness {
                Harness::ClaudeCode => reading.claude(source, at, &record),
                Harness::Codex => reading.codex(source, at, &record),
            }),
            Err(error) => bodies.push(Body::Coverage(Coverage::of(
                "record_unreadable",
                source,
                Some(at),
                format!("the record at byte {at} does not read: {error}"),
            ))),
        }
        at += line.len() as u64;
        source.offset = at;
        if at - beginning >= 1_048_576 {
            return true;
        }
    }
}

/// The directory a session is bound to: `directory` resolved, or as given
/// when it cannot be.
pub(crate) fn bound_directory(directory: &str) -> String {
    let given = if directory.is_empty() { "." } else { directory };
    std::fs::canonicalize(given).map_or_else(
        |error| {
            crate::error::said(&format!(
                "{given} does not resolve, and is kept as given: {error}"
            ));
            given.to_owned()
        },
        |path| path.display().to_string(),
    )
}

/// The executable `launch` runs, found as its environment's `PATH` finds
/// it, and the version it reports, refused `tracking_contract_unsupported`
/// unless it is the version `tracking` declares.
pub(crate) fn launched(
    launch: &Launch,
    tracking: &Tracking,
) -> Result<(String, String), RunnerError> {
    let unsupported = |words: String| RunnerError::refused("tracking_contract_unsupported", words);
    let program = Path::new(&launch.program);
    let found = if launch.program.contains('/') {
        Some(program.to_owned())
    } else {
        let path = launch
            .environment
            .get("PATH")
            .cloned()
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default();
        std::env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    };
    let executable = found
        .and_then(|found| std::fs::canonicalize(found).ok())
        .ok_or_else(|| {
            unsupported(format!(
                "{} is not found to ask its version",
                launch.program
            ))
        })?;
    let output = std::process::Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|error| {
            unsupported(format!(
                "{} did not say its version: {error}",
                executable.display()
            ))
        })?;
    let said = String::from_utf8_lossy(&output.stdout);
    let version = version_in(&said)
        .ok_or_else(|| unsupported(format!("{} names no version", executable.display())))?;
    crate::tracking::measured(&tracking.adapter, &version)?;
    if version != tracking.version {
        return Err(unsupported(format!(
            "{} is version {version}, and the profile declares {}",
            executable.display(),
            tracking.version
        )));
    }
    Ok((executable.display().to_string(), version))
}

/// Say, in the feed, the executable and version launched for session `id`.
pub(crate) fn tracking_started(
    table: &mut Table,
    id: &str,
    executable: &str,
    version: &str,
) -> Result<(), RunnerError> {
    let tracking = table
        .sessions
        .get(id)
        .and_then(|session| session.guard.tracking.as_ref());
    let adapter = tracking.map(|tracking| tracking.adapter.clone());
    let coverage = Coverage {
        state: "tracking_started".to_owned(),
        source: None,
        generation: 0,
        offset: None,
        words: format!("launched {executable}, which says it is version {version}"),
        executable: Some(executable.to_owned()),
        harness_version: Some(version.to_owned()),
        adapter,
    };
    append(table, id, vec![Body::Coverage(coverage)], None)
}

pub(crate) fn window_limit(table: &mut Table, id: &str, bodies: &[Body]) -> Option<Leader> {
    let session = table.sessions.get_mut(id)?;
    if session.ending {
        return None;
    }
    let rotation = session.rotation.as_mut()?;
    if rotation.tripped() {
        return None;
    }
    let reached = bodies.iter().any(|body| match body {
        Body::Usage(record) => {
            record.account.as_deref() == Some(rotation.account())
                && rotation.windows_in(&record.figures.plan_windows, now_ms())
        }
        _ => false,
    });
    if reached {
        rotation.trip();
        if session.guard.leader.is_none() {
            crate::error::said("rotation_signal_failed: the process's leader is unproved");
        }
        return session.guard.leader.clone();
    }
    None
}
