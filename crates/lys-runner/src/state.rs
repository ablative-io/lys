//! The runner's record of the sessions it holds, and its own id.
//!
//! The record is one file, replaced whole and atomically at each start and
//! each end, and made durable before the change is answered. It holds each
//! session's id, process id, size, start and end; never output, never input,
//! never an account. It is not a log: a restart reads one file of the
//! sessions held, however long the runner has run.
//!
//! The directory is held by one runner at a time: an exclusive lock on its
//! `lock` file is taken when it is opened and kept while the runner lives,
//! and the kernel lets it go when the runner's process ends, however it
//! ends. A second runner opening it is refused `runner_state_held`.
//!
//! The runner's id is 16 random bytes in hex, made the first time the
//! directory is opened and kept beside the record, so every request names
//! the one runner it was signed for. Nothing about requests already
//! answered is kept: each is bound to a challenge made for its connection
//! alone (see [`crate::protocol`]).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RunnerError;
use crate::peer::Leader;
use crate::protocol::Ended;

mod legacy;

/// The record's format.
pub const FORMAT: &str = "lys-runner-sessions/v2";

/// One session as the record keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptSession {
    /// The session.
    pub session: String,
    /// Its process id, while one was known.
    pub pid: Option<u32>,
    /// The leader's identity recorded at spawn, absent when it was not proved.
    pub leader_start: Option<Leader>,
    /// When it started, in milliseconds since the Unix epoch.
    pub started_at: u64,
    /// Width in columns.
    pub columns: u16,
    /// Height in rows.
    pub rows: u16,
    /// Its end, once seen.
    pub ended: Option<Ended>,
}

/// The whole record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// [`FORMAT`].
    pub format: String,
    /// Every session held.
    pub sessions: Vec<KeptSession>,
}

impl Kept {
    /// The record of `sessions`, in this format.
    pub fn new(sessions: Vec<KeptSession>) -> Self {
        Self {
            format: FORMAT.to_owned(),
            sessions,
        }
    }
}

/// Where the record is kept.
pub struct StateFile {
    path: PathBuf,
    runner: String,
    lock: fs::File,
}

fn unavailable(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::State {
        reason: what.to_string(),
    }
}

impl StateFile {
    /// The record in `dir`, which is made, readable by its owner alone, when
    /// it does not exist.
    pub fn open(dir: &Path) -> Result<Self, RunnerError> {
        fs::create_dir_all(dir)
            .map_err(|error| unavailable(format!("making {}: {error}", dir.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
                .map_err(|error| unavailable(format!("closing {}: {error}", dir.display())))?;
        }
        let held = hold(dir)?;
        Ok(Self {
            path: dir.join("sessions.json"),
            runner: runner_id(&dir.join("runner.id"))?,
            lock: held,
        })
    }

    /// The runner's own id.
    pub fn runner(&self) -> &str {
        &self.runner
    }

    /// The record, empty when none was written.
    pub fn read(&self) -> Result<Kept, RunnerError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Kept::new(Vec::new()));
            }
            Err(error) => {
                return Err(unavailable(format!(
                    "reading {}: {error}",
                    self.path.display()
                )));
            }
        };
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
            unavailable(format!("{} does not read: {error}", self.path.display()))
        })?;
        let found = value
            .get("format")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| unavailable("the runner record names no format"))?;
        let kept: Kept = match found {
            FORMAT => serde_json::from_value(value),
            legacy::FORMAT => legacy::migrate(value),
            other => {
                return Err(unavailable(format!(
                    "{} is in format {other}, not {FORMAT}",
                    self.path.display()
                )));
            }
        }
        .map_err(|error| unavailable(format!("{} does not read: {error}", self.path.display())))?;
        for session in &kept.sessions {
            if session.leader_start.as_ref().is_some_and(|leader| {
                session.pid != Some(leader.pid) || leader.pid <= 1 || leader.start.0.is_empty()
            }) {
                return Err(unavailable(format!(
                    "session {} has an invalid leader start identity",
                    session.session
                )));
            }
        }
        Ok(kept)
    }

    /// Replace the record with `kept`, durably.
    pub fn write(&self, kept: &Kept) -> Result<(), RunnerError> {
        let bytes = serde_json::to_vec_pretty(kept).map_err(unavailable)?;
        replace(&self.path, &bytes)
            .map_err(|error| unavailable(format!("writing {}: {error}", self.path.display())))
    }
}

impl Drop for StateFile {
    /// Let the directory go, so a runner opened after this one in the same
    /// process may hold it; the kernel lets it go at the process's end too.
    fn drop(&mut self) {
        if let Err(error) = rustix::fs::flock(&self.lock, rustix::fs::FlockOperation::Unlock) {
            crate::error::said(&format!(
                "the state directory's lock was already gone: {error}"
            ));
        }
    }
}

/// Write `bytes` beside `path`, durably, then move them over it.
pub(crate) fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = fs::File::create(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}

/// The id kept at `path`, made and kept durably when there is none; one
/// that is not the 32 lowercase hexadecimal characters this writes is
/// refused by name.
fn runner_id(path: &Path) -> Result<String, RunnerError> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let id = text.trim();
            if id.len() == 32 && crate::protocol::unhex(id).is_some() {
                Ok(id.to_owned())
            } else {
                Err(unavailable(format!(
                    "{} does not hold a runner id as 32 hexadecimal characters",
                    path.display()
                )))
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let id = crate::protocol::hex(&rand::random::<[u8; 16]>());
            replace(path, id.as_bytes())
                .map_err(|error| unavailable(format!("writing {}: {error}", path.display())))?;
            Ok(id)
        }
        Err(error) => Err(unavailable(format!("reading {}: {error}", path.display()))),
    }
}

/// The exclusive lock on `dir`, refused `runner_state_held` when another
/// runner holds it.
fn hold(dir: &Path) -> Result<fs::File, RunnerError> {
    let path = dir.join("lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|error| unavailable(format!("opening {}: {error}", path.display())))?;
    match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(file),
        Err(rustix::io::Errno::WOULDBLOCK) => Err(RunnerError::StateHeld {
            state: dir.display().to_string(),
        }),
        Err(error) => Err(unavailable(format!("locking {}: {error}", path.display()))),
    }
}
