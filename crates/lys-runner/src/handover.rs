//! Handing a runner's running sessions to a new build without ending one.
//!
//! The runner never gives its sessions to another process: it replaces its
//! own program. So every session's process stays this process's child, its
//! exit is still waited on with its real status, the pid file and the exit
//! lock the install watches stay as they are, and the socket is never
//! unbound: a caller that connects meanwhile is queued and answered by the
//! new build.
//!
//! In order, the runner:
//!
//! 1. asks the new build which handover record it reads (`lys runner
//!    handover-format`) and refuses `handover_format_unsupported` on any
//!    other answer, before anything changes;
//! 2. refuses new starts, new input and new stops (`runner_handing_over`),
//!    and waits for every start, every exit being recorded and every input
//!    already accepted to finish: each wait ends when the thing it waits for
//!    happens, or when the caller who asked closes its request;
//! 3. stops each session's output at a byte boundary: what the terminal
//!    holds beyond it stays in the terminal, whose writer simply waits;
//! 4. keeps the handover record (this module's [`Manifest`]) in one durable
//!    write, with each session's held state and scrollback and the number
//!    of every descriptor kept across the replacement;
//! 5. answers `handing_over`, and replaces its program with the new build's
//!    `lys runner take-over`.
//!
//! The new build takes every session in the record up, removes the record
//! and says it listens. A session it cannot take up is ended, by name, with
//! its real exit status. A replacement that fails leaves the old build
//! running every session as before, and says so. A new build that dies
//! leaves the record behind; the next runner reports each of its sessions
//! ended `handover_failed`, naming the build.

use std::collections::{BTreeMap, BTreeSet};
use std::os::fd::{AsFd, BorrowedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::error::RunnerError;
use crate::judge::Policy;
use crate::peer::Leader;
use crate::protocol::{Launch, Stopped};
use crate::refusal_log::AuditGap;
use crate::rotation::RotationState;
use crate::tracking::Tracking;

pub use crate::fd_own::{receive as receive_descriptor, send as send_descriptor};

/// The handover record this build reads and writes.
pub const FORMAT: &str = "lys-runner-handover/v1";

/// The record's name in the runner's state directory.
pub const FILE: &str = "handover.json";

/// The record a runner keeps for the build taking its sessions up.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// [`FORMAT`].
    pub format: String,
    /// The build taking the sessions up.
    pub binary: String,
    /// The socket the runner listens on.
    pub socket: PathBuf,
    /// The runner's state directory.
    pub state: PathBuf,
    /// The server's public key, lowercase hex.
    pub server_key: String,
    /// Each session's scrollback, in bytes.
    pub scrollback: usize,
    /// The listening socket's descriptor number.
    pub listener: RawFd,
    /// The state directory lock's descriptor number.
    pub state_lock: RawFd,
    /// Every running session.
    pub sessions: Vec<Handed>,
    /// Where each session's audit has a gap.
    #[serde(default)]
    pub gaps: BTreeMap<String, AuditGap>,
}

/// One running session as it is handed over.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handed {
    /// The session.
    pub session: String,
    /// Its generation: each account move starts a new one.
    pub generation: u64,
    /// Its leader, as spawned.
    pub leader: Leader,
    /// Its terminal's descriptor number.
    pub terminal: RawFd,
    /// The name of its terminal's other end, checked against the descriptor.
    pub tty: String,
    /// When it started, in milliseconds since the Unix epoch.
    pub started_at: u64,
    /// Width in columns.
    pub columns: u16,
    /// Height in rows.
    pub rows: u16,
    /// What it was started with.
    pub launch: Option<Launch>,
    /// Its place in its rotation.
    pub rotation: Option<RotationState>,
    /// Its tool-boundary policy.
    pub policy: Option<Policy>,
    /// How its harness is tracked.
    pub tracking: Option<Tracking>,
    /// Its bound working directory.
    pub cwd: String,
    /// Whether it is between turns.
    pub idle: bool,
    /// Whether it was being ended.
    pub ending: bool,
    /// Who stopped it, when a stop of everything is ending it.
    pub stopped: Option<Stopped>,
    /// Its scrollback, lowercase hex.
    pub output: String,
    /// The cursor of the scrollback's first byte.
    pub oldest: u64,
    /// Whether its declared usage-limit words were seen.
    pub tripped: bool,
}

/// Where a runner listens and what it was started with, kept so its
/// handover record names them and its listener stays open across it.
#[derive(Debug)]
pub(crate) struct Holding {
    /// A second handle on the listening socket.
    pub(crate) listener: std::os::unix::net::UnixListener,
    /// The socket's path.
    pub(crate) socket: PathBuf,
    /// The state directory.
    pub(crate) state: PathBuf,
    /// The server's public key.
    pub(crate) server_key: [u8; 32],
}

/// The record's path in `state`.
pub fn path(state: &Path) -> PathBuf {
    state.join(FILE)
}

impl Manifest {
    /// The record at `path`, refused unless it is in [`FORMAT`].
    pub fn read(path: &Path) -> Result<Self, RunnerError> {
        let bytes = std::fs::read(path).map_err(|error| {
            RunnerError::refused(
                "handover_record_unreadable",
                format!("{}: {error}", path.display()),
            )
        })?;
        let manifest: Self = serde_json::from_slice(&bytes).map_err(|error| {
            RunnerError::refused(
                "handover_record_unreadable",
                format!("{}: {error}", path.display()),
            )
        })?;
        if manifest.format != FORMAT {
            return Err(RunnerError::refused(
                "handover_format_unsupported",
                format!("the record is in {}, and this build reads {FORMAT}", manifest.format),
            ));
        }
        Ok(manifest)
    }

    /// Keep the record at `path` in one durable write.
    pub fn write(&self, path: &Path) -> Result<(), RunnerError> {
        let bytes = serde_json::to_vec(self).map_err(|error| {
            RunnerError::refused("handover_record_unwritten", error.to_string())
        })?;
        crate::state::replace(path, &bytes).map_err(|error| {
            RunnerError::refused(
                "handover_record_unwritten",
                format!("{}: {error}", path.display()),
            )
        })
    }

    /// A record an earlier handover left behind in `state`, taken away: its
    /// build died before it took the sessions up. Each session it names is
    /// answered with the words its end is to carry.
    pub fn unfinished(state: &Path) -> Result<BTreeMap<String, String>, RunnerError> {
        let path = path(state);
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let words = match Self::read(&path) {
            Ok(manifest) => manifest
                .sessions
                .into_iter()
                .map(|handed| {
                    let words = format!(
                        "the handover to {} did not complete: that build ended before it took the session up",
                        manifest.binary
                    );
                    (handed.session, words)
                })
                .collect(),
            Err(error) => {
                crate::error::said(&format!("an unfinished handover record: {error}"));
                BTreeMap::new()
            }
        };
        std::fs::remove_file(&path).map_err(|error| {
            RunnerError::refused(
                "handover_record_unremoved",
                format!("{}: {error}", path.display()),
            )
        })?;
        Ok(words)
    }
}

/// Ask `binary` which handover record it reads, refused
/// `handover_format_unsupported` unless it answers [`FORMAT`].
pub fn check_format(
    binary: &str,
    environment: &BTreeMap<String, String>,
) -> Result<(), RunnerError> {
    if !Path::new(binary).is_absolute() {
        return Err(RunnerError::refused(
            "handover_format_unsupported",
            format!("{binary} is not an absolute path"),
        ));
    }
    let output = std::process::Command::new(binary)
        .env_clear()
        .envs(environment)
        .args(["runner", "handover-format"])
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| {
            RunnerError::refused(
                "handover_format_unsupported",
                format!("{binary} could not be asked its handover format: {error}"),
            )
        })?;
    let said = String::from_utf8_lossy(&output.stdout);
    if output.status.success() && said.trim() == FORMAT {
        return Ok(());
    }
    Err(RunnerError::refused(
        "handover_format_unsupported",
        format!(
            "{binary} answered {:?} ({}), and this runner writes {FORMAT}",
            said.trim(),
            output.status
        ),
    ))
}

#[derive(Default)]
struct Pumps {
    asked: bool,
    running: BTreeSet<(String, u64)>,
    parked: BTreeSet<(String, u64)>,
}

/// Every session's output reader, stopped at a byte boundary on request.
///
/// Each reader waits on its terminal and on this wake together. Asked, the
/// wake is made readable, every reader stops after keeping what it read,
/// and the ask is answered once every running reader has stopped.
pub(crate) struct Quiesce {
    wake: UnixStream,
    woken: UnixStream,
    pumps: Mutex<Pumps>,
    changed: Condvar,
}

fn poisoned(error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("handover_state_poisoned", error.to_string())
}

impl Quiesce {
    pub(crate) fn new() -> Result<Self, RunnerError> {
        let (wake, woken) = UnixStream::pair().map_err(|error| {
            RunnerError::refused("handover_wake_unavailable", error.to_string())
        })?;
        Ok(Self {
            wake,
            woken,
            pumps: Mutex::new(Pumps::default()),
            changed: Condvar::new(),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Pumps>, RunnerError> {
        self.pumps.lock().map_err(poisoned)
    }

    /// What every reader waits on beside its terminal.
    pub(crate) fn fd(&self) -> BorrowedFd<'_> {
        self.woken.as_fd()
    }

    /// A reader of generation `generation` of `id` begins.
    pub(crate) fn enter(&self, id: &str, generation: u64) {
        match self.lock() {
            Ok(mut pumps) => {
                pumps.running.insert((id.to_owned(), generation));
            }
            Err(error) => crate::error::said(&error.to_string()),
        }
    }

    /// A reader ends.
    pub(crate) fn leave(&self, id: &str, generation: u64) {
        match self.lock() {
            Ok(mut pumps) => {
                let key = (id.to_owned(), generation);
                pumps.running.remove(&key);
                pumps.parked.remove(&key);
                self.changed.notify_all();
            }
            Err(error) => crate::error::said(&error.to_string()),
        }
    }

    /// Whether readers are asked to stop.
    pub(crate) fn asked(&self) -> bool {
        self.lock().is_ok_and(|pumps| pumps.asked)
    }

    /// Stop this reader until the ask is withdrawn; the replacement of the
    /// program ends it here instead.
    pub(crate) fn park(&self, id: &str, generation: u64) -> Result<(), RunnerError> {
        let key = (id.to_owned(), generation);
        let mut pumps = self.lock()?;
        pumps.parked.insert(key.clone());
        self.changed.notify_all();
        while pumps.asked {
            pumps = self.changed.wait(pumps).map_err(poisoned)?;
        }
        pumps.parked.remove(&key);
        Ok(())
    }

    /// Ask every reader to stop, answering once each running one has, or
    /// refused `caller_left` when `left` is set first.
    pub(crate) fn ask(&self, left: &AtomicBool) -> Result<(), RunnerError> {
        let mut pumps = self.lock()?;
        if !pumps.asked {
            pumps.asked = true;
            std::io::Write::write_all(&mut &self.wake, &[1]).map_err(|error| {
                RunnerError::refused("handover_wake_unavailable", error.to_string())
            })?;
        }
        while !pumps.running.is_subset(&pumps.parked) {
            if left.load(Ordering::SeqCst) {
                return Err(RunnerError::refused(
                    "caller_left",
                    "the caller closed the handover before every session's output stopped",
                ));
            }
            pumps = self.changed.wait(pumps).map_err(poisoned)?;
        }
        Ok(())
    }

    /// Withdraw the ask: every stopped reader goes on from where it stopped.
    pub(crate) fn resume(&self) -> Result<(), RunnerError> {
        let mut pumps = self.lock()?;
        if pumps.asked {
            let mut byte = [0_u8];
            std::io::Read::read_exact(&mut &self.woken, &mut byte).map_err(|error| {
                RunnerError::refused("handover_wake_unavailable", error.to_string())
            })?;
            pumps.asked = false;
        }
        self.changed.notify_all();
        Ok(())
    }

    /// Wake an ask, so it sees its caller left.
    pub(crate) fn wake(&self) {
        if let Ok(pumps) = self.lock() {
            self.changed.notify_all();
            drop(pumps);
        }
    }
}
