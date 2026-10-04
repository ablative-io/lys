//! `lys runner join`: this computer joins Lys as the runner of a computer
//! Lys names, with the connection code read from standard input, then
//! serves and dials as `lys runner serve` and `lys runner dial` do, for as
//! long as the command runs. Every act is `lys-runner`'s.
//!
//! The files are kept where `lys identity install` keeps its own runner's:
//! the socket and the server's public key in the run folder, the held
//! sessions and this computer's key in the data folder.

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_runner::dial::Dial;
use lys_runner::dial::join::{Join, reachable};
use lys_runner::protocol::hex;
use lys_runner::{Options, Runner, RunnerError};
use zeroize::Zeroizing;

use crate::commands::error::{CliError, CliResult};
use crate::identity::install::layout::Layout;
use crate::identity::private_files;

/// This computer's own key, in the data folder: a raw 32-byte Ed25519 seed.
pub const MACHINE_KEY: &str = "runner-machine.key";

/// The server's public key, in the run folder, where the install keeps it.
pub const SERVER_KEY: &str = "runner-server.pub";

/// The refusal when standard input gives no connection code.
pub const CODE_MISSING: &str = "runner_join_code_missing";

fn io(context: String) -> impl FnOnce(std::io::Error) -> CliError {
    move |source| CliError::Io { context, source }
}

/// Runs `lys runner join`.
pub fn run(
    server: &str,
    machine: &str,
    server_ca: Option<PathBuf>,
    scrollback: usize,
) -> CliResult<()> {
    reachable(server)?;
    let code = read_code(&mut std::io::stdin().lock(), std::io::stdin().is_terminal())?;
    let layout = Layout::discover()?;
    for directory in [layout.run_dir(), layout.data_dir()] {
        private_files::ensure_dir(&directory)?;
    }
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &layout.data_dir().join(MACHINE_KEY),
    )?);
    let server_key = Join {
        server,
        authority: server_ca.as_deref(),
        machine,
        code: &code,
        key: key.public_key_bytes(),
    }
    .join()?;
    drop(code);
    private_files::write(
        &layout.run_dir().join(SERVER_KEY),
        hex(&server_key).as_bytes(),
    )?;
    let mut out = std::io::stdout();
    writeln!(
        out,
        "This computer joined Lys at {server} as computer {machine}.\n\
         Its runner runs while this command runs. Stop it with Control-C, which ends every session it holds."
    )
    .and_then(|()| out.flush())
    .map_err(io("writing that this computer joined".to_owned()))?;
    let dial = Dial {
        server: server.to_owned(),
        machine: machine.to_owned(),
        key,
        socket: layout.runner_socket(),
        authority: server_ca,
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
/// runner is asked to stop as a stop signal asks it, ending every session
/// it holds, and the dial's refusal is answered.
fn serve_and_dial(options: &Options, dial: Dial) -> CliResult<()> {
    let runner = Runner::open(options)?;
    let id = runner.sessions().runner().to_owned();
    let ended: Arc<Mutex<Option<RunnerError>>> = Arc::default();
    let kept = Arc::clone(&ended);
    let socket = options.socket.clone();
    let mut said = Ok(());
    runner.serve_until_stopped(|| {
        let mut out = std::io::stdout();
        said = writeln!(out, "listening {} as runner {id}", socket.display())
            .and_then(|()| out.flush());
        std::thread::spawn(move || {
            let error = match dial.bridge() {
                Ok(()) => RunnerError::Dial {
                    reason: "the connection to Lys ended".to_owned(),
                },
                Err(error) => error,
            };
            lys_runner::error::said(&format!(
                "the connection to Lys ended: {error}: stopping the runner"
            ));
            match kept.lock() {
                Ok(mut held) => *held = Some(error),
                Err(poisoned) => *poisoned.into_inner() = Some(error),
            }
            if let Err(error) = rustix::process::kill_process(
                rustix::process::getpid(),
                rustix::process::Signal::TERM,
            ) {
                lys_runner::error::said(&format!("the runner could not be asked to stop: {error}"));
            }
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

#[cfg(test)]
#[path = "runner_join_tests.rs"]
mod tests;
