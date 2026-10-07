//! One process in its own pseudo-terminal.
//!
//! The terminal is opened, made the child's controlling terminal and resized
//! through `portable-pty`, so this crate holds no unsafe code of its own. The
//! runner's end of the terminal is kept; the child's end is closed here once
//! the child holds it, so the runner reads the end of output when the child
//! and everything it started have closed the terminal. A process is ended
//! with its whole process group, through `rustix`'s safe signal call.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::{Read, Write};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use rustix::process::{Pid, Signal, kill_process_group, test_kill_process_group};

use crate::error::RunnerError;

mod cleanup;

/// The terminal type a session is given when its launch names none.
pub const TERM: &str = "xterm-256color";

/// An isolated helper has no implicit shell. portable-pty otherwise inserts
/// the account's login shell even after `env_clear`; an explicit empty value
/// prevents that ambient setting from crossing the boundary.
pub const ISOLATED_SHELL: &str = "";

/// What a process is started with.
pub struct Spawn<'a> {
    /// The program.
    pub program: &'a str,
    /// Its arguments.
    pub arguments: &'a [String],
    /// The directory it runs in. Never empty: a launch that names none is
    /// refused by name, and neither the runner's folder nor the login's home
    /// stands in for it.
    pub directory: &'a str,
    /// Variables set beside the runner's own environment.
    pub environment: &'a BTreeMap<String, String>,
    /// Width in columns.
    pub columns: u16,
    /// Height in rows.
    pub rows: u16,
}

/// A process running in its own pseudo-terminal.
pub struct Spawned {
    /// The terminal's output.
    pub reader: Box<dyn Read + Send>,
    /// The terminal's input.
    pub writer: Box<dyn Write + Send>,
    /// The runner's end of the terminal, kept to resize it.
    pub master: Box<dyn MasterPty + Send>,
    /// The process, waited on for its exit.
    pub child: Box<dyn Child + Send + Sync>,
    /// Its process id.
    pub pid: u32,
}

fn failed(what: &str, error: &dyn Display) -> RunnerError {
    RunnerError::refused("spawn_failed", format!("{what}: {error}"))
}

fn size(columns: u16, rows: u16) -> Result<PtySize, RunnerError> {
    if columns == 0 || rows == 0 {
        return Err(RunnerError::refused(
            "size_invalid",
            "a terminal is at least one column wide and one row high",
        ));
    }
    Ok(PtySize {
        rows,
        cols: columns,
        pixel_width: 0,
        pixel_height: 0,
    })
}

/// Start `spawn` in a new pseudo-terminal of its size.
pub fn spawn(spawn: &Spawn<'_>) -> Result<Spawned, RunnerError> {
    spawn_with_environment(spawn, false)
}

/// Start a trusted containment helper with only the explicitly supplied
/// environment, TERM and an empty SHELL. No inherited loader, proxy, credential or harness
/// setting reaches its first instruction. This is not a sandbox receipt.
pub fn spawn_isolated(spawn: &Spawn<'_>) -> Result<Spawned, RunnerError> {
    for name in spawn.environment.keys() {
        if name != "TERM" {
            return Err(RunnerError::refused(
                "containment_entry_refused",
                format!(
                    "trusted helper environment cannot contain {name}; install agent variables after containment"
                ),
            ));
        }
    }
    spawn_with_environment(spawn, true)
}

fn spawn_with_environment(spawn: &Spawn<'_>, isolated: bool) -> Result<Spawned, RunnerError> {
    // Runs go where the launch says they go. A launch that names no
    // directory is refused by name: never the runner's folder, never the
    // login's home, never a folder of Lys's (Tom, 3 October 2026).
    if spawn.directory.is_empty() {
        return Err(RunnerError::refused(
            "launch_without_directory",
            "the launch names no working directory to start the run in",
        ));
    }
    let requested_directory = std::path::PathBuf::from(spawn.directory);
    if !requested_directory.is_absolute() {
        return Err(RunnerError::refused(
            "spawn_failed",
            format!(
                "the run's directory {} is not absolute",
                requested_directory.display()
            ),
        ));
    }
    let directory = rustix::fs::open(
        &requested_directory,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|error| {
        failed(
            &format!("working directory {}", requested_directory.display()),
            &error,
        )
    })?;
    let pair = native_pty_system()
        .openpty(size(spawn.columns, spawn.rows)?)
        .map_err(|error| failed("the pseudo-terminal could not be opened", &error))?;
    let mut command = CommandBuilder::new(spawn.program);
    if isolated {
        command.env_clear();
        command.env("SHELL", ISOLATED_SHELL);
    }
    command.args(spawn.arguments);
    if !spawn.environment.contains_key("TERM") {
        command.env("TERM", TERM);
    }
    for (name, value) in spawn.environment {
        command.env(name, value);
    }
    crate::pty_command::set_directory(&mut command, spawn.program, &requested_directory)?;
    // Prepare fallible terminal handles before starting any process. A failed
    // clone/take must not leave an agent running with no session owner.
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| failed("the terminal's output could not be read", &error))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| failed("the terminal's input could not be written", &error))?;
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| failed(&format!("{} could not be started", spawn.program), &error))?;
    drop(directory);
    drop(pair.slave);
    let Some(pid) = child.process_id() else {
        child
            .kill()
            .map_err(|error| failed("process has no id and could not be stopped", &error))?;
        child
            .wait()
            .map_err(|error| failed("process has no id and its exit could not be read", &error))?;
        return Err(failed(
            "the process",
            &"has no process id; stopped and reaped",
        ));
    };
    Ok(Spawned {
        reader,
        writer,
        master: pair.master,
        child,
        pid,
    })
}

/// Resize the terminal `master` holds.
pub fn resize(master: &dyn MasterPty, columns: u16, rows: u16) -> Result<(), RunnerError> {
    master
        .resize(size(columns, rows)?)
        .map_err(|error| RunnerError::refused("resize_failed", error.to_string()))
}

/// What a restart found of a process group its last run recorded and never
/// saw end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Left {
    /// No process is in the group: it is gone.
    Gone,
    /// Every listed member exited or was absent; the reason records cleanup diagnostics.
    Ended {
        /// The group probe and each proved member's signal and exit diagnostics.
        reason: Option<String>,
    },
    /// The live leader has another start identity, so the group was not signalled.
    Reused,
    /// Listed members could not all be proved, signalled, or observed exiting.
    Unended {
        /// Every member left, with the named reason.
        reason: String,
    },
}

/// Signal each member only when its session is the recorded leader and its
/// start is not earlier than the leader's. A live, reused leader prevents
/// cleanup; an absent leader does not prevent proving the remaining members.
/// Identity verification and signalling are separate kernel operations: a
/// process can exit and its id can be reassigned between them.
pub fn end_left_group(leader: &crate::peer::Leader) -> Result<Left, RunnerError> {
    end_left_group_with(leader, test_kill_process_group)
}

fn end_left_group_with(
    leader: &crate::peer::Leader,
    probe: impl FnOnce(Pid) -> rustix::io::Result<()>,
) -> Result<Left, RunnerError> {
    Ok(prepare_with(leader, probe)?.wait(None))
}

/// Exit registrations retained after ownership checks and signals, before waiting.
pub struct PreparedEnd {
    plan: cleanup::Plan<cleanup::Kernel>,
}

impl PreparedEnd {
    /// Await registered exits and clean up newcomers, without holding a session lock.
    /// A readable cancellation descriptor refuses every exit still awaited.
    pub fn wait(mut self, cancel: Option<std::os::fd::BorrowedFd<'_>>) -> Left {
        self.plan.wait(cancel)
    }

    /// Refuse unconfirmed exits without blocking, for a synchronous drop path.
    pub fn refuse(self) -> Left {
        self.plan.decline()
    }
}

/// Prove members, register exit notifications, and signal before returning.
/// Waiting is explicitly deferred so a caller can release its session table.
///
/// # Errors
/// Refuses an invalid group or unavailable kernel exit queue.
pub fn prepare_left_group(leader: &crate::peer::Leader) -> Result<PreparedEnd, RunnerError> {
    prepare_with(leader, test_kill_process_group)
}

fn prepare_with(
    leader: &crate::peer::Leader,
    probe: impl FnOnce(Pid) -> rustix::io::Result<()>,
) -> Result<PreparedEnd, RunnerError> {
    let group = group(leader.pid)?;
    #[cfg(target_os = "macos")]
    let kernel = cleanup::Kernel::new()?;
    #[cfg(target_os = "linux")]
    let kernel = cleanup::Kernel::new();
    Ok(PreparedEnd {
        plan: cleanup::Plan::prepare(leader.clone(), kernel, probe(group)),
    })
}

fn group(pid: u32) -> Result<Pid, RunnerError> {
    i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .filter(|id| id.as_raw_nonzero().get() > 1)
        .ok_or_else(|| RunnerError::refused("end_failed", format!("{pid} is not a process id")))
}

/// End the process `pid` and everything it started in its terminal: the
/// child leads its own session, so its process group is ended whole, at
/// once, by a signal no process can ignore.
pub fn end_group(pid: u32) -> Result<(), RunnerError> {
    kill_process_group(group(pid)?, Signal::KILL).map_err(|error| {
        RunnerError::refused("end_failed", format!("process group {pid}: {error}"))
    })
}

/// Hang up the recorded leader's terminal, as closing a terminal window does:
/// its process group is sent the hang-up every interactive program honours,
/// so a harness can flush its final output and a shell exits. Its owner waits
/// on the child's exit, then ends verified remnants with [`end_left_group`].
///
/// # Errors
/// Refuses a reused leader or a failure to read or signal it.
pub fn end(leader: &crate::peer::Leader) -> Result<(), RunnerError> {
    let pid = group(leader.pid)?;
    match crate::peer::present_start(leader.pid)? {
        None => return Ok(()),
        Some(start) if start == leader.start => {}
        Some(_) => {
            return Err(RunnerError::refused(
                "process_start_mismatch",
                "the session leader was reused",
            ));
        }
    }
    match kill_process_group(pid, Signal::HUP) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(RunnerError::refused("end_failed", error.to_string())),
    }
}
