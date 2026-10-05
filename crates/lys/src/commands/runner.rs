//! `lys runner`: parse, then hand to `lys-runner`, which holds all of the
//! runner's logic.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::dial::Dial;
use lys_runner::protocol::unhex;
use lys_runner::{Options, Runner, RunnerError};

use crate::cli::{JudgeHarness, RunnerCommand};

use crate::commands::error::CliResult;

/// Runs `lys runner`.
pub fn run(command: RunnerCommand) -> CliResult<()> {
    match command {
        RunnerCommand::Serve {
            socket,
            state,
            server_key,
            scrollback,
            proxy_state,
        } => {
            let runner = Runner::open(&Options {
                server_key: public_key(&server_key)?,
                socket: socket.clone(),
                state,
                scrollback,
            })?;
            if let Some(proxy_state) = &proxy_state {
                runner.sessions().proxy_state(proxy_state);
            }
            serve(runner, &socket)
        }

        RunnerCommand::Dial {
            socket,
            server,
            machine,
            machine_key,
            server_ca,
        } => {
            let key = Arc::new(Ed25519Identity::load(&machine_key)?);
            Dial {
                server,
                machine,
                key,
                socket,
                authority: server_ca,
            }
            .bridge()?;
            Ok(())
        }
        RunnerCommand::Join {
            server,
            machine,
            server_ca,
            scrollback,
        } => crate::commands::runner_join::run(&server, &machine, server_ca, scrollback),
        RunnerCommand::Judge { socket, harness } => judge(&socket, harness),
        RunnerCommand::StatusLine { socket } => status_line(&socket),
    }
}

/// Serve `runner` on `socket` until it is asked to stop, saying once it
/// listens; the line a starter waits for.
fn serve(runner: Runner, socket: &Path) -> CliResult<()> {
    let id = runner.sessions().runner().to_owned();
    let mut said = Ok(());
    runner.serve_until_stopped(|| {
        let mut out = std::io::stdout();
        said = writeln!(out, "listening {} as runner {id}", socket.display())
            .and_then(|()| out.flush());
        if let Err(error) = &said {
            lys_runner::error::said(&format!(
                "listening on {}, and that could not be written: {error}",
                socket.display()
            ));
        }
    })?;
    said.map_err(|source| crate::commands::error::CliError::Io {
        context: "writing that the runner listens".to_owned(),
        source,
    })
}

/// Runs `lys runner status-line`: one line for the harness to show. The
/// command exits zero whatever happened, so the harness shows the line that
/// says what did.
fn status_line(socket: &Path) -> CliResult<()> {
    let mut stdin = String::new();
    let handed = std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin)
        .map_err(|error| RunnerError::Malformed {
            reason: format!("the status line's input could not be read: {error}"),
        })
        .and_then(|_| lys_runner::status_line::hand(socket, &stdin));
    let mut out = std::io::stdout();
    writeln!(out, "{}", lys_runner::status_line::shown(&handed))
        .and_then(|()| out.flush())
        .map_err(|source| crate::commands::error::CliError::Io {
            context: "writing the status line".to_owned(),
            source,
        })
}

/// Runs `lys runner judge`: the hook's answer, a deny on any failure.
fn judge(socket: &Path, harness: JudgeHarness) -> CliResult<()> {
    let mut stdin = String::new();
    let answer = match std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin) {
        Ok(_) => match harness {
            JudgeHarness::Claude => lys_runner::claude_judge::output(socket, &stdin),
            JudgeHarness::Codex => lys_runner::codex_judge_client::output(socket, &stdin),
        },
        Err(error) => lys_runner::codex_judge::response(&lys_runner::refusals::Verdict::denied(
            "judge_input_unread",
            format!("Lys could not read the hook's input: {error}"),
            "not_attributed",
        )),
    };
    let mut out = std::io::stdout();
    writeln!(out, "{answer}")
        .and_then(|()| out.flush())
        .map_err(|source| crate::commands::error::CliError::Io {
            context: "writing the judge's answer".to_owned(),
            source,
        })
}

/// The server's public key, read from the hex in `path`.
fn public_key(path: &Path) -> Result<[u8; 32], RunnerError> {
    let text = std::fs::read_to_string(path).map_err(|error| RunnerError::Key {
        reason: format!("reading {}: {error}", path.display()),
    })?;
    unhex(text.trim())
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| RunnerError::Key {
            reason: format!(
                "{} does not hold a public key as 64 hexadecimal characters",
                path.display()
            ),
        })
}
