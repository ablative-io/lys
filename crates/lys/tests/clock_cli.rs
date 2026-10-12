//! Uninstalled fixture that runs the actual parser and CA command paths.

#![cfg(all(feature = "clock-fixture", not(test)))]

#[path = "../src/cli.rs"]
pub mod cli;
#[path = "../src/commands/mod.rs"]
pub mod commands;
#[path = "../src/identity/mod.rs"]
pub mod identity;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use chrono::{DateTime, Utc};
use clap::Parser;
use lys_core::clock::{Clock, ClockError, ClockSource};

#[derive(Debug)]
struct SuppliedTime {
    at: Option<DateTime<Utc>>,
    reads: AtomicU64,
}

impl Clock for SuppliedTime {
    fn now(&self) -> Result<DateTime<Utc>, ClockError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.at.ok_or_else(|| ClockError::Unavailable { reason: "fixture read refused".to_owned() })
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut args = std::env::args_os();
    let program = args.next().ok_or("fixture program name is missing")?;
    let supplied = args.next().ok_or("fixture creation time is missing")?;
    let supplied = supplied.to_str().ok_or("fixture creation time is not UTF-8")?;
    let at = if supplied == "unavailable" {
        None
    } else {
        Some(DateTime::from_timestamp(supplied.parse()?, 0).ok_or("fixture creation time is out of range")?)
    };
    let parsed = cli::Cli::parse_from(std::iter::once(program).chain(args));
    let cli::Command::Ca(command) = parsed.command else {
        return Err("clock fixture accepts CA commands only".into());
    };
    let clock = Arc::new(SuppliedTime { at, reads: AtomicU64::new(0) });
    let started = Instant::now();
    let result = commands::ca_dispatch::run(command, parsed.json, ClockSource::Supplied(clock.clone()));
    let elapsed_nanos = u64::try_from(started.elapsed().as_nanos())?;
    eprintln!("CLOCK_WORK {}", serde_json::json!({
        "provider_reads": clock.reads.load(Ordering::SeqCst),
        "elapsed_nanos": elapsed_nanos,
        "completed": true,
        "success": result.is_ok(),
    }));
    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(error) => {
            eprintln!("error: {error}");
            if parsed.json { commands::output::emit_json_error(&error.to_string()); }
            Ok(ExitCode::FAILURE)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => { eprintln!("clock fixture: {error}"); ExitCode::FAILURE }
    }
}
