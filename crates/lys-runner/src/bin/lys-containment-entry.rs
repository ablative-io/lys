//! Single-threaded trusted Seatbelt entry; no model, listener or fallback process.

use std::process::ExitCode;

use lys_runner::RunnerError;
use lys_runner::containment_entry::{Input, exec};

fn run() -> Result<std::convert::Infallible, RunnerError> {
    let mut arguments = std::env::args_os().skip(1);
    let input = arguments.next().ok_or_else(|| {
        RunnerError::refused(
            "containment_entry_refused",
            "missing structured launch input",
        )
    })?;
    if arguments.next().is_some() {
        return Err(RunnerError::refused(
            "containment_entry_refused",
            "unexpected extra launch argument",
        ));
    }
    let text = input.to_str().ok_or_else(|| {
        RunnerError::refused("containment_entry_refused", "launch input is not UTF-8")
    })?;
    let input: Input = serde_json::from_str(text).map_err(|error| {
        RunnerError::refused(
            "containment_entry_refused",
            format!("launch input: {error}"),
        )
    })?;
    exec(&input)
}

fn main() -> ExitCode {
    match run() {
        Ok(never) => match never {},
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
