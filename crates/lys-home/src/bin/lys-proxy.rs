//! `lys-proxy`: the little proxy (HOME-001 R10).
//!
//! Point a harness at `http://<listen>/anthropic` (or `/openai`); every
//! call is forwarded to the provider unchanged, and each model call is
//! recorded as one `lys.call` entry in `--home`, under the session its own
//! key names or the day's `unlinked` session. One JSON report line per call
//! is printed on stdout (ids, the session, a status and counts); the calls a
//! previous run left open are recorded `lost` and reported first. Nothing
//! here prints a header value or a body byte.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use lys_home::proxy::error::ProxyError;
use lys_home::proxy::forward::{Base, Proxy, ProxyConfig};
use lys_home::proxy::journal::CallReport;

/// The little proxy: forwards unchanged and records each call under its session.
#[derive(Debug, Parser)]
#[command(name = "lys-proxy", version, about)]
struct Args {
    /// The address to listen on.
    #[arg(long, default_value = "127.0.0.1:8484")]
    listen: String,
    /// The home the calls are recorded in.
    #[arg(long)]
    home: PathBuf,
    /// Where the open-call journal and the capture spool live.
    #[arg(long)]
    state: PathBuf,
    /// Where a `/anthropic` path is forwarded.
    #[arg(long, default_value = "https://api.anthropic.com")]
    anthropic: String,
    /// Where an `/openai` path is forwarded.
    #[arg(long, default_value = "https://api.openai.com")]
    openai: String,
    /// How many calls may hold spooled bodies not yet recorded; above it a
    /// call is forwarded and recorded `unrecorded`.
    #[arg(long, default_value_t = 64)]
    capture_slots: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lys-proxy: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), ProxyError> {
    let config = ProxyConfig {
        home: args.home,
        state: args.state,
        anthropic: Base::parse(&args.anthropic)?,
        openai: Base::parse(&args.openai)?,
        capture_slots: args.capture_slots,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| ProxyError::io("starting the runtime", &config.state, source))?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind(&args.listen)
            .await
            .map_err(|source| {
                ProxyError::io("binding the listen address", &config.state, source)
            })?;
        let started = Proxy::start(config)?;
        for report in &started.lost {
            print_report(report);
        }
        let reports = started.reports;
        std::thread::spawn(move || {
            for report in reports {
                print_report(&report);
            }
        });
        started.proxy.serve(listener).await
    })
}

fn print_report(report: &CallReport) {
    match serde_json::to_string(report) {
        Ok(line) => println!("{line}"),
        Err(error) => eprintln!("lys-proxy: a call report could not be serialised: {error}"),
    }
}
