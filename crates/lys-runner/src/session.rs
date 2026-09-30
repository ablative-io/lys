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
//! recorded process group is made gone: anything left in it, a process that
//! ignored the hang-up, is ended then, and the end names that signal.

use std::collections::BTreeMap;
use std::path::Path;
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
use crate::scrollback::Scrollback;
use crate::state::{Kept, KeptSession, StateFile};
use crate::tracking::Tracking;
use crate::tracking_store::Feed;

mod lifecycle;

pub use crate::refusal_log::AuditGap;
pub use lifecycle::Collected;
pub(crate) use lifecycle::{Wake, accounts, append};

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
}

#[cfg(test)]
#[path = "../tests/input_no_screen/shared_lock.rs"]
mod input_no_screen;

impl Live {
    /// End the process and everything it started, naming a failure in the
    /// runner's log: the exit, when it comes, is what answers.
    pub(crate) fn end(&self, id: &str) {
        if let Err(error) = crate::pty::end_group(self.pid) {
            crate::error::said(&format!(
                "session {id}: the process was already ending: {error}"
            ));
        }
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
    columns: u16,
    rows: u16,
    scrollback: Scrollback,
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
    /// The session's scrollback.
    pub(crate) fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// The session's end, once it has ended.
    pub(crate) fn ended(&self) -> Option<Ended> {
        self.ended.clone()
    }

    fn view(&self, id: &str) -> SessionView {
        SessionView {
            session: id.to_owned(),
            pid: self.pid,
            started_at: self.started_at,
            columns: self.columns,
            rows: self.rows,
            oldest: self.scrollback.oldest(),
            cursor: self.scrollback.end(),
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
        }
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
    scrollback: usize,
}

pub(crate) fn unknown(id: &str) -> RunnerError {
    RunnerError::refused("session_unknown", format!("no session {id} is held"))
}

/// End what is left of session `id`'s process group `pid`, which the last
/// run recorded and never saw end, answering the signal that ended it when
/// anything was left; a group that cannot be ended is named in the log.
fn left_behind(id: &str, pid: u32) -> Option<String> {
    match crate::pty::end_left_group(pid) {
        Ok(crate::pty::Left::Gone) => None,
        Ok(crate::pty::Left::Ended) => {
            crate::error::said(&format!(
                "session {id}: process group {pid} outlived the last runner and was ended"
            ));
            Some("SIGKILL".to_owned())
        }
        Err(error) => {
            crate::error::said(&format!("session {id}: {error}"));
            None
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
        let state = StateFile::open(state_dir)?;
        let found_at = now_ms();
        let mut table = Table {
            owner: Weak::new(),
            sessions: BTreeMap::new(),
            stopping: false,
            feed: Feed::open(state_dir)?,
            desk: Desk::default(),
            gaps: BTreeMap::new(),
            operations: Operations::open(state_dir)?,
        };
        for kept in state.read()?.sessions {
            let ended = match kept.ended {
                Some(ended) => ended,
                None => Ended {
                    how: EndedHow::EndedByRunnerRestart,
                    at: found_at,
                    status: None,
                    signal: kept.pid.and_then(|pid| left_behind(&kept.session, pid)),
                },
            };
            table.sessions.insert(
                kept.session,
                Session {
                    started_at: kept.started_at,
                    pid: kept.pid,
                    columns: kept.columns,
                    rows: kept.rows,
                    scrollback: Scrollback::new(scrollback),
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
        let sessions = Arc::new_cyclic(|owner| {
            table.owner = owner.clone();
            Self {
                table: Mutex::new(table),
                changed: Condvar::new(),
                state,
                scrollback,
            }
        });
        let table = sessions.lock();
        sessions.persist(&table)?;
        drop(table);
        Ok(sessions)
    }

    /// The runner's own id, which every request it answers names.
    pub fn runner(&self) -> &str {
        self.state.runner()
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Table> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn wait<'a>(&self, table: MutexGuard<'a, Table>) -> MutexGuard<'a, Table> {
        self.changed
            .wait(table)
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Wake everything waiting on the table: a caller left, or a thing changed.
    pub fn wake(&self) {
        self.changed.notify_all();
    }

    fn persist(&self, table: &Table) -> Result<(), RunnerError> {
        let sessions = table
            .sessions
            .iter()
            .map(|(id, session)| KeptSession {
                session: id.clone(),
                pid: session.pid,
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
        launch: Launch,
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
        if table.sessions.contains_key(&launch.session) {
            return Err(RunnerError::refused(
                "session_exists",
                format!("session {} is already held", launch.session),
            ));
        }
        let id = launch.session.clone();
        let cwd = lifecycle::bound_directory(&launch.directory);
        let mut session = Session {
            started_at: now_ms(),
            pid: None,
            columns: launch.columns,
            rows: launch.rows,
            scrollback: Scrollback::new(self.scrollback),
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
        let pid = self.run(&id, &mut session, false)?;
        let started_at = session.started_at;
        table.sessions.insert(id.clone(), session);
        self.persist(&table)?;
        if let Some((executable, version)) = launched {
            lifecycle::tracking_started(&mut table, &id, &executable, &version);
        }
        drop(table);
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

    /// Run `check` on session `id` each time the table changes, until it
    /// answers, the caller leaves, or the runner stops.
    pub(crate) fn until<T>(
        &self,
        id: &str,
        left: &AtomicBool,
        mut check: impl FnMut(&mut Session, &str) -> Option<Result<T, RunnerError>>,
    ) -> Result<T, RunnerError> {
        let mut table = self.lock();
        loop {
            let stopping = table.stopping;
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            if let Some(answer) = check(session, id) {
                return answer;
            }
            if left.load(Ordering::SeqCst) {
                return Err(RunnerError::refused(
                    "caller_left",
                    "the caller closed the request",
                ));
            }
            if stopping {
                return Err(RunnerError::refused(
                    "runner_stopping",
                    "the runner is stopping",
                ));
            }
            table = self.wait(table);
        }
    }

    /// End session `id`'s process and answer once its exit is seen.
    pub fn end(&self, id: &str, left: &AtomicBool) -> Result<Ended, RunnerError> {
        {
            let mut table = self.lock();
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            if session.ended.is_none() {
                session.ending = true;
                if let Some(live) = &session.live {
                    live.end(id);
                }
            }
        }
        self.until(id, left, |session, _| session.ended.clone().map(Ok))
    }

    /// What the runner holds: every session, or the one named.
    pub fn status(&self, only: Option<&str>) -> Result<StatusView, RunnerError> {
        let table = self.lock();
        let sessions = match only {
            Some(id) => vec![table.sessions.get(id).ok_or_else(|| unknown(id))?.view(id)],
            None => table
                .sessions
                .iter()
                .map(|(id, session)| session.view(id))
                .collect(),
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
            if session.ended.is_none() {
                session.ending = true;
                if let Some(live) = &session.live {
                    live.end(id);
                }
            }
        }
        while table
            .sessions
            .values()
            .any(|session| session.ended.is_none() && session.live.is_some())
        {
            table = self.wait(table);
        }
        self.persist_logged(&table);
        drop(table);
        self.wake();
    }
}
