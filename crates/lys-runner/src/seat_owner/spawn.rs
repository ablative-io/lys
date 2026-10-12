//! Starting an owner from the runner (AGENTS-004 R1): the plan it reads,
//! the detached spawn, and the ready line waited on as a signal.
//!
//! The owner is `lys runner seat-owner`, started in a session and process
//! group of its own, its standard input closed, its words in its own log,
//! so that the terminal that watches the seat, the runner that launched it
//! and the identity server that admits its acts can each close or die
//! without closing one of the owner's descriptors or signalling its harness.
//! Readiness is a line on a pipe: the runner blocks reading the owner's
//! standard output until the owner, listening, writes it. No timer waits
//! for an owner; an owner that exits first closes the pipe, and that is
//! its refusal.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use super::counts::Meter;
use super::protocol::{
    LOG, OwnerBinding, OwnerEndpoint, PLAN, SOCKET, fits_socket, owner_dir, owner_socket,
    parse_ready_line,
};
use super::store::{JOURNAL, PROJECTION};
use crate::error::RunnerError;
use crate::harness_control::ManagedLaunch;
use crate::peer::{self, Leader};

/// The plan an owner reads at start, written by the runner into the
/// owner's directory before the owner is spawned. It holds public identity
/// and credential references only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerPlan {
    /// The seat, session, conversation and lease generation.
    pub binding: OwnerBinding,
    /// The managed launch, exactly as AGENTS-002 asked for it.
    pub managed: ManagedLaunch,
    /// The person responsible for the session, if named.
    pub responsible: Option<String>,
    /// Names under which the runner's secret store answers; never bytes.
    pub credential_references: Vec<String>,
    /// The intent id of the establish, chosen by the runner.
    pub establish: String,
    /// The intent id of the harness record, chosen by the runner.
    pub harness: String,
    /// The socket the owner listens on, chosen by the runner at start
    /// ([`super::protocol::owner_socket`]). A plan written before owner
    /// sockets moved has none, and its owner listens on
    /// [`SOCKET`] in its directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket: Option<PathBuf>,
}

impl OwnerPlan {
    /// A plan for `binding` and `managed`, with fresh intent ids.
    #[must_use]
    pub fn new(binding: OwnerBinding, managed: ManagedLaunch, responsible: Option<String>) -> Self {
        Self {
            binding,
            managed,
            responsible,
            credential_references: Vec::new(),
            establish: fresh_intent(),
            harness: fresh_intent(),
            socket: None,
        }
    }

    /// The socket the owner in `dir` listens on: the plan's own, or
    /// [`SOCKET`] in `dir` for a plan written before owner sockets moved.
    #[must_use]
    pub fn socket_in(&self, dir: &Path) -> PathBuf {
        self.socket.clone().unwrap_or_else(|| dir.join(SOCKET))
    }

    /// Refuses a plan whose launch names another session than its binding.
    ///
    /// # Errors
    ///
    /// `seat_owner_binding_invalid`.
    pub fn validate(&self) -> Result<(), RunnerError> {
        self.binding.validate()?;
        if self.binding.conversation.is_empty()
            && (self.managed.transport != crate::harness_control::Transport::Codex
                || self.managed.resume)
        {
            return Err(RunnerError::refused(
                "seat_owner_binding_invalid",
                "only a fresh Codex launch may await a conversation",
            ));
        }
        if self.managed.resume {
            self.managed
                .transport
                .validate_resume(&self.managed.conversation)?;
        }
        if self.managed.launch.session != self.binding.session {
            return Err(RunnerError::refused(
                "seat_owner_binding_invalid",
                format!(
                    "the launch names session {} and the binding session {}",
                    self.managed.launch.session, self.binding.session
                ),
            ));
        }
        if self.managed.conversation != self.binding.conversation {
            return Err(RunnerError::refused(
                "seat_owner_binding_invalid",
                "the launch and the binding name different conversations",
            ));
        }
        if !super::rules::is_hex_id(&self.establish) || !super::rules::is_hex_id(&self.harness) {
            return Err(RunnerError::refused(
                "seat_owner_intent_invalid",
                "a plan's intent ids are 32 lowercase hexadecimal characters",
            ));
        }
        Ok(())
    }

    /// Writes the plan into `dir`, readable by its owner only.
    ///
    /// # Errors
    ///
    /// `seat_owner_start_failed` when it cannot be written.
    pub fn write(&self, dir: &Path) -> Result<PathBuf, RunnerError> {
        let path = dir.join(PLAN);
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| RunnerError::refused("seat_owner_start_failed", error.to_string()))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| start_failed(format!("writing {}: {error}", path.display())))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| start_failed(format!("writing {}: {error}", path.display())))?;
        Ok(path)
    }

    /// Reads a plan back from an owner's directory.
    ///
    /// # Errors
    ///
    /// `seat_owner_start_failed` when it cannot be read or does not validate.
    pub fn read(dir: &Path) -> Result<Self, RunnerError> {
        let path = dir.join(PLAN);
        let bytes = fs::read(&path)
            .map_err(|error| start_failed(format!("reading {}: {error}", path.display())))?;
        let plan: Self = serde_json::from_slice(&bytes)
            .map_err(|error| start_failed(format!("{} does not read: {error}", path.display())))?;
        plan.validate()?;
        Ok(plan)
    }
}

/// A fresh intent id: 32 lowercase hexadecimal characters.
#[must_use]
pub fn fresh_intent() -> String {
    crate::protocol::hex(&rand::random::<[u8; 16]>())
}

fn start_failed(words: impl Into<String>) -> RunnerError {
    RunnerError::refused("seat_owner_start_failed", words)
}

/// How the runner starts an owner.
#[derive(Debug, Clone, Copy)]
pub struct OwnerSpawn<'a> {
    /// The `lys` executable the owner runs as.
    pub lys: &'a Path,
    /// The runner's state directory, under which owners live.
    pub state: &'a Path,
    /// The directory owner sockets are made in, beside the runner's own
    /// socket.
    pub sockets: &'a Path,
    /// The server's public key file, which the owner verifies acts with.
    pub server_key: &'a Path,
    /// Scrollback for the owner's one session.
    pub scrollback: usize,
}

/// Starts the owner `plan` describes and waits on its ready line.
///
/// # Errors
///
/// `seat_owner_held` when the session already has an owner directory with
/// a record; `seat_owner_start_failed` when the owner cannot be spawned or
/// exits before it is ready; `seat_owner_unproved` when the ready line
/// names a process the kernel does not confirm.
pub fn start(spawn: &OwnerSpawn<'_>, plan: &OwnerPlan) -> Result<OwnerEndpoint, RunnerError> {
    plan.validate()?;
    let dir = owner_dir(spawn.state, &plan.binding.session);
    let planned = owner_socket(spawn.sockets, &plan.binding.session);
    fits_socket(&planned).map_err(|error| start_failed(error.to_string()))?;
    let plan = OwnerPlan {
        socket: Some(planned.clone()),
        ..plan.clone()
    };
    // An owner's record is its journal until its first checkpoint writes the
    // projection: either one present means the session has an owner. A file
    // whose presence the kernel cannot say is a refusal, never an absence.
    for record in [PROJECTION, JOURNAL] {
        let path = dir.join(record);
        let held = path
            .try_exists()
            .map_err(|error| start_failed(format!("reading {}: {error}", path.display())))?;
        if held {
            return Err(RunnerError::refused(
                "seat_owner_held",
                format!("session {} already has an owner", plan.binding.session),
            ));
        }
    }
    for made in [&dir, spawn.sockets] {
        fs::create_dir_all(made)
            .and_then(|()| fs::set_permissions(made, fs::Permissions::from_mode(0o700)))
            .map_err(|error| start_failed(format!("making {}: {error}", made.display())))?;
    }
    let began = std::time::Instant::now();
    plan.write(&dir)?;
    crate::error::step("owner_plan_written", began);
    let began = std::time::Instant::now();
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(dir.join(LOG))
        .map_err(|error| start_failed(format!("opening the owner's log: {error}")))?;
    let mut command = Command::new(spawn.lys);
    command
        .arg("runner")
        .arg("seat-owner")
        .arg("--dir")
        .arg(&dir)
        .arg("--server-key")
        .arg(spawn.server_key)
        .arg("--scrollback")
        .arg(spawn.scrollback.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(log);
    // The owner moves itself into a session and process group of its own
    // (`setsid`) as its first act in `process::serve`: the workspace denies
    // unsafe code, so no `pre_exec` hook runs here, and the child is not made
    // a group leader first, which would make its `setsid` fail.
    let mut child = command.spawn().map_err(|error| {
        start_failed(format!("{} could not start: {error}", spawn.lys.display()))
    })?;
    let Some(pipe) = child.stdout.take() else {
        return Err(abandon(
            &mut child,
            &start_failed("the owner's pipe was not made"),
        ));
    };
    let ready = match ready_within(pipe, READY_WITHIN) {
        Ok(ready) => {
            crate::error::step("owner_spawned_to_ready", began);
            ready
        }
        Err(error) => {
            let ended = abandon(&mut child, &error);
            return Err(with_log_tail(&ended, &dir.join(LOG)));
        }
    };
    match proved_ready(ready, &child, &planned) {
        Ok((runner, announced, build)) => {
            // The owner outlives this runner; reap it if it exits first,
            // without waiting here. The child is handed over only once the
            // reaper runs, so a reaper that cannot start leaves this start
            // holding it, to end and wait for: dropped unwaited, the killed
            // owner would stay a zombie the kernel still answers for.
            let pid = announced.pid;
            let (hand, handed) = std::sync::mpsc::channel::<std::process::Child>();
            let reaper = std::thread::Builder::new()
                .name(format!("lys-owner-reaper-{pid}"))
                .spawn(move || {
                    let Ok(mut child) = handed.recv() else {
                        return;
                    };
                    if let Err(error) = child.wait() {
                        crate::error::said(&format!("seat_owner_reaper_failed: {error}"));
                    }
                });
            if let Err(error) = reaper {
                return Err(abandon(
                    &mut child,
                    &start_failed(format!("the reaper could not start: {error}")),
                ));
            }
            if let Err(std::sync::mpsc::SendError(mut unhanded)) = hand.send(child) {
                return Err(abandon(
                    &mut unhanded,
                    &start_failed("the reaper ended before it took the owner"),
                ));
            }
            let mut endpoint = OwnerEndpoint::new(dir, planned, runner, announced);
            endpoint.build = build;
            Ok(endpoint)
        }
        Err(error) => Err(abandon(&mut child, &error)),
    }
}

/// `refusal`, its words ending with the last of the owner's log: an owner
/// that exits before it is ready says why only there.
fn with_log_tail(refusal: &RunnerError, log: &Path) -> RunnerError {
    const TAIL: usize = 2_048;
    let tail = fs::read(log).map_or_else(
        |error| format!("its log could not be read: {error}"),
        |bytes| {
            let from = bytes.len().saturating_sub(TAIL);
            let said = String::from_utf8_lossy(bytes.get(from..).unwrap_or_default());
            format!("its log ends: {}", said.trim())
        },
    );
    RunnerError::refused(&refusal.name(), format!("{refusal}; {tail}"))
}

/// How long a start waits for its owner's ready line: the owner starts its
/// harness before it writes it, so this is the harness's start and more.
pub const READY_WITHIN: std::time::Duration = std::time::Duration::from_secs(120);

/// The owner's ready line, read from `pipe` on a thread of its own, or
/// `seat_owner_start_failed` once `within` has passed with none: a start is
/// never left waiting on an owner that neither writes nor exits.
///
/// # Errors
///
/// [`wait_ready`]'s refusals, and `seat_owner_start_failed` past `within`.
pub fn ready_within(
    pipe: impl std::io::Read + Send + 'static,
    within: std::time::Duration,
) -> Result<(String, PathBuf, Leader, String), RunnerError> {
    let (sent, answer) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("lys-owner-ready".to_owned())
        .spawn(move || {
            let meter = Meter::default();
            // The receiver is gone only once the start has given up.
            drop(sent.send(wait_ready(&mut BufReader::new(pipe), &meter)));
        })
        .map_err(|error| start_failed(format!("the ready reader could not start: {error}")))?;
    answer.recv_timeout(within).unwrap_or_else(|_| {
        Err(start_failed(format!(
            "the owner wrote no ready line within {} s",
            within.as_secs()
        )))
    })
}

/// The ready line's runner, owner and build, once the owner it names is
/// the process this start spawned, proved at the kernel, listening on its
/// planned socket.
fn proved_ready(
    (runner, socket, announced, build): (String, PathBuf, Leader, String),
    child: &std::process::Child,
    planned: &Path,
) -> Result<(String, Leader, String), RunnerError> {
    if announced.pid != child.id() {
        return Err(RunnerError::refused(
            "seat_owner_unproved",
            format!(
                "the owner announced pid {} and the runner spawned pid {}",
                announced.pid,
                child.id()
            ),
        ));
    }
    let proved = peer::start_identity(announced.pid)?;
    if proved != announced.start {
        return Err(RunnerError::refused(
            "seat_owner_unproved",
            "the owner's announced start identity is not the kernel's",
        ));
    }
    if socket != planned {
        return Err(start_failed(format!(
            "the owner listens on {} and not on its planned {}",
            socket.display(),
            planned.display()
        )));
    }
    Ok((runner, announced, build))
}

/// Ends the owner a refused start spawned, with everything it started, and
/// answers `refusal` saying so: a start refused after its owner was spawned
/// leaves nothing running. The owner leads a process group of its own once
/// it has called `setsid`, so that group is killed; before then the owner
/// alone is. It is then waited for, which proves it gone.
fn abandon(child: &mut std::process::Child, refusal: &RunnerError) -> RunnerError {
    let group = ended_group(child.id());
    let killed = child.kill();
    let ended = match child.wait() {
        Ok(status) => format!("the owner was ended ({status}){group}"),
        Err(error) => format!("the owner could not be waited for: {error}; kill: {killed:?}"),
    };
    RunnerError::refused(&refusal.name(), format!("{refusal}; {ended}"))
}

/// Ends the owner `owner`, proved by its start identity, with its process
/// group, and waits until the kernel no longer has it, answering `refusal`
/// saying so. For an owner whose child handle this runner no longer holds.
///
/// # Errors
///
/// None: the refusal is the answer, with the end's outcome in its words.
pub fn end_owner(owner: &Leader, refusal: &RunnerError) -> RunnerError {
    let ended = match peer::start_identity(owner.pid) {
        Ok(start) if start == owner.start => {
            let group = ended_group(owner.pid);
            if super::exit_wait::gone_within(owner.pid, std::time::Duration::from_secs(5)) {
                format!("the owner was ended{group}")
            } else {
                format!("the owner did not end within 5 s{group}")
            }
        }
        Ok(_) => "the owner had already ended".to_owned(),
        Err(error) => format!("the owner's identity could not be read: {error}"),
    };
    RunnerError::refused(&refusal.name(), format!("{refusal}; {ended}"))
}

/// Kills the process group `pid` leads, answering words for the refusal.
fn ended_group(pid: u32) -> String {
    let Some(group) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    else {
        return String::new();
    };
    match rustix::process::kill_process_group(group, rustix::process::Signal::KILL) {
        Ok(()) => ", with its process group".to_owned(),
        Err(rustix::io::Errno::SRCH) => String::new(),
        Err(error) => format!(", its process group not ended: {error}"),
    }
}

/// Waits on the owner's ready line. The read blocks on the pipe: that is
/// the signal, and the meter shows no wake until the line arrives.
///
/// # Errors
///
/// `seat_owner_start_failed` when the pipe closes first or the first line
/// is not a ready line.
pub fn wait_ready(
    reader: &mut impl BufRead,
    meter: &Meter,
) -> Result<(String, PathBuf, Leader, String), RunnerError> {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|error| start_failed(format!("reading the owner's ready line: {error}")))?;
    meter.call();
    if read == 0 {
        return Err(start_failed("the owner exited before it was ready"));
    }
    meter.copied(u64::try_from(read).unwrap_or(u64::MAX));
    parse_ready_line(&line)
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
