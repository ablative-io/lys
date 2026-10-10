//! A fixture for the AGENTS-004 survival tests: the real owner steps with no
//! harness, and a runner that starts one and then waits to be killed.
//!
//! As `runner seat-owner --dir D --server-key K --scrollback N` it reads the
//! plan, records its establish intent, opens a runner of its own on the
//! owner socket, adopts its owner state and writes the ready line, exactly
//! as `lys runner seat-owner` does short of starting the managed session.
//! As `--as-runner STATE SESSION KEY_HEX` it is the runner: it starts an
//! owner for a supervised binding through `Sessions::start_owned`, prints
//! the owned seat as JSON on standard output and blocks on standard input
//! until the test kills it, which is how the test proves the owner outlives
//! the process that started it.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use lys_runner::harness_control::{ManagedLaunch, Transport};
use lys_runner::peer::{self, Leader};
use lys_runner::protocol::{Launch, unhex};
use lys_runner::seat_owner::process::{OwnerState, establish};
use lys_runner::seat_owner::protocol::{OwnerBinding, RUNNER_STATE, SOCKET, ready_line};
use lys_runner::seat_owner::spawn::OwnerPlan;
use lys_runner::seat_owner::store::OwnerStore;
use lys_runner::session::Sessions;
use lys_runner::{Options, Runner, RunnerError};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("runner") => owner(&args),
        Some("--as-runner") => as_runner(&args),
        _ => Err(RunnerError::refused(
            "fixture_args_invalid",
            "use: runner seat-owner --dir D --server-key K --scrollback N | --as-runner STATE SESSION KEY_HEX",
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn option(args: &[String], name: &str) -> Result<String, RunnerError> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|at| args.get(at + 1).cloned())
        .ok_or_else(|| RunnerError::refused("fixture_args_invalid", format!("{name} is needed")))
}

fn key_from_hex(hex: &str) -> Result<[u8; 32], RunnerError> {
    unhex(hex.trim())
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| {
            RunnerError::refused(
                "fixture_args_invalid",
                "the key is 64 hexadecimal characters",
            )
        })
}

fn owner(args: &[String]) -> Result<(), RunnerError> {
    let dir = PathBuf::from(option(args, "--dir")?);
    let key_path = option(args, "--server-key")?;
    let server_key = key_from_hex(&std::fs::read_to_string(&key_path).map_err(|error| {
        RunnerError::refused(
            "fixture_args_invalid",
            format!("reading {key_path}: {error}"),
        )
    })?)?;
    let scrollback: usize = option(args, "--scrollback")?
        .parse()
        .map_err(|_| RunnerError::refused("fixture_args_invalid", "scrollback is a number"))?;
    let plan = OwnerPlan::read(&dir)?;
    let pid = std::process::id();
    let own = Leader {
        pid,
        start: peer::start_identity(pid)?,
    };
    let mut store = OwnerStore::open(&dir)?;
    establish(&mut store, &plan, &own)?;
    let socket = dir.join(SOCKET);
    let runner = Runner::open(&Options {
        socket: socket.clone(),
        state: dir.join(RUNNER_STATE),
        server_key,
        scrollback,
    })?;
    let sessions = Arc::clone(runner.sessions());
    let runner_id = sessions.runner().to_owned();
    sessions.adopt_owner(Arc::new(OwnerState::new(
        store,
        plan.binding.clone(),
        own.clone(),
        runner_id.clone(),
        socket.clone(),
    )))?;
    // The build this fake owner announces: `<state>/owner-build` when the
    // test wrote one, so a seat started after an "upgrade" names the new
    // build while an earlier seat keeps its owner and its build.
    let build = socket
        .ancestors()
        .nth(3)
        .map(|state| state.join("owner-build"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map_or_else(|| "fake".to_owned(), |build| build.trim().to_owned());
    let ready = ready_line(&runner_id, &socket, &own, &build);
    runner.serve_until_stopped(move || {
        let mut out = std::io::stdout();
        if let Err(error) = writeln!(out, "{ready}").and_then(|()| out.flush()) {
            eprintln!("fixture_ready_unwritten: {error}");
        }
    })
}

/// The binding and launch the tests use, by session name.
#[must_use]
pub fn supervised(session: &str) -> ManagedLaunch {
    ManagedLaunch {
        launch: Launch {
            session: session.to_owned(),
            program: "claude".to_owned(),
            arguments: Vec::new(),
            directory: "/".to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 120,
            rows: 40,
            rotation: None,
            policy: None,
        },
        transport: Transport::Claude,
        conversation: "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f".to_owned(),
        requires_controls: false,
        owner: Some(OwnerBinding {
            seat: format!("seat-{session}"),
            session: session.to_owned(),
            conversation: "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f".to_owned(),
            generation: 1,
        }),
    }
}

fn as_runner(args: &[String]) -> Result<(), RunnerError> {
    let state = PathBuf::from(args.get(1).cloned().unwrap_or_default());
    let session = args.get(2).cloned().unwrap_or_default();
    let key = key_from_hex(args.get(3).map(String::as_str).unwrap_or_default())?;
    let sessions = Sessions::open(&state, 1 << 16)?;
    sessions.set_server_key(key);
    sessions.set_owner_program(
        std::env::current_exe()
            .map_err(|error| RunnerError::refused("fixture_args_invalid", error.to_string()))?,
    );
    let managed = supervised(&session);
    let binding = managed.owner.clone().ok_or_else(|| {
        RunnerError::refused("fixture_args_invalid", "the fixture launch is supervised")
    })?;
    sessions.start_owned(managed, binding, None)?;
    let seat = sessions.owned_seat(&session)?.ok_or_else(|| {
        RunnerError::refused("fixture_owner_missing", "the owner was not indexed")
    })?;
    let line = serde_json::to_string(&seat)
        .map_err(|error| RunnerError::refused("fixture_json", error.to_string()))?;
    let mut out = std::io::stdout();
    writeln!(out, "{line}")
        .and_then(|()| out.flush())
        .map_err(|error| RunnerError::refused("fixture_stdout", error.to_string()))?;
    // Block until killed: the test proves the owner outlives this runner.
    let mut sink = Vec::new();
    std::io::stdin()
        .read_to_end(&mut sink)
        .map_err(|error| RunnerError::refused("fixture_stdin", error.to_string()))?;
    Ok(())
}
