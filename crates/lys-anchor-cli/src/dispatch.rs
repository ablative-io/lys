//! Argument parsing, dispatch, and the mapping from an outcome to an exit code.
//!
//! Parse-and-dispatch only. Every arm forwards to a subcommand module and does
//! nothing else — a decision made here would be a decision no library consumer
//! of `lys-anchor` gets.

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::commands;

/// Parses `std::env::args`, dispatches, and translates the outcome into an exit
/// code. Every failure path prints a diagnostic to stderr.
///
/// Exits the process on an argument-parsing error, which is clap's behaviour and
/// its exit code 2.
#[must_use]
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let result = match &cli.command {
        Command::Init {
            dir,
            origin,
            key,
            genesis,
            admission,
        } => commands::anchor::init::run(dir, origin, key, genesis, admission, json),
        Command::Status {
            dir,
            key,
            admission,
        } => commands::anchor::status::run(dir, key, admission, json),
        Command::Checkpoint {
            dir,
            key,
            out,
            admission,
        } => commands::anchor::checkpoint::run(dir, key, out, admission, json),
        Command::Prove {
            dir,
            key,
            leaf_index,
            out,
            admission,
        } => commands::anchor::prove::run(dir, key, *leaf_index, out, admission, json),
        #[cfg(feature = "unstable-anchor")]
        Command::Submit {
            dir,
            key,
            statement,
            credential,
            receipt_out,
            artifact_out,
            admission,
        } => commands::anchor::submit::run(
            commands::anchor::submit::SubmitPaths {
                dir,
                key,
                statement,
                credential: credential.as_deref(),
                receipt_out,
                artifact_out,
            },
            admission,
            json,
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // The diagnostic always goes to stderr, so an operator watching a
            // terminal sees it in the usual place. Under `--json` the failure is
            // ALSO emitted as an object on stdout: a caller that asked for
            // parseable output must not receive unparseable output at exactly
            // the moment it matters most. The message is the CLI's existing
            // text — JSON mode reformats, it never widens, and in particular it
            // never adds detail to an admission refusal that carries none.
            eprintln!("error: {error}");
            if json {
                commands::output::emit_json_error(&error.to_string());
            }
            ExitCode::FAILURE
        }
    }
}
