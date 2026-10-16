//! A pipe child joins a new process session before its harness can execute.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use portable_pty::{Child, ChildKiller, ExitStatus};
use rustix::process::Pid;
use serde::{Deserialize, Serialize};

use super::{Binding, Executable, Transport};
use crate::error::RunnerError;
use crate::peer::Leader;

#[cfg(test)]
mod qualification;

mod executable;
mod pipe_spawn;
pub use executable::executable;
pub(crate) use executable::report_version;

mod qualified;
use qualified::QUALIFIED;
pub use qualified::mark as version_mark;
#[cfg(test)]
mod qualified_mark_tests;

pub mod identity_cache;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    program: PathBuf,
    arguments: Vec<String>,
    directory: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    pid: u32,
    executable: Executable,
}

fn failed(name: &str, error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(name, error.to_string())
}

/// Read exactly one whole protocol frame; EOF and truncated frames are named.
///
/// # Errors
/// Refuses unreadable, incomplete or non-JSON frames.
pub fn frame(reader: &mut impl BufRead) -> Result<serde_json::Value, RunnerError> {
    let mut text = String::new();
    let read = reader
        .read_line(&mut text)
        .map_err(|error| failed("control_transport_lost", error))?;
    if read == 0 {
        return Err(failed("control_transport_lost", "managed pipe ended"));
    }
    if !text.ends_with('\n') {
        return Err(failed(
            "control_frame_invalid",
            "managed frame is incomplete",
        ));
    }
    serde_json::from_str(&text).map_err(|error| {
        failed(
            "control_frame_invalid",
            format!("managed frame is not JSON: {error}"),
        )
    })
}

/// The installed CLI child entry: prove session isolation, wait for the parent,
/// then replace this process with the selected harness without a shell.
///
/// # Errors
/// Refuses invalid startup input, isolation, executable or exec failures.
pub fn entry() -> Result<(), RunnerError> {
    let mut input = File::from(
        rustix::io::dup(std::io::stdin()).map_err(|error| failed("control_entry_failed", error))?,
    );
    let entry: Entry = serde_json::from_value(startup_frame(&mut input)?).map_err(|error| {
        failed(
            "control_entry_invalid",
            format!("startup specification is invalid: {error}"),
        )
    })?;
    if !entry.directory.is_absolute() || !entry.directory.is_dir() {
        return Err(failed(
            "launch_without_directory",
            "managed launch needs its absolute working directory",
        ));
    }
    let selected = executable(&entry.program)?;
    rustix::process::setsid().map_err(|error| failed("control_process_unproved", error))?;
    let ready = Ready {
        pid: std::process::id(),
        executable: selected,
    };
    let mut stdout = std::io::stdout();
    serde_json::to_writer(&mut stdout, &ready)
        .map_err(|error| failed("control_entry_failed", error))?;
    stdout
        .write_all(b"\n")
        .and_then(|()| stdout.flush())
        .map_err(|error| failed("control_entry_failed", error))?;
    execute_permission(&mut input)?;
    drop(input);
    // The invocation spelling can select a fork's native entry point.
    let error = Command::new(&entry.program)
        .args(entry.arguments)
        .current_dir(entry.directory)
        .exec();
    Err(failed("control_exec_failed", error))
}

fn startup_frame(input: &mut impl Read) -> Result<serde_json::Value, RunnerError> {
    let mut line = Vec::new();
    let mut byte = [0];
    loop {
        if let Err(error) = input.read_exact(&mut byte) {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                return Err(if line.is_empty() {
                    failed("control_transport_lost", "managed pipe ended")
                } else {
                    failed("control_frame_invalid", "managed frame is incomplete")
                });
            }
            return Err(failed("control_transport_lost", error));
        }
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
    }
    serde_json::from_slice(&line).map_err(|error| {
        failed(
            "control_frame_invalid",
            format!("managed frame is not JSON: {error}"),
        )
    })
}

fn execute_permission(input: &mut impl Read) -> Result<(), RunnerError> {
    let mut permission = [0; b"{\"execute\":true}\n".len()];
    input
        .read_exact(&mut permission)
        .map_err(|error| failed("control_transport_lost", error))?;
    if permission != *b"{\"execute\":true}\n" {
        return Err(failed(
            "control_entry_invalid",
            "parent did not authorize the proved process",
        ));
    }
    Ok(())
}

pub(crate) struct Spawned {
    pub(crate) reader: Box<dyn Read + Send>,
    pub(crate) writer: Box<dyn Write + Send>,
    pub(crate) child: Box<dyn Child + Send + Sync>,
    pub(crate) binding: Binding,
}

/// Resolve the selected executable using the launch's effective PATH.
fn selected(program: &str, environment: &BTreeMap<String, String>) -> Result<PathBuf, RunnerError> {
    crate::program_path::selected(program, environment)
        .map_err(|reason| failed("control_executable_unproved", reason))
}

fn reported_version(
    selected: &Path,
    harness_name: &str,
    environment: &BTreeMap<String, String>,
) -> Result<(String, String), RunnerError> {
    // A file whose version was asked while it had its present identity is
    // not run again to say it (`identity_cache`); Claude Code's `--version`
    // is a whole program start.
    Ok(if let Some(reported) = identity_cache::version(selected) {
        ("0 (kept)".to_owned(), reported)
    } else {
        let began = std::time::Instant::now();
        let (status, bytes) = report_version(selected, environment).map_err(|error| {
            failed(
                "control_adapter_unqualified",
                format!(
                    "{harness_name} binary {} could not report its version: {error}",
                    selected.display()
                ),
            )
        })?;
        crate::error::step("harness_version_probe", began);
        let reported = String::from_utf8(bytes).map_err(|error| {
            failed(
                "control_adapter_unqualified",
                format!(
                    "{harness_name} binary {} reported a non-UTF-8 version: {error}",
                    selected.display()
                ),
            )
        })?;
        if !status.success() {
            return Err(failed(
                "control_adapter_unqualified",
                format!(
                    "{harness_name} binary {} could not report its version (exit {status})",
                    selected.display()
                ),
            ));
        }
        identity_cache::remember_version(selected, &reported);
        (status.to_string(), reported)
    })
}

pub(crate) fn spawn(
    launch: &crate::protocol::Launch,
    transport: Transport,
    conversation: &str,
    generation: u64,
    resume: bool,
) -> Result<Spawned, RunnerError> {
    let selected = selected(&launch.program, &launch.environment)?;
    let (harness_name, adapter) = match transport {
        Transport::Claude => ("Claude", "claude-stream-json/1"),
        Transport::Codex => ("Codex", "codex-app-server/1"),
        Transport::Pty => {
            return Err(failed(
                "control_transport_unsupported",
                "automatic controls require a managed pipe",
            ));
        }
    };
    let (exited, reported) = reported_version(&selected, harness_name, &launch.environment)?;
    let harness_version = crate::tracking::version_in(&reported).ok_or_else(|| {
        failed(
            "control_adapter_unqualified",
            format!(
                "{harness_name} binary {} reported no version (exit {})",
                selected.display(),
                exited
            ),
        )
    })?;
    let admitted = QUALIFIED.iter().any(|proof| proof.adapter == adapter);
    #[cfg(test)]
    let admitted = admitted || qualification::approved(adapter, &harness_version)?;
    if !admitted {
        return Err(failed(
            "control_adapter_unqualified",
            format!(
                "{harness_name} binary {} reports version {harness_version}; {adapter} has no launched-binary qualification",
                selected.display()
            ),
        ));
    }
    let harness = executable(&selected)?;
    let adapter = adapter.to_owned();
    let arguments = match transport {
        Transport::Claude => {
            if resume {
                super::claude::resume_arguments(&launch.arguments, conversation)?
            } else {
                super::claude::arguments(&launch.arguments, conversation)?
            }
        }
        Transport::Codex => super::codex::arguments(&launch.arguments)?,
        Transport::Pty => {
            return Err(failed(
                "control_transport_unsupported",
                "automatic controls require a managed pipe",
            ));
        }
    };
    pipe_spawn::spawn(
        launch,
        pipe_spawn::Plan {
            selected,
            arguments,
            harness,
            adapter,
            harness_version,
            conversation: conversation.to_owned(),
            generation,
            clean: false,
        },
    )
}

pub(crate) fn spawn_seat(
    launch: &crate::protocol::Launch,
    generation: u64,
) -> Result<Spawned, RunnerError> {
    let selected = selected(&launch.program, &launch.environment)?;
    let harness = executable(&selected)?;
    pipe_spawn::spawn(
        launch,
        pipe_spawn::Plan {
            selected,
            arguments: launch.arguments.clone(),
            harness,
            adapter: "acp/1".to_owned(),
            harness_version: "seat-program/v1".to_owned(),
            conversation: String::new(),
            generation,
            clean: true,
        },
    )
}

#[derive(Debug)]
struct PipeChild {
    child: std::process::Child,
    leader: Leader,
}

#[derive(Debug, Clone)]
struct Killer(Leader);

impl ChildKiller for Killer {
    fn kill(&mut self) -> std::io::Result<()> {
        crate::pty::end(&self.0).map_err(std::io::Error::other)
    }
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(self.clone())
    }
}

impl ChildKiller for PipeChild {
    fn kill(&mut self) -> std::io::Result<()> {
        crate::pty::end(&self.leader).map_err(std::io::Error::other)
    }
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(Killer(self.leader.clone()))
    }
}

impl Child for PipeChild {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.child
            .try_wait()
            .map(|status| status.map(ExitStatus::from))
    }
    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.wait().map(ExitStatus::from)
    }
    fn process_id(&self) -> Option<u32> {
        Some(self.child.id())
    }
}

pub(crate) fn validate_start(
    sessions: &crate::session::Sessions,
    managed: &super::ManagedLaunch,
    responsible: Option<&str>,
) -> Result<(), RunnerError> {
    if managed.transport == Transport::Pty && managed.requires_controls {
        return Err(RunnerError::refused(
            "control_transport_unsupported",
            "required automatic controls cannot use a terminal",
        ));
    }
    if let Some(person) = responsible
        && (person.is_empty()
            || person == "lys"
            || person.trim() != person
            || person.chars().any(char::is_control))
    {
        return Err(RunnerError::refused(
            "SessionResponsibleInvalid",
            "a verified responsible person is required",
        ));
    }
    if managed.requires_controls
        && sessions
            .lock()?
            .sessions
            .get(&managed.launch.session)
            .is_some_and(|session| session.managed.is_none())
    {
        return Err(RunnerError::refused(
            "control_transport_unsupported",
            "existing terminal session cannot be migrated to a managed pipe",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod seat_test_support;

#[cfg(test)]
mod startup_tests;

#[cfg(test)]
#[path = "process_path_tests.rs"]
mod path_tests;
