//! `lys runner join`: this computer joins Lys as the runner of a computer
//! Lys names, with the connection code read from standard input, then
//! serves and dials as `lys runner serve` and `lys runner dial` do, for as
//! long as the command runs. Every act is `lys-runner`'s.
//!
//! The files are kept where `lys identity install` keeps its own runner's:
//! the socket and the server's public key in the run folder, the held
//! sessions in the data folder and this computer's key in the keys folder.

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lys_runner::dial::Dial;
use lys_runner::dial::join::{Join, reachable};
use lys_runner::protocol::hex;
use lys_runner::{Options, Runner, RunnerError};
use zeroize::Zeroizing;

use crate::commands::error::{CliError, CliResult};
use crate::identity::install::layout::Layout;
use crate::identity::private_files;

/// The private key's file name: a raw 32-byte Ed25519 seed.
pub const MACHINE_KEY: &str = "runner-machine.key";

/// The server's public key, in the run folder, where the install keeps it.
pub const SERVER_KEY: &str = "runner-server.pub";

/// The refusal when standard input gives no connection code.
pub const CODE_MISSING: &str = "runner_join_code_missing";

/// The connection kept after admission, without its single-use code.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedJoin {
    pub(crate) server: String,
    pub(crate) machine: String,
    pub(crate) server_ca: Option<PathBuf>,
}

impl SavedJoin {
    pub(crate) fn validate(&self) -> CliResult<()> {
        // An address another computer cannot dial (not https, or this computer's own loopback) is refused by its own
        // name first, in the words a person reads at join.
        lys_runner::dial::join::reachable(&self.server)?;
        let address = self.server.strip_prefix("https://").ok_or_else(|| {
            RunnerError::refused("runner_join_address_invalid", "the server must use HTTPS")
        })?;
        let authority = address.split('/').next().unwrap_or_default();
        if authority.is_empty()
            || authority.contains('@')
            || self.server.bytes().any(|byte| byte <= b' ' || byte == 127)
            || self.server.contains(['?', '#'])
            || self.machine.is_empty()
            || self.machine.len() > 128
            || !self
                .machine
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(RunnerError::refused("runner_join_invalid", "the server address or computer id is invalid; download a fresh installer from Network").into());
        }
        reachable(&self.server)?;
        if self
            .server_ca
            .as_ref()
            .is_some_and(|path| !path.is_absolute() || !path.is_file())
        {
            return Err(RunnerError::refused(
                "runner_join_authority_invalid",
                "the saved certificate authority must name an existing absolute file",
            )
            .into());
        }
        Ok(())
    }

    pub(crate) fn save(&self, layout: &Layout) -> CliResult<()> {
        self.validate()?;
        private_files::ensure_dir(&layout.run_dir())?;
        let bytes = serde_json::to_vec(self).map_err(|source| CliError::JsonSerialize {
            what: "saved runner connection",
            source,
        })?;
        private_files::write(&layout.run_dir().join("runner-join.json"), &bytes)?;
        Ok(())
    }

    pub(crate) fn read(layout: &Layout) -> CliResult<Self> {
        let bytes =
            private_files::read(&layout.run_dir().join("runner-join.json"))?.ok_or_else(|| {
                RunnerError::refused(
                    "runner_join_missing",
                    "this computer has no saved connection; open its installer from Network",
                )
            })?;
        let saved: Self = serde_json::from_slice(&bytes).map_err(|error| {
            RunnerError::refused(
                "runner_join_invalid",
                format!("the saved runner connection is invalid; open a fresh installer from Network ({})", super::runner::service::json_detail(&error)),
            )
        })?;
        saved.validate()?;
        Ok(saved)
    }
}

fn io(context: String) -> impl FnOnce(std::io::Error) -> CliError {
    move |source| CliError::Io { context, source }
}

/// Runs `lys runner join`.
///
/// # Errors
/// Returns named connection, join-code or runner-start failures.
pub fn run(
    server: &str,
    machine: &str,
    server_ca: Option<PathBuf>,
    scrollback: usize,
) -> CliResult<()> {
    let server_ca = server_ca
        .map(std::fs::canonicalize)
        .transpose()
        .map_err(io("reading the server certificate authority".to_owned()))?;
    let saved = SavedJoin {
        server: server.trim_end_matches('/').to_owned(),
        machine: machine.to_owned(),
        server_ca,
    };
    saved.validate()?;
    let code = read_code(&mut std::io::stdin().lock(), std::io::stdin().is_terminal())?;
    let layout = Layout::discover()?;
    join(&layout, &saved, code.as_str())?;
    drop(code);
    let mut out = std::io::stdout();
    writeln!(
        out,
        "This computer joined Lys. Its runner runs until this command is stopped."
    )
    .and_then(|()| out.flush())
    .map_err(io("writing that this computer joined".to_owned()))?;
    start_saved(&layout, scrollback)
}

/// Admit once, then keep only the connection and the existing key locations.
pub(crate) fn join(layout: &Layout, saved: &SavedJoin, code: &str) -> CliResult<()> {
    saved.validate()?;
    if code.trim().is_empty() {
        return Err(RunnerError::refused(CODE_MISSING, "the connection code is empty").into());
    }
    for directory in [layout.run_dir(), layout.data_dir()] {
        private_files::ensure_dir(&directory)?;
    }
    let key = Arc::new(crate::identity::install::layout::runner_machine::provide(
        layout,
        &mut lys_runner::error::said,
    )?);
    let server_key = Join {
        server: &saved.server,
        authority: saved.server_ca.as_deref(),
        machine: &saved.machine,
        code,
        key: key.public_key_bytes(),
    }
    .join()
    .map_err(|error| {
        RunnerError::refused(
            &error.name(),
            error.to_string().replace(code, "[connection code removed]"),
        )
    })?;
    private_files::write(
        &layout.run_dir().join(SERVER_KEY),
        hex(&server_key).as_bytes(),
    )?;
    saved.save(layout)
}

/// Start from the saved connection; starting never generates a replacement key.
pub(crate) fn start_saved(layout: &Layout, scrollback: usize) -> CliResult<()> {
    let saved = SavedJoin::read(layout)?;
    let key = Arc::new(lys_core::Ed25519Identity::load(
        &layout.runner_machine_key(),
    )?);
    let server_key = super::runner::public_key(&layout.run_dir().join(SERVER_KEY))?;
    let dial = Dial {
        server: saved.server,
        machine: saved.machine,
        key,
        socket: layout.runner_socket(),
        authority: saved.server_ca,
    };
    serve_and_dial(
        &Options {
            server_key,
            socket: layout.runner_socket(),
            state: layout.data_dir().join("runner"),
            scrollback,
        },
        dial,
    )
}

/// The connection code: one line of `input`, asked for on standard error
/// when `asking`, since standard input is then a person at a terminal.
/// Refused `runner_join_code_missing` when the line is empty.
///
/// # Errors
/// Returns named connection, join-code or runner-start failures.
pub fn read_code(input: &mut impl BufRead, asking: bool) -> CliResult<Zeroizing<String>> {
    if asking {
        let mut said = std::io::stderr();
        write!(
            said,
            "Paste the connection code Lys gave for this computer, then press Return: "
        )
        .and_then(|()| said.flush())
        .map_err(io("asking for the connection code".to_owned()))?;
    }
    let mut line = Zeroizing::new(String::new());
    input.read_line(&mut line).map_err(io(
        "reading the connection code from standard input".to_owned()
    ))?;
    let code = Zeroizing::new(line.trim().to_owned());
    if code.is_empty() {
        return Err(RunnerError::refused(
            CODE_MISSING,
            "no connection code was read from standard input: paste the code Lys gave for this computer",
        )
        .into());
    }
    Ok(code)
}

/// Serve as `lys runner serve` does, and once the runner listens, dial as
/// `lys runner dial` does on a thread of its own. When the dial ends, the
/// runner keeps its sessions. Only an explicit console stop ends them; a
/// dependency or admission failure is recorded for the next console read.
fn serve_and_dial(options: &Options, dial: Dial) -> CliResult<()> {
    let runner = Runner::open(options)?;
    let id = runner.sessions().runner().to_owned();
    let ended: Arc<Mutex<Option<RunnerError>>> = Arc::default();
    let kept = Arc::clone(&ended);
    let socket = options.socket.clone();
    let mut said = Ok(());
    runner.serve_until_stopped(|| {
        let mut out = std::io::stdout();
        said = writeln!(
            out,
            "listening {} as runner {id}, ready in {} ms",
            socket.display(),
            crate::commands::started::elapsed_ms()
        )
        .and_then(|()| writeln!(out, "runner_service_ready {}", env!("LYS_BUILD")))
        .and_then(|()| out.flush());
        std::thread::spawn(move || {
            let error = match dial.bridge() {
                Ok(()) => RunnerError::Dial {
                    reason: "the connection to Lys ended".to_owned(),
                },
                Err(error) => error,
            };
            lys_runner::error::said(&format!(
                "runner_dependency_unavailable: the connection to Lys ended: {error}; the runner and its sessions remain alive; restore the dependency or reconnect this computer"
            ));
            dial_ended(&kept, error);
        });
    })?;
    said.map_err(io("writing that the runner listens".to_owned()))?;
    let ended = match ended.lock() {
        Ok(mut held) => held.take(),
        Err(poisoned) => poisoned.into_inner().take(),
    };
    match ended {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

fn dial_ended(held: &Mutex<Option<RunnerError>>, error: RunnerError) {
    match held.lock() {
        Ok(mut held) => *held = Some(error),
        Err(poisoned) => *poisoned.into_inner() = Some(error),
    }
}

/// A login service keeps the parent alive while its serving child starts.
/// A complete ready line admits the new build; pipe closure and exit refuse it.
#[cfg(unix)]
pub(crate) fn supervise(layout: &Layout) -> CliResult<()> {
    use std::process::{Command, Stdio};
    let mut command = Command::new(layout.binary("lys"));
    command
        .args(["runner", "start", "--worker"])
        .stdin(Stdio::null());
    supervise_command(layout, &mut command, &mut |commit| {
        crate::commands::runner::update::ready(layout, commit)
    })
}

#[cfg(unix)]
pub(crate) fn supervise_command(
    layout: &Layout,
    command: &mut std::process::Command,
    confirm: &mut dyn FnMut(&str) -> CliResult<()>,
) -> CliResult<()> {
    use crate::commands::runner::service;
    use std::process::Stdio;
    use tokio::io::unix::AsyncFd;
    use tokio::signal::unix::{SignalKind, signal};

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .map_err(|error| service::refused("runner_service_runtime_failed", error.to_string()))?;
    runtime.block_on(async {
        let held = |kind| signal(kind).map_err(|error| service::refused("runner_service_signal_failed", error.to_string()));
        let (mut term, mut interrupt, mut hangup, mut exited) = (
            held(SignalKind::terminate())?, held(SignalKind::interrupt())?,
            held(SignalKind::hangup())?, held(SignalKind::child())?,
        );
        let mut child = command.stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()
            .map_err(|error| service::io("start runner", &layout.binary("lys"), error))?;
        let native = i32::try_from(child.id()).ok().and_then(rustix::process::Pid::from_raw);
        let result: CliResult<()> = async {
            let output = child.stdout.take().ok_or_else(|| {
                service::refused("runner_service_output_failed", "the runner's readiness pipe is missing")
            })?;
            let flags = rustix::fs::fcntl_getfl(&output)
                .map_err(|error| service::refused("runner_service_output_failed", error.to_string()))?;
            rustix::fs::fcntl_setfl(&output, flags | rustix::fs::OFlags::NONBLOCK)
                .map_err(|error| service::refused("runner_service_output_failed", error.to_string()))?;
            let mut output = AsyncFd::new(output)
                .map_err(|error| service::io("watch runner output", &layout.binary("lys"), error))?;
            let mut pending = Vec::new();
            let (mut ready, mut stopping, mut pipe_open) = (false, false, true);
            loop {
                let requested = tokio::select! {
                    line = worker_line(&mut output, &mut pending), if pipe_open => {
                        if let Some(line) = line? {
                                writeln!(std::io::stdout().lock(), "{line}")
                                    .map_err(|error| service::io("write runner output", &layout.binary("lys"), error))?;
                                if !ready && let Some(commit) = line.strip_prefix("runner_service_ready ").filter(|commit| !commit.is_empty()) {
                                    confirm(commit)?;
                                    ready = true;
                                }
                        } else {
                                pipe_open = false;
                                if !ready && !stopping {
                                    break Err(service::refused("runner_service_unready", "the runner closed its readiness pipe before reporting ready"));
                                }
                        }
                        None
                    }
                    _ = exited.recv() => {
                        match child.try_wait() {
                            Ok(Some(status)) => break if stopping { Ok(()) } else {
                                Err(service::refused("runner_service_exited", format!("the runner exited ({status})")))
                            },
                            Ok(None) => {},
                            Err(error) => break Err(service::io("read runner exit", &layout.binary("lys"), error)),
                        }
                        None
                    }
                    _ = term.recv() => Some(rustix::process::Signal::TERM),
                    _ = interrupt.recv() => Some(rustix::process::Signal::INT),
                    _ = hangup.recv() => Some(rustix::process::Signal::HUP),
                };
                if let Some(requested) = requested {
                    let Some(pid) = native else {
                        break Err(service::refused("runner_service_pid_invalid", "the runner has no valid process id"));
                    };
                    match rustix::process::kill_process(pid, requested) {
                        Ok(()) | Err(rustix::io::Errno::SRCH) => stopping = true,
                        Err(error) => break Err(service::refused("runner_service_stop_failed", error.to_string())),
                    }
                }
            }
        }.await;
        // This handle owns the child until wait collects it, including error paths.
        let cleanup = match child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => child.kill().and_then(|()| child.wait().map(|_| ())),
            Err(error) => Err(error),
        };
        if let Err(error) = cleanup {
            return Err(service::refused("runner_service_cleanup_failed", format!("{error}; runner outcome: {}", result.as_ref().err().map_or_else(|| "stopped".to_owned(), ToString::to_string))));
        }
        result
    })
}

/// Preserve partial lines across signal handling; only a newline completes readiness.
#[cfg(unix)]
async fn worker_line(
    output: &mut tokio::io::unix::AsyncFd<std::process::ChildStdout>,
    pending: &mut Vec<u8>,
) -> CliResult<Option<String>> {
    use crate::commands::runner::service;
    use std::io::Read;

    loop {
        if let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<_> = pending.drain(..=end).collect();
            line.truncate(end);
            return String::from_utf8(line).map(Some).map_err(|error| {
                service::refused(
                    "runner_service_output_failed",
                    format!("the runner output is not UTF-8: {}", error.utf8_error()),
                )
            });
        }
        let mut bytes = [0_u8; 4096];
        let mut readable = output
            .readable_mut()
            .await
            .map_err(|error| service::refused("runner_service_output_failed", error.to_string()))?;
        match readable.try_io(|output| output.get_mut().read(&mut bytes)) {
            Ok(Ok(0)) => return Ok(None),
            Ok(Ok(count)) => pending.extend_from_slice(&bytes[..count]),
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Ok(Err(error)) => {
                return Err(service::refused(
                    "runner_service_output_failed",
                    error.to_string(),
                ));
            }
            Err(_) => {}
        }
    }
}

#[cfg(not(unix))]
pub(crate) fn supervise(layout: &Layout) -> CliResult<()> {
    Err(super::runner::service::refused(
        "runner_service_platform_unsupported",
        format!(
            "runner services at {} need macOS or Linux",
            layout.root.display()
        ),
    ))
}

#[cfg(test)]
#[path = "runner_join_tests.rs"]
mod tests;
