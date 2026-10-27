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
use std::sync::{Arc, Condvar, Mutex, Weak, mpsc};

use portable_pty::MasterPty;

use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::input::Input;
use crate::judge::Policy;
use crate::operations::Operations;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Launch, SessionView, Stopped};
use crate::refusals::Desk;
use crate::rotation::RotationState;
use crate::state::{Kept, KeptSession, StateFile};
use crate::tracking::Tracking;
use crate::tracking_store::Feed;

mod control;

#[cfg(test)]
#[path = "../tests/injection/cases.rs"]
mod injection_tests;

mod peer_view;

mod launch;
mod lifecycle;
mod locks;
pub(crate) mod output;
mod owned;
mod restart;
mod stop;
mod trust_dialog;
mod workers;

#[cfg(test)]
#[path = "../tests/session_start/cases.rs"]
mod start_tests;

#[cfg(test)]
#[path = "../tests/session_poison/cases.rs"]
mod poison_tests;

#[cfg(test)]
#[path = "../tests/session_notify.rs"]
mod notify_tests;

#[cfg(test)]
type SpawnProbe = Box<dyn FnOnce() + Send>;

pub use crate::refusal_log::AuditGap;
pub use lifecycle::Collected;
pub(crate) use lifecycle::{Wake, accounts, append, transcript_parent, window_limit};
use stop::left_behind;
pub use stop::{Ask, StoppedEverything};

/// The runner's own name, as `status` answers it.
pub const RUNNER: &str = "lys-runner";

mod clock;
pub use clock::now_ms;

/// The live ends of a running process.
pub(crate) struct Live {
    pub(crate) writer: Input,
    master: Option<Box<dyn MasterPty + Send>>,
    pub(crate) control: Option<crate::harness_control::events::Runtime>,
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
        if let Some(pipe) = &self.writer.seat {
            return pipe.close();
        }
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

mod guard;
pub use guard::Guard;

/// One session.
pub(crate) struct Session {
    pub(crate) started_at: u64,
    pub(crate) pid: Option<u32>,
    pub(crate) leader_start: Option<Leader>,
    columns: u16,
    rows: u16,
    pub(crate) output: Arc<output::OutputHandle>,
    pub(crate) ended: Option<Ended>,
    pub(crate) live: Option<Live>,
    pub(crate) generation: u64,
    pub(crate) launch: Option<Launch>,
    pub(crate) managed: Option<crate::harness_control::Settings>,
    pub(crate) rotation: Option<RotationState>,
    pub(crate) ending: bool,
    pub(crate) guard: Guard,
    pub(crate) follower: Option<mpsc::Sender<Wake>>,
    pub(crate) pending_status: Option<crate::collector::status::PendingStatus>,
    /// The account and plan windows last kept from the status line of a
    /// session tracked neither natively nor through the proxy.
    pub(crate) plan_windows: Option<(String, Vec<crate::tracking_budget::PlanWindow>)>,
    pub(crate) stopped: Option<Stopped>,
    /// Its managed frames rendered as lines, bounded like its output.
    pub(crate) attach: crate::attach::Ring,
    /// When its harness last signalled through a frame, hook or status line.
    pub(crate) last_signal: Option<u64>,
    /// The threads that act for it and hold the runner while they run,
    /// joined by whoever answers its end (see `workers`).
    pub(crate) workers: Vec<std::thread::JoinHandle<()>>,
}

impl Session {
    fn view(&self, id: &str) -> Result<SessionView, RunnerError> {
        let output = self.output.lock()?;
        Ok(SessionView {
            seat_program: self
                .launch
                .as_ref()
                .and_then(|launch| {
                    launch
                        .environment
                        .get(crate::harness_control::seat_program::PROOF_VARIABLE)
                })
                .map(|proof| serde_json::from_str(proof))
                .transpose()
                .map_err(|error| {
                    RunnerError::refused("seat_program_record_invalid", error.to_string())
                })?,
            qualification: self
                .live
                .as_ref()
                .and_then(|live| live.control.as_ref())
                .map(|control| {
                    crate::harness_control::process::version_mark(
                        &control.controller.binding.adapter,
                        &control.controller.binding.harness_version,
                    )
                })
                .transpose()?
                .flatten(),
            session: id.to_owned(),
            control: self
                .live
                .as_ref()
                .and_then(|live| live.control.as_ref())
                .map(|control| control.controller.control_status()),
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
                .map(|rotation| {
                    #[cfg(test)]
                    {
                        control_history::READS
                            .with(|reads| reads.set(reads.get() + rotation.moves().len()));
                    }
                    rotation.moves().to_vec()
                })
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
    pub(crate) responsible: BTreeMap<String, String>,
    starting: BTreeSet<String>,
    stopping: bool,
    pub(crate) feed: Feed,
    pub(crate) desk: Desk,
    pub(crate) gaps: BTreeMap<String, AuditGap>,
    pub(crate) operations: Operations,
}

/// The sessions a runner holds, and what wakes those waiting on them.
pub struct Sessions {
    upgrading: std::sync::atomic::AtomicBool,
    table: Mutex<Table>,
    changed: Condvar,
    state: StateFile,
    pub(crate) state_dir: PathBuf,
    /// The proxy's `usage` directory on this machine, once said.
    proxy_usage: std::sync::OnceLock<PathBuf>,
    /// The socket this runner serves on, once bound: where a proxied run's
    /// status line hands what its harness reported.
    serving: std::sync::OnceLock<PathBuf>,
    /// The server's public key, once the runner opened its socket; owners
    /// this runner starts verify acts with it (AGENTS-004 R1).
    pub(crate) server_key: std::sync::OnceLock<[u8; 32]>,
    /// This runner's own owner state, when it is a seat owner.
    pub(crate) owner_state: std::sync::OnceLock<Arc<crate::seat_owner::process::OwnerState>>,
    /// The seats this runner started owners for, indexed by session.
    pub(crate) owned: Mutex<BTreeMap<String, crate::seat_owner::sessions::OwnedSeat>>,
    /// Sessions whose owner is being started and is not yet ready: reserved
    /// under the `owned` lock before the spawn, so a second start for the
    /// same seat is refused by name and never spawns a second owner.
    pub(crate) owner_starts: Mutex<BTreeSet<String>>,
    /// The program owners run as, when said; this runner's own executable
    /// otherwise.
    pub(crate) owner_program: std::sync::OnceLock<PathBuf>,
    /// Owners on record the kernel did not confirm at open: unknown
    /// authority, kept and named, never restarted here.
    pub(crate) unreachable: Mutex<Vec<crate::seat_owner::recovery::Found>>,
    pub(crate) scrollback: usize,
    pub(crate) writer: crate::durable::Writer,
    #[cfg(test)]
    spawn_probe: Mutex<Option<SpawnProbe>>,
    #[cfg(test)]
    worker_probes: workers::Probes,
}

pub(super) struct Starting {
    sessions: Arc<Sessions>,
    id: String,
}

impl Drop for Starting {
    fn drop(&mut self) {
        if let Some(mut table) = self.sessions.lock_logged() {
            table.starting.remove(&self.id);
            drop(table);
            self.sessions.wake();
        }
    }
}

fn table_poisoned(error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("session_table_poisoned", error.to_string())
}

pub(crate) fn unknown(id: &str) -> RunnerError {
    RunnerError::refused("session_unknown", format!("no session {id} is held"))
}

fn notify_named(launch: &mut Launch) -> Result<(), RunnerError> {
    let placeholder =
        r#"notify=["$LYS_PROGRAM", "runner", "notify", "--socket", "$LYS_RUNNER_SOCKET"]"#;
    if launch
        .arguments
        .iter()
        .any(|argument| argument == placeholder)
    {
        let program = launch.environment.get("LYS_PROGRAM").ok_or_else(|| {
            RunnerError::refused("notify_unnamed", "the runner did not publish LYS_PROGRAM")
        })?;
        let socket = launch.environment.get("LYS_RUNNER_SOCKET").ok_or_else(|| {
            RunnerError::refused(
                "notify_unnamed",
                "the runner did not publish LYS_RUNNER_SOCKET",
            )
        })?;
        let notify = format!(
            "notify={}",
            serde_json::to_string(&[
                program.as_str(),
                "runner",
                "notify",
                "--socket",
                socket.as_str()
            ])
            .map_err(|error| RunnerError::refused("notify_unnamed", error.to_string()))?
        );
        for argument in &mut launch.arguments {
            if argument == placeholder {
                argument.clone_from(&notify);
            }
        }
    }
    Ok(())
}

include!("session/opening.rs");

#[cfg(test)]
pub(crate) mod control_history {
    thread_local! {
        pub(crate) static READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
}

pub mod kept_on_stop;
/// Retained launches for a controlled runner replacement.
pub mod upgrade;
