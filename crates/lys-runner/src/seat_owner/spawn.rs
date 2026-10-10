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
    LOG, OwnerBinding, OwnerEndpoint, PLAN, SOCKET, owner_dir, parse_ready_line,
};
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
        }
    }

    /// Refuses a plan whose launch names another session than its binding.
    ///
    /// # Errors
    ///
    /// `seat_owner_binding_invalid`.
    pub fn validate(&self) -> Result<(), RunnerError> {
        self.binding.validate()?;
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
    if dir.join("seat-owners.json").exists() {
        return Err(RunnerError::refused(
            "seat_owner_held",
            format!("session {} already has an owner", plan.binding.session),
        ));
    }
    fs::create_dir_all(&dir)
        .and_then(|()| fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)))
        .map_err(|error| start_failed(format!("making {}: {error}", dir.display())))?;
    plan.write(&dir)?;
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
    let pipe = child
        .stdout
        .take()
        .ok_or_else(|| start_failed("the owner's pipe was not made"))?;
    let meter = Meter::default();
    let ready = wait_ready(&mut BufReader::new(pipe), &meter);
    let (runner, socket, announced, build) = match ready {
        Ok(ready) => ready,
        Err(error) => {
            let status = child.wait().map_or_else(
                |wait| format!("and its exit could not be read: {wait}"),
                |status| format!("and exited {status}"),
            );
            return Err(start_failed(format!("{error} {status}")));
        }
    };
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
    if socket != dir.join(SOCKET) {
        return Err(start_failed(format!(
            "the owner listens on {} and not in its directory",
            socket.display()
        )));
    }
    // The owner outlives this runner; reap it if it exits first, without
    // waiting here.
    std::thread::Builder::new()
        .name(format!("lys-owner-reaper-{}", announced.pid))
        .spawn(move || {
            if let Err(error) = child.wait() {
                crate::error::said(&format!("seat_owner_reaper_failed: {error}"));
            }
        })
        .map_err(|error| start_failed(format!("the reaper could not start: {error}")))?;
    let mut endpoint = OwnerEndpoint::new(dir, runner, announced);
    endpoint.build = build;
    Ok(endpoint)
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
