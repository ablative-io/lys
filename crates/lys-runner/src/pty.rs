//! One process in its own pseudo-terminal.
//!
//! The terminal is opened, made the child's controlling terminal and resized
//! through `portable-pty`, so this crate holds no unsafe code of its own. The
//! runner's end of the terminal is kept; the child's end is closed here once
//! the child holds it, so the runner reads the end of output when the child
//! and everything it started have closed the terminal. A process is ended
//! with its whole process group, through `rustix`'s safe signal call.
//!
//! A process group is known by its leader's id and the instant that leader
//! started, as the operating system reports it ([`leader_started`]); a
//! restart that finds a group recorded without an end signals it only when
//! both still match, never on the group's number alone.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::{Read, Write};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use rustix::process::{Pid, Signal, kill_process_group};

use crate::error::RunnerError;

/// The terminal type a session is given when its launch names none.
pub const TERM: &str = "xterm-256color";

/// What a process is started with.
pub struct Spawn<'a> {
    /// The program.
    pub program: &'a str,
    /// Its arguments.
    pub arguments: &'a [String],
    /// The directory it runs in; the runner's own when empty.
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
    let pair = native_pty_system()
        .openpty(size(spawn.columns, spawn.rows)?)
        .map_err(|error| failed("the pseudo-terminal could not be opened", &error))?;
    let mut command = CommandBuilder::new(spawn.program);
    command.args(spawn.arguments);
    if !spawn.directory.is_empty() {
        command.cwd(spawn.directory);
    }
    if !spawn.environment.contains_key("TERM") {
        command.env("TERM", TERM);
    }
    for (name, value) in spawn.environment {
        command.env(name, value);
    }
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| failed(&format!("{} could not be started", spawn.program), &error))?;
    drop(pair.slave);
    let pid = child
        .process_id()
        .ok_or_else(|| failed("the process", &"has no process id"))?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| failed("the terminal's output could not be read", &error))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| failed("the terminal's input could not be written", &error))?;
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

/// The instant process `pid` started, as the operating system reports it:
/// `ps`'s `lstart`, read in the C locale and in UTC so the same process
/// reads the same whoever asks. It is to the second, the finest `ps`
/// reports on macOS and Linux alike: a process id is taken again within the
/// second its last holder started only after the ids wrap round. `None`
/// when no process has that id: `ps` then fails and says nothing. The text
/// is kept as it was reported and compared exactly; it is never parsed, so
/// no reading of it can make two different reports agree. A `ps` that
/// cannot be run, or answers in any other way, is refused
/// `leader_unreadable` by name.
pub fn leader_started(pid: u32) -> Result<Option<String>, RunnerError> {
    let unreadable = |why: String| {
        RunnerError::refused(
            "leader_unreadable",
            format!("the start of process {pid} could not be read: {why}"),
        )
    };
    let output = std::process::Command::new(PS)
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| unreadable(format!("{PS} could not be run: {error}")))?;
    let started = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let said = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    match (output.status.success(), started.is_empty(), said.is_empty()) {
        (true, false, _) => Ok(Some(started)),
        (false, true, true) => Ok(None),
        _ => Err(unreadable(format!(
            "{PS} ended {} and answered '{started}', saying '{said}'",
            output.status
        ))),
    }
}

/// The program that reports a process's start.
const PS: &str = "/bin/ps";

fn group(pid: u32) -> Result<Pid, RunnerError> {
    i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
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
