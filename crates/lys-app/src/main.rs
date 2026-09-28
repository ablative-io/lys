//! `lys-app`: Lys.app's executable. Opening Lys installs it, starts it,
//! upgrades it or opens it, with every step on a Lys page in the browser
//! and no terminal at any point.
//!
//! - `lys-app` (as the Finder opens it): starts the app's work, opens the
//!   browser to its page, and exits ([`launcher`]).
//! - `lys-app --serve`: the app's work, serving its page ([`flow`]).
//! - `lys-app --at-login`: the login item's start ([`flow::at_login`]).
//! - `lys-app --uninstall [--remove-data]`: the uninstall helper
//!   ([`uninstall`]).
//!
//! Each takes `--root DIR` to name the data root; the platform's
//! application data path is used otherwise. `--version` names the build.
//! Every failure is a named [`refusal::Refusal`], shown on the page when
//! there is one and written to the app's log in full.

mod bundle;
mod engine;
mod engine_wait;
mod flow;
mod launcher;
mod login_item;
mod progress;
mod refusal;
mod server;
mod uninstall;

use std::path::PathBuf;
use std::process::ExitCode;

use refusal::Refusal;

/// What `--version` prints after the name: the crate version and the commit
/// the binary was built from, stamped by `build.rs`.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("LYS_BUILD"), ")");

/// What the app was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    /// Opened by a person.
    Open,
    /// The app's work.
    Serve,
    /// The login item.
    AtLogin,
    /// The uninstall helper, removing the data folder when `true`.
    Uninstall(bool),
    /// `--version`.
    Version,
}

/// The mode and the data root the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    mode: Mode,
    root: Option<PathBuf>,
}

/// Reads the arguments. The process serial number an older macOS passes an
/// app it opens (`-psn_…`) is the system's, not a request, and is passed
/// over; anything else unknown is refused by name.
fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, Refusal> {
    let unknown = |arg: &str| {
        Refusal::app(
            "argument_unknown",
            "Lys was started with something it does not understand.",
            format!("unknown argument `{arg}`"),
        )
    };
    let mut mode = Mode::Open;
    let mut remove_data = false;
    let mut root = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            launcher::SERVE => mode = Mode::Serve,
            login_item::AT_LOGIN => mode = Mode::AtLogin,
            uninstall::UNINSTALL => mode = Mode::Uninstall(false),
            uninstall::REMOVE_DATA => remove_data = true,
            "--version" | "-V" => mode = Mode::Version,
            "--root" => root = Some(PathBuf::from(args.next().ok_or_else(|| unknown("--root"))?)),
            system if system.starts_with("-psn_") => {}
            other => return Err(unknown(other)),
        }
    }
    match (&mode, remove_data) {
        (Mode::Uninstall(_), true) => mode = Mode::Uninstall(true),
        (_, true) => return Err(unknown(uninstall::REMOVE_DATA)),
        (_, false) => {}
    }
    Ok(Args { mode, root })
}

/// Runs what `args` ask, answering the refusal that stopped it.
fn run(args: &Args) -> Result<(), Refusal> {
    let root = args.root.as_deref();
    match args.mode {
        Mode::Open => launcher::open(root),
        Mode::Serve => flow::serve(root),
        Mode::AtLogin => flow::at_login(root),
        Mode::Uninstall(remove_data) => uninstall::run(root, remove_data),
        Mode::Version => {
            println!("lys-app {VERSION}");
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let outcome = parse(std::env::args().skip(1)).and_then(|args| run(&args));
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(refusal) => {
            eprintln!("error: {refusal}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
