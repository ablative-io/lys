//! The secrets broker's command line: make a broker, seal a credential from
//! standard input, grant and revoke its use, issue and drop handles, sign a
//! presentation as an agent, page through the audit log from its tail, audit
//! every line of it, and serve the proxy.

#![warn(clippy::await_holding_lock)]

mod account_binding;
#[cfg(test)]
mod account_binding_tests;
mod app_client;
mod app_client_tests;
mod args;
mod callers;
mod cli;
mod command_run;
mod files;
mod manage;
mod oauth_proxy;
mod redact;
#[cfg(test)]
mod redact_tests;
#[cfg(test)]
mod route_write_fault_tests;
mod router;
mod save_app;
mod save_app_tests;
mod serve;
#[cfg(test)]
mod serve_tests;
mod signing;
mod spice;
mod upgrade_window;
mod values;
#[cfg(test)]
mod values_tests;
mod view;
#[cfg(test)]
mod view_tests;
mod watched;

use std::io::Read;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches};
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, EntryClass, HandleId, Holder, Presentation, Recipients, RecordedLine, Relation, Scope,
    Secret, SecretsError, UpstreamRevocation, new_operation_id, request_digest, to_hex,
};

use args::{Page, RecordClass};
use cli::{Cli, Command, Where};
use files::{FileGrants, Layout, Route, now_ms};
use spice::Grants;

fn read_credential() -> Result<Secret, SecretsError> {
    let mut value = Vec::new();
    std::io::stdin()
        .read_to_end(&mut value)
        .map_err(|source| SecretsError::Io {
            context: "reading the credential from standard input".to_owned(),
            source,
        })?;
    while value.last().is_some_and(u8::is_ascii_whitespace) {
        value.pop();
    }
    Ok(Secret::new(value))
}

fn open(at: &Where) -> Result<Broker<Grants>, SecretsError> {
    let layout = Layout::new(&at.root, &at.keys);
    open_with(at, Grants::File(FileGrants::new(layout.grants())))
}

fn open_with(at: &Where, grants: Grants) -> Result<Broker<Grants>, SecretsError> {
    let layout = Layout::new(&at.root, &at.keys);
    match upgrade_window::check(&at.root) {
        Some(ready) => Broker::open_deferred(&layout.paths(), grants, Box::new(now_ms), ready),
        None => Broker::open(&layout.paths(), grants, Box::new(now_ms)),
    }
}

fn print_line(recorded: RecordedLine) {
    let line = recorded.line;
    println!(
        "{:>3} {:<8} {:<32} {:<14} {:<16} {}",
        recorded.index,
        line.kind.label(),
        line.handle.unwrap_or_default(),
        line.identity.unwrap_or_default(),
        line.secret.unwrap_or_default(),
        line.outcome
    );
}

/// Prints one page of the audit log through its window, then the index the
/// next older page ends before, or that this page reached the first line.
/// It reads the page's lines and no others.
fn print_page(broker: &Broker<Grants>, page: &Page) -> Result<(), SecretsError> {
    let lines = broker.audit().window(page.before, page.most.get())?;
    // A page holds at least one line unless it ends at line 0.
    let from = lines.first().map_or(0, |first| first.index);
    for recorded in lines {
        print_line(recorded);
    }
    if from > 0 {
        println!("older: --before {from}");
    } else {
        println!("first line reached");
    }
    Ok(())
}

fn run(command: Command) -> Result<(), SecretsError> {
    match command {
        Command::Init(_) | Command::Seal { .. } | Command::SealOauth { .. } => {
            command_run::credentials(command)
        }
        Command::SealRecord { .. } | Command::ReadRecord { .. } => command_run::records(command),
        Command::Grant { .. }
        | Command::Revoke { .. }
        | Command::Scope { .. }
        | Command::Recipients { .. } => command_run::permissions(command),
        Command::Issue { .. } | Command::Drop { .. } | Command::SpawnLogin { .. } => {
            command_run::handles(command)
        }
        Command::AddAccount { .. }
        | Command::NextAccount { .. }
        | Command::RestoreAccount { .. }
        | Command::Accounts { .. } => command_run::accounts(command),
        Command::TrustService { .. } | Command::ServiceKey { .. } | Command::Sign { .. } => {
            command_run::signing(command)
        }
        Command::List { .. }
        | Command::Revocation { .. }
        | Command::ConfirmRevocation { .. }
        | Command::Log { .. }
        | Command::Audit { .. }
        | Command::Serve { .. } => command_run::inspection(command),
    }
}

/// What `--version` prints after the name: the crate version and the commit
/// the binary was built from, stamped by `build.rs`.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("LYS_BUILD"), ")");

/// The arguments, with `--version` answering [`VERSION`].
fn parse() -> Cli {
    let matches = Cli::command().version(VERSION).get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

/// When this process began, for the proxy's ready line.
pub(crate) static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn main() -> ExitCode {
    STARTED.get_or_init(std::time::Instant::now);
    match run(parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
