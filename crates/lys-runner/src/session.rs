//! The sessions a runner holds, each in its own pseudo-terminal.
//!
//! Every session's output is kept in its scrollback by one thread that
//! reads its terminal, and its exit is seen by one thread that waits on its
//! process; neither runs on a clock. Every change wakes whatever is waiting
//! on the table: a read that follows, a wait for a pattern, an end. A wait
//! stops when what it waits for happens, when the session ends, or when the
//! caller leaves; nothing ends one after a time.
//!
//! A session's end is recorded when its exit is seen and its terminal has
//! given its last byte, so a process it started that still holds the
//! terminal keeps the session running until it too is gone.
//!
//! A session started with a guard holds its tool-boundary policy, how its
//! harness is tracked, and its leader's start identity, read at spawn, which
//! a peer's proof is checked against; each is held in memory for the life
//! of the session's process, which ends with the runner.
//!
//! Each session's start and end are kept in the runner's record, so a
//! runner started again reports every session it held. A session the record
//! holds with no end was lost with the runner that held it: it is reported
//! `ended_by_runner_restart`, at the instant the restart found it gone and
//! with no exit status, since none was seen. Before it is reported gone its
//! recorded process group is ended only when its leader's recorded start
//! identity still matches. A group whose ownership cannot be proved is
//! never signalled, and the reported end says why.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

use portable_pty::MasterPty;

use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::input::Input;
use crate::judge::Policy;
use crate::operations::Operations;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Key, Launch, SessionView, StatusView};
use crate::refusals::Desk;
use crate::rotation::RotationState;
use crate::state::{Kept, KeptSession, StateFile};
use crate::tracking::Tracking;
use crate::tracking_store::Feed;

mod lifecycle;
pub(crate) mod output;
mod restart;

#[cfg(test)]
#[path = "../tests/session_start/cases.rs"]
mod start_tests;

#[cfg(test)]
type SpawnProbe = Box<dyn FnOnce() + Send>;

pub use crate::refusal_log::AuditGap;
pub use lifecycle::Collected;
pub(crate) use lifecycle::{Wake, accounts, append, transcript_parent, window_limit};

/// The runner's own name, as `status` answers it.
pub const RUNNER: &str = "lys-runner";

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The live ends of a running process.
pub(crate) struct Live {
    pub(crate) writer: Input,
    master: Box<dyn MasterPty + Send>,
    pid: u32,
    leader: Option<Leader>,
}

#[cfg(test)]
#[path = "../tests/input_no_screen/shared_lock.rs"]
mod input_no_screen;

#[cfg(test)]
#[path = "../tests/durable/cases.rs"]
mod durable_tests;

impl Live {
    /// Ask the verified leader to exit so its final usage can be flushed.
    pub(crate) fn end(&self) -> Result<(), RunnerError> {
        let leader = self.leader.as_ref().ok_or_else(|| {
            RunnerError::refused(
                "leader_unproved",
                "the process's start identity was not recorded",
            )
        })?;
        crate::pty::end(leader)
    }

    /// Emergency stop retains immediate process-group termination.
    pub(crate) fn kill(&self) -> Result<(), RunnerError> {
        crate::pty::end_group(self.pid)
    }
}

/// What a session was started with beside its launch, and where its turns
/// stand.
#[derive(Debug, Clone, Default)]
pub struct Guard {
    /// The tool-boundary policy installed for the launch.
    pub policy: Option<Policy>,
    /// How its harness is tracked.
    pub tracking: Option<Tracking>,
    /// Its leader, as spawned.
    pub leader: Option<Leader>,
    /// Its bound working directory.
    pub cwd: String,
    /// Whether it is between turns, as its harness last said.
    pub idle: bool,
}

/// One session.
pub(crate) struct Session {
    pub(crate) started_at: u64,
    pid: Option<u32>,
    pub(crate) leader_start: Option<Leader>,
    columns: u16,
    rows: u16,
    pub(crate) output: Arc<output::OutputHandle>,
    pub(crate) ended: Option<Ended>,
    pub(crate) live: Option<Live>,
    generation: u64,
    pub(crate) launch: Option<Launch>,
    pub(crate) rotation: Option<RotationState>,
    pub(crate) ending: bool,
    pub(crate) guard: Guard,
    pub(crate) follower: Option<mpsc::Sender<Wake>>,
}

impl Session {
    fn view(&self, id: &str) -> Result<SessionView, RunnerError> {
        let output = self.output.lock()?;
        Ok(SessionView {
            session: id.to_owned(),
            pid: self.pid,
            started_at: self.started_at,
            columns: self.columns,
            rows: self.rows,
            oldest: output.scrollback().oldest(),
            cursor: output.scrollback().end(),
            account: self
                .rotation
                .as_ref()
                .map(|rotation| rotation.account().to_owned()),
            moves: self
                .rotation
                .as_ref()
                .map(|rotation| rotation.moves().to_vec())
                .unwrap_or_default(),
            ended: self.ended.clone(),
            policy: self
                .launch
                .as_ref()
                .and_then(|launch| launch.policy.as_deref())
                .map(Admitted::judged_under),
        })
    }

    pub(crate) fn live(&mut self, id: &str) -> Result<&mut Live, RunnerError> {
        if self.ended.is_some() {
            return Err(RunnerError::refused(
                "session_ended",
                format!("session {id} has ended"),
            ));
        }
        self.live.as_mut().ok_or_else(|| {
            RunnerError::refused(
                "session_ended",
                format!("session {id} has no running process"),
            )
        })
    }
}

/// The table every thread shares.
pub(crate) struct Table {
    pub(crate) owner: Weak<Sessions>,
    pub(crate) sessions: BTreeMap<String, Session>,
    starting: BTreeSet<String>,
    stopping: bool,
    pub(crate) feed: Feed,
    pub(crate) desk: Desk,
    pub(crate) gaps: BTreeMap<String, AuditGap>,
    pub(crate) operations: Operations,
}

/// The sessions a runner holds, and what wakes those waiting on them.
pub struct Sessions {
    table: Mutex<Table>,
    changed: Condvar,
    state: StateFile,
    state_dir: PathBuf,
    scrollback: usize,
    pub(crate) writer: crate::durable::Writer,
    #[cfg(test)]
    spawn_probe: Mutex<Option<SpawnProbe>>,
}

pub(super) struct Starting {
    sessions: Arc<Sessions>,
    id: String,
}

impl Drop for Starting {
    fn drop(&mut self) {
        self.sessions.lock().starting.remove(&self.id);
        self.sessions.wake();
    }
}

pub(crate) fn unknown(id: &str) -> RunnerError {
    RunnerError::refused("session_unknown", format!("no session {id} is held"))
}

/// Report a lost session's cleanup without treating a group number as ownership.
/// Missing ownership and members left running are named beside any signal sent.
fn left_behind(id: &str, leader: Option<&Leader>) -> (Option<String>, Option<String>) {
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

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Sessions {
    /// The sessions recorded in `state_dir`, each kept with a scrollback of
    /// `scrollback` bytes, refused `scrollback_invalid` when that keeps
    /// none. Every session the record holds without an end is reported
    /// ended by the runner's restart, now, with no exit status.
    pub fn open(state_dir: &Path, scrollback: usize) -> Result<Arc<Self>, RunnerError> {
        if scrollback == 0 {
            return Err(RunnerError::refused(
                "scrollback_invalid",
                "a session's scrollback keeps at least one byte",
            ));
        }
        let mut state = StateFile::open(state_dir)?;
        let found_at = now_ms();
        let mut table = Table {
            owner: Weak::new(),
            sessions: BTreeMap::new(),
            starting: BTreeSet::new(),
            stopping: false,
            feed: Feed::open(state_dir)?,
            desk: Desk::default(),
            gaps: BTreeMap::new(),
            operations: Operations::open(state_dir)?,
        };
        for kept in state.read()?.sessions {
            let ended = if let Some(ended) = kept.ended {
                ended
            } else {
                let (signal, reason) = left_behind(&kept.session, kept.leader_start.as_ref());
                Ended {
                    how: EndedHow::EndedByRunnerRestart,
                    at: found_at,
                    status: None,
                    signal,
                    reason,
                }
            };
            table.sessions.insert(
                kept.session,
                Session {
                    started_at: kept.started_at,
                    pid: kept.pid,
                    leader_start: kept.leader_start,
                    columns: kept.columns,
                    rows: kept.rows,
                    output: Arc::new(output::OutputHandle::new(scrollback, Some(ended.clone()))),
                    ended: Some(ended),
                    live: None,
                    generation: 0,
                    launch: None,
                    rotation: None,
                    ending: false,
                    guard: Guard::default(),
                    follower: None,
                },
            );
        }
        let state_dir = state_dir
            .canonicalize()
            .map_err(|error| RunnerError::refused("launch_config_refused", error.to_string()))?;
        let writer = crate::durable::Writer::new()?;
        state.writer(writer.clone());
        table.feed.writer(writer.clone());
        table.operations.writer(writer.clone());
        let sessions = Arc::new_cyclic(|owner| {
            table.owner = Weak::clone(owner);
            Self {
                table: Mutex::new(table),
                changed: Condvar::new(),
                state,
                state_dir,
                scrollback,
                writer,
                #[cfg(test)]
                spawn_probe: Mutex::new(None),
            }
        });
        let table = sessions.lock();
        sessions.persist(&table)?;
        drop(table);
        sessions.writer.barrier()?;
        Ok(sessions)
    }

    /// The runner's own id, which every request it answers names.
    pub fn runner(&self) -> &str {
        self.state.runner()
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Table> {
        #[cfg(test)]
        lifecycle::output_tests::table_locked();
        let mut table = self.table.lock().unwrap_or_else(PoisonError::into_inner);
        table.operations.prune(now_ms());
        table
    }

    fn wait<'a>(&self, table: MutexGuard<'a, Table>) -> MutexGuard<'a, Table> {
        self.changed
            .wait(table)
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Wake everything waiting on the table: a caller left, or a thing changed.
    pub fn wake(&self) {
        #[cfg(test)]
        lifecycle::output_tests::table_woken();
        self.changed.notify_all();
    }

    /// Wake a cancelled output request and the shared control waiters.
    pub fn wake_session(&self, id: &str) -> Result<(), RunnerError> {
        let table = self.lock();
        let output = Arc::clone(&table.sessions.get(id).ok_or_else(|| unknown(id))?.output);
        self.changed.notify_all();
        drop(table);
        output.wake()
    }

    fn persist(&self, table: &Table) -> Result<(), RunnerError> {
        let sessions = table
            .sessions
            .iter()
            .map(|(id, session)| KeptSession {
                session: id.clone(),
                pid: session.pid,
                leader_start: session.leader_start.clone(),
                started_at: session.started_at,
                columns: session.columns,
                rows: session.rows,
                ended: session.ended.clone(),
            })
            .collect();
        self.state.write(&Kept::new(sessions))
    }

    /// Record the table, naming any failure where the runner's log shows it:
    /// a thread that saw an exit cannot answer anyone.
    fn persist_logged(&self, table: &Table) {
        if let Err(error) = self.persist(table) {
            crate::error::said(&format!("the runner's record was not written: {error}"));
        }
    }

    /// Start `launch` in its own pseudo-terminal, answering its process id
    /// and when it started.
    pub fn start(self: &Arc<Self>, launch: Launch) -> Result<(u32, u64), RunnerError> {
        self.begin(launch, None, None)
    }

    /// Start `launch` as [`Sessions::start`] does, holding `policy` for its
    /// judge and tracking its harness as `tracking` says. A tracking the
    /// runner was not measured against is refused before anything runs, and
    /// so is an executable that reports another version.
    pub fn begin(
        self: &Arc<Self>,
        mut launch: Launch,
        policy: Option<Policy>,
        tracking: Option<Tracking>,
    ) -> Result<(u32, u64), RunnerError> {
        let launched = match &tracking {
            Some(tracking) => {
                tracking.checked()?;
                Some(lifecycle::launched(&launch, tracking)?)
            }
            None => None,
        };
        if !valid_id(&launch.session) {
            return Err(RunnerError::refused(
                "session_invalid",
                "a session id is one or more letters, digits, '-' or '_'",
            ));
        }
        let rotation = launch
            .rotation
            .clone()
            .map(RotationState::new)
            .transpose()?;
        let mut table = self.lock();
        if table.stopping {
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner is stopping and starts nothing",
            ));
        }
        if table.sessions.contains_key(&launch.session) || table.starting.contains(&launch.session)
        {
            return Err(RunnerError::refused(
                "session_exists",
                format!("session {} is already held", launch.session),
            ));
        }
        table.starting.insert(launch.session.clone());
        let reservation = Starting {
            sessions: Arc::clone(self),
            id: launch.session.clone(),
        };
        drop(table);
        crate::launch_config::prepare(&self.state_dir, &mut launch)?;
        let id = launch.session.clone();
        let cwd = lifecycle::bound_directory(&launch.directory);
        let mut session = Session {
            started_at: now_ms(),
            pid: None,
            leader_start: None,
            columns: launch.columns,
            rows: launch.rows,
            output: Arc::new(output::OutputHandle::new(self.scrollback, None)),
            ended: None,
            live: None,
            generation: 0,
            launch: Some(launch),
            rotation,
            ending: false,
            guard: Guard {
                policy,
                tracking,
                leader: None,
                cwd,
                idle: true,
            },
            follower: None,
        };
        let prepared = self.run(&lifecycle::plan(&session, false)?)?;
        let pending = Self::install(&mut session, prepared)?;
        let pid = session.pid.ok_or_else(|| {
            RunnerError::refused(
                "spawn_install_failed",
                "the installed child has no process id",
            )
        })?;
        let started_at = session.started_at;
        let mut table = self.lock();
        if table.stopping {
            drop(table);
            pending.cancel()?;
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner stopped during the launch",
            ));
        }
        table.sessions.insert(id.clone(), session);
        let mut recorded = self.persist(&table);
        if let Some((executable, version)) = launched {
            recorded = recorded
                .and_then(|()| lifecycle::tracking_started(&mut table, &id, &executable, &version));
        }
        drop(table);
        self.activate(&id, pending);
        drop(reservation);
        recorded?;
        self.writer.barrier()?;
        self.wake();
        Ok((pid, started_at))
    }

    /// Run `check` on the table each time it changes, until it answers, the
    /// caller leaves, or the runner stops.
    pub(crate) fn until_any<T>(
        &self,
        left: &AtomicBool,
        mut check: impl FnMut(&mut Table) -> Option<T>,
    ) -> Result<T, RunnerError> {
        let mut table = self.lock();
        loop {
            if let Some(answer) = check(&mut table) {
                return Ok(answer);
            }
            if left.load(Ordering::SeqCst) {
                return Err(RunnerError::refused(
                    "caller_left",
                    "the caller closed the request",
                ));
            }
            if table.stopping {
                return Err(RunnerError::refused(
                    "runner_stopping",
                    "the runner is stopping",
                ));
            }
            table = self.wait(table);
        }
    }

    /// Type `bytes` into session `id`.
    pub fn write(&self, id: &str, bytes: &[u8]) -> Result<(), RunnerError> {
        let writer = {
            let mut table = self.lock();
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            session.live(id)?.writer.clone()
        };
        writer.write(bytes.to_vec())
    }

    /// Type `text`, then Enter when asked.
    pub fn input(&self, id: &str, text: &str, enter: bool) -> Result<(), RunnerError> {
        let mut bytes = text.as_bytes().to_vec();
        if enter {
            bytes.extend_from_slice(Key::Enter.bytes());
        }
        self.write(id, &bytes)
    }

    /// Send `keys`, in order.
    pub fn keys(&self, id: &str, keys: &[Key]) -> Result<(), RunnerError> {
        let bytes: Vec<u8> = keys
            .iter()
            .flat_map(|key| key.bytes().iter().copied())
            .collect();
        self.write(id, &bytes)
    }

    /// Resize session `id`'s terminal.
    pub fn resize(&self, id: &str, columns: u16, rows: u16) -> Result<(), RunnerError> {
        let mut table = self.lock();
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        crate::pty::resize(&*session.live(id)?.master, columns, rows)?;
        session.columns = columns;
        session.rows = rows;
        Ok(())
    }

    /// Run `check` on session `id` each time its output changes, until it
    /// answers, the caller leaves, or the runner stops.
    pub(crate) fn until<T>(
        &self,
        id: &str,
        left: &AtomicBool,
        check: impl FnMut(&mut output::OutputState, &str) -> Option<Result<T, RunnerError>>,
    ) -> Result<T, RunnerError> {
        let output = Arc::clone(
            &self
                .lock()
                .sessions
                .get(id)
                .ok_or_else(|| unknown(id))?
                .output,
        );
        output.until(id, left, check)
    }

    /// End session `id`'s process and answer once its exit is seen.
    pub fn end(&self, id: &str, left: &AtomicBool) -> Result<Ended, RunnerError> {
        {
            let mut table = self.lock();
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            if session.ended.is_none() {
                session.ending = true;
                if let Some(live) = &session.live {
                    live.end()?;
                }
            }
        }
        let ended = self.until(id, left, |session, _| session.ended().map(Ok))?;
        self.writer.barrier()?;
        Ok(ended)
    }

    /// What the runner holds: every session, or the one named.
    pub fn status(&self, only: Option<&str>) -> Result<StatusView, RunnerError> {
        let table = self.lock();
        let sessions = match only {
            Some(id) => vec![
                table
                    .sessions
                    .get(id)
                    .ok_or_else(|| unknown(id))?
                    .view(id)?,
            ],
            None => table
                .sessions
                .iter()
                .map(|(id, session)| session.view(id))
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(StatusView {
            runner: RUNNER.to_owned(),
            protocol: crate::protocol::PROTOCOL_VERSION,
            sessions,
        })
    }

    /// End every running session and answer once each exit is seen; the
    /// runner then starts nothing more.
    pub fn stop_all(&self) {
        let mut table = self.lock();
        table.stopping = true;
        for (id, session) in &mut table.sessions {
            if let Err(error) = session.output.stop() {
                crate::error::said(&format!("session {id}: shutdown_wake_failed: {error}"));
            }
            if session.ended.is_none() {
                session.ending = true;
                if let Some(live) = &session.live {
                    if let Err(error) = live.end() {
                        crate::error::said(&format!(
                            "session {id}: shutdown_signal_failed: {error}"
                        ));
                    }
                }
            }
        }
        while !table.starting.is_empty()
            || table
                .sessions
                .values()
                .any(|session| session.ended.is_none())
        {
            table = self.wait(table);
        }
        self.persist_logged(&table);
        drop(table);
        if let Err(error) = self.writer.barrier() {
            crate::error::said(&format!("runner_shutdown_record_failed: {error}"));
        }
        self.wake();
    }
}
