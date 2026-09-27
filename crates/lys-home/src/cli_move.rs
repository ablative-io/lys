//! `lys-home ship --home <dir> --remote <path>` and `lys-home fetch
//! --remote <path> --home <dir>` (HOME-019 R5, R6): each prints one JSON
//! report of commits, paths, session ids and execution ids, never a line of
//! a session, an entry's data or a block's bytes. A refusal is printed on
//! stderr by the binary with nothing on stdout and exits 1, except two that
//! are reports: ship's `stale_index` and fetch's `verification_failed`,
//! printed on stdout with exit status 1. The remote is resolved against the
//! current directory and git runs with PATH from the process environment
//! ([`crate::home_move::git`]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;

use crate::cli::given::Outcome;
use crate::error::HomeError;
use crate::home_move::fetch::{Fetched, fetch, refusal_report};
use crate::home_move::ship::{Shipped, ship};

/// The exit status of a refusal printed as a report.
const STATUS_REFUSED: i32 = 1;

/// The arguments of `ship`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct ShipArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The remote: a path on this machine, where a bare repository is or will be made.
    #[arg(long)]
    pub remote: String,
}

/// The arguments of `fetch`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct FetchArgs {
    /// The remote: a path on this machine holding the shipped ref.
    #[arg(long)]
    pub remote: String,
    /// The new home directory: absent, or an empty directory.
    #[arg(long)]
    pub home: PathBuf,
}

/// Run `ship` and return its report with the status the process exits with.
pub fn run_ship(args: &ShipArgs) -> Result<Outcome, HomeError> {
    let base = current_dir()?;
    let inherited: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    match ship(&args.home, &args.remote, &base, &inherited)? {
        Shipped::Pushed(report) => Ok(Outcome::done(
            json!({"command": "ship", "report": report}),
        )),
        Shipped::StaleIndex(sessions) => Ok(Outcome {
            report: json!({"command": "ship", "refused": "stale_index", "sessions": sessions}),
            status: STATUS_REFUSED,
        }),
    }
}

/// Run `fetch` and return its report with the status the process exits with.
pub fn run_fetch(args: &FetchArgs) -> Result<Outcome, HomeError> {
    let base = current_dir()?;
    let inherited: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    match fetch(&args.remote, &args.home, &base, &inherited)? {
        Fetched::Arrived(report) => Ok(Outcome::done(
            json!({"command": "fetch", "report": report}),
        )),
        Fetched::VerificationFailed(found) => Ok(Outcome {
            report: refusal_report(&found),
            status: STATUS_REFUSED,
        }),
    }
}

/// The directory a relative remote is resolved against: this process's
/// current directory.
fn current_dir() -> Result<PathBuf, HomeError> {
    std::env::current_dir()
        .map_err(|e| HomeError::io("reading the current directory", Path::new("."), e))
}
