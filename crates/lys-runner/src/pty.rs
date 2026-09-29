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

/// What a restart found of a process group its last run recorded and never
/// saw end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Left {
    /// No process is in the group: it is gone.
    Gone,
    /// Processes were still in it, and were ended with it.
    Ended,
}

/// End what is left of process group `pid`, which a runner started and a
/// restart found recorded without an end. A process that ignored the
/// hang-up its terminal closing sent is still in the group, and is ended
/// with it by a signal no process can ignore. A group that answers but may
/// not be signalled is not this user's, so not the runner's: it is refused
/// `end_failed` by name, and the caller reports the runner's own as gone.
pub fn end_left_group(pid: u32) -> Result<Left, RunnerError> {
    let group = group(pid)?;
    match test_kill_process_group(group) {
        Err(rustix::io::Errno::SRCH) => Ok(Left::Gone),
        Err(error) => Err(RunnerError::refused(
            "end_failed",
            format!("process group {pid} answers but is not this runner's to end: {error}"),
        )),
        Ok(()) => end_group(pid).map(|()| Left::Ended),
    }
}

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
