//! A pipe child joins a new process session before its harness can execute.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use portable_pty::{Child, ChildKiller, ExitStatus};
use rustix::process::{Pid, getpgid, getsid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{Binding, Executable, Transport};
use crate::error::RunnerError;
use crate::peer::Leader;

struct Qualification {
    adapter: &'static str,
    version: &'static str,
}

// A wire fixture is insufficient to qualify an installed executable.
const QUALIFIED: &[Qualification] = &[];

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

/// Fingerprint the executable resolved for a launch, with bounded read buffers.
///
/// # Errors
/// Refuses missing, nonregular, nonexecutable or unreadable files.
pub fn executable(path: &Path) -> Result<Executable, RunnerError> {
    let path = path
        .canonicalize()
        .map_err(|error| failed("control_executable_unproved", error))?;
    let mut file =
        File::open(&path).map_err(|error| failed("control_executable_unproved", error))?;
    let metadata = file
        .metadata()
        .map_err(|error| failed("control_executable_unproved", error))?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(failed(
            "control_executable_unproved",
            "selected file is not an executable",
        ));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 16384];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| failed("control_executable_unproved", error))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(Executable {
        path: path
            .to_str()
            .ok_or_else(|| failed("control_executable_unproved", "executable path is not text"))?
            .to_owned(),
        sha256: crate::protocol::hex(&hash.finalize()),
    })
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
        executable: selected.clone(),
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
    let error = Command::new(selected.path)
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
    let found = if program.contains('/') {
        PathBuf::from(program)
    } else {
        let path = environment
            .get("PATH")
            .cloned()
            .or_else(|| std::env::var("PATH").ok())
            .ok_or_else(|| failed("control_executable_unproved", "launch has no PATH"))?;
        std::env::split_paths(&path)
            .map(|directory| directory.join(program))
            .find(|path| path.is_file())
            .ok_or_else(|| {
                failed(
                    "control_executable_unproved",
                    "selected harness was not found",
                )
            })?
    };
    found
        .canonicalize()
        .map_err(|error| failed("control_executable_unproved", error))
}

pub(crate) fn spawn(
    launch: &crate::protocol::Launch,
    transport: Transport,
    conversation: &str,
    generation: u64,
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
    let (status, bytes) = report_version(&selected, &launch.environment).map_err(|error| {
        failed(
            "control_adapter_unqualified",
            format!(
                "{harness_name} binary {} could not report its version: {error}",
                selected.display()
            ),
        )
    })?;
    let reported = String::from_utf8(bytes).map_err(|error| {
        failed(
            "control_adapter_unqualified",
            format!(
                "{harness_name} binary {} reported a non-UTF-8 version: {error}",
                selected.display()
            ),
        )
    })?;
    let harness_version = crate::tracking::version_in(&reported).ok_or_else(|| {
        failed(
            "control_adapter_unqualified",
            format!(
                "{harness_name} binary {} reported no version (exit {})",
                selected.display(),
                status
            ),
        )
    })?;
    if !status.success()
        || !QUALIFIED
            .iter()
            .any(|proof| proof.adapter == adapter && proof.version == harness_version)
    {
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
        Transport::Claude => super::claude::arguments(&launch.arguments, conversation)?,
        Transport::Codex => super::codex::arguments(&launch.arguments)?,
        Transport::Pty => {
            return Err(failed(
                "control_transport_unsupported",
                "automatic controls require a managed pipe",
            ));
        }
    };
    let current =
        std::env::current_exe().map_err(|error| failed("control_entry_unproved", error))?;
    let entry_identity = executable(&current)?;
    let mut child = Command::new(&current)
        .args(["runner", "managed-entry"])
        .envs(&launch.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| failed("control_spawn_failed", error))?;
    let mut proved = None;
    let started = (|| {
        let mut writer = child
            .stdin
            .take()
            .ok_or_else(|| failed("control_spawn_failed", "child has no input pipe"))?;
        let mut reader = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| failed("control_spawn_failed", "child has no output pipe"))?,
        );
        let entry = Entry {
            program: PathBuf::from(&harness.path),
            arguments,
            directory: PathBuf::from(&launch.directory),
        };
        serde_json::to_writer(&mut writer, &entry)
            .map_err(|error| failed("control_entry_failed", error))?;
        writer
            .write_all(b"\n")
            .and_then(|()| writer.flush())
            .map_err(|error| failed("control_entry_failed", error))?;
        let ready: Ready = serde_json::from_value(frame(&mut reader)?).map_err(|error| {
            failed(
                "control_process_unproved",
                format!("entry identity frame is invalid: {error}"),
            )
        })?;
        let pid = child.id();
        let native = Pid::from_raw(
            i32::try_from(pid).map_err(|error| failed("control_process_unproved", error))?,
        )
        .ok_or_else(|| failed("control_process_unproved", "child pid is invalid"))?;
        if ready.pid != pid
            || ready.executable != harness
            || getsid(Some(native)).map_err(|error| failed("control_process_unproved", error))?
                != native
            || getpgid(Some(native)).map_err(|error| failed("control_process_unproved", error))?
                != native
        {
            return Err(failed(
                "control_process_unproved",
                "entry does not own its proved process session and group",
            ));
        }
        let leader = Leader {
            pid,
            start: crate::peer::start_identity(pid)?,
        };
        proved = Some(leader.clone());
        writer
            .write_all(b"{\"execute\":true}\n")
            .and_then(|()| writer.flush())
            .map_err(|error| failed("control_entry_failed", error))?;
        let binding = Binding {
            session: launch.session.clone(),
            generation,
            leader,
            conversation: conversation.to_owned(),
            entry: entry_identity,
            harness,
            harness_version,
            adapter,
        };
        Ok((reader, writer, binding))
    })();
    match started {
        Ok((reader, writer, binding)) => Ok(Spawned {
            reader: Box::new(reader),
            writer: Box::new(writer),
            child: Box::new(PipeChild {
                child,
                leader: binding.leader.clone(),
            }),
            binding,
        }),
        Err(error) => {
            if child
                .try_wait()
                .map_err(|wait| failed("control_spawn_cleanup_failed", wait))?
                .is_none()
            {
                if let Some(leader) = &proved {
                    crate::pty::end(leader)
                        .map_err(|kill| failed("control_spawn_cleanup_failed", kill))?;
                } else {
                    child
                        .kill()
                        .map_err(|kill| failed("control_spawn_cleanup_failed", kill))?;
                }
            }
            child
                .wait()
                .map_err(|wait| failed("control_spawn_cleanup_failed", wait))?;
            Err(error)
        }
    }
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

fn report_version(
    program: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<(std::process::ExitStatus, Vec<u8>), RunnerError> {
    let mut child = Command::new(program)
        .arg("--version")
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| failed("control_adapter_unqualified", error))?;
    let read: Result<Vec<u8>, RunnerError> = (|| {
        let mut bytes = Vec::new();
        let mut stdout = child.stdout.take().ok_or_else(|| {
            failed(
                "control_adapter_unqualified",
                "version probe has no output pipe",
            )
        })?;
        stdout
            .read_to_end(&mut bytes)
            .map_err(|error| failed("control_adapter_unqualified", error))?;
        Ok(bytes)
    })();
    if read.is_err()
        && child
            .try_wait()
            .map_err(|error| failed("control_probe_cleanup_failed", error))?
            .is_none()
    {
        child
            .kill()
            .map_err(|error| failed("control_probe_cleanup_failed", error))?;
    }
    let status = child
        .wait()
        .map_err(|error| failed("control_probe_cleanup_failed", error))?;
    Ok((status, read?))
}

#[cfg(test)]
mod startup_tests {
    use super::{execute_permission, startup_frame};
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn exec_after_startup(entry: bool) -> Result<(), Box<dyn std::error::Error>> {
        let (mut reader, mut writer) = std::io::pipe()?;
        let first = b"{\"id\":\"lys-initialize\",\"method\":\"initialize\"}\n";
        let mut bytes = Vec::new();
        if entry {
            bytes.extend_from_slice(b"{\"startup\":true}\n");
        }
        bytes.extend_from_slice(b"{\"execute\":true}\n");
        bytes.extend_from_slice(first);
        writer.write_all(&bytes)?;
        drop(writer);
        if entry {
            assert_eq!(
                startup_frame(&mut reader)?,
                serde_json::json!({"startup":true})
            );
        }
        execute_permission(&mut reader)?;
        let output = Command::new("/bin/cat")
            .stdin(reader)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert_eq!(output.stdout, first);
        Ok(())
    }

    #[test]
    fn startup_permission_preserves_the_first_frame_for_the_executed_program()
    -> Result<(), Box<dyn std::error::Error>> {
        exec_after_startup(false)
    }

    #[test]
    fn startup_entry_takes_only_its_own_line_before_permission_and_exec()
    -> Result<(), Box<dyn std::error::Error>> {
        exec_after_startup(true)
    }
}
