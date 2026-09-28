//! The app's work: what opening Lys does, decided from what is installed,
//! run as library calls into `lys-install` in this process, and shown on
//! the progress page as it happens.
//!
//! - Nothing installed: the install runs, each step shown as it begins, and
//!   the browser is handed to first-run setup.
//! - Installed with another build than this app carries: the install moves
//!   to this app's build through `lys identity upgrade`'s own upgrade, its
//!   lines shown, the previous build put back if the new one does not come
//!   up, and the browser is handed to Lys's screens.
//! - Installed and running: the browser is handed to Lys's screens.
//! - Installed and stopped: the install is run again, which starts only
//!   what is not running, and the browser is handed to Lys's screens.
//!
//! Before anything that needs the container engine, the engine is asked;
//! when it does not answer, the page shows what to do and the work waits
//! for it on the engine's socket event ([`crate::engine`]). A failure shows
//! its words and the one next thing to do; "Try again" runs the work again,
//! and since the install makes only what is missing, it resumes.
//!
//! Invariants: no work is started from a disk image or a translocated copy;
//! nothing here starts a shell or a terminal, asks the person for a port,
//! a command or a password file, or names the issuer on the page.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_install::install::layout::Layout;
use lys_install::install::{self, engine_path, services};
use lys_install::steps::Step;
use lys_install::upgrade;

use crate::bundle::{self, APP, Bundle, Installed};
use crate::engine::{self, Guidance};
use crate::engine_wait;
use crate::launcher::{self, Claim};
use crate::progress::{Board, Phase, StepView, Work, steps};
use crate::refusal::Refusal;
use crate::{login_item, server, uninstall};

/// The administrator address a first install is made with, until first-run
/// setup (DIRECTORY-047) asks the person for their own.
pub const BOOTSTRAP_ADMIN: &str = "admin@identity.test";

/// The first-run setup page the browser is handed to after an install.
pub fn setup_url() -> String {
    format!("{}/setup", Layout::service_url())
}

/// The data root: `root` when named, the platform's otherwise.
pub fn layout(root: Option<&Path>) -> Result<Layout, Refusal> {
    match root {
        Some(root) => Ok(Layout::at(root.to_path_buf())),
        None => Layout::discover().map_err(|error| {
            Refusal::app(
                "data_folder_unknown",
                "Lys could not find where to keep its data.",
                error,
            )
        }),
    }
}

/// A failure and whether "Try again" runs the work again.
struct Failure {
    refusal: Refusal,
    retry: bool,
}

impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        Self {
            refusal,
            retry: true,
        }
    }
}

/// One run of the work, as the page shows it.
struct Run<'a> {
    board: &'a Board,
    work: Work,
    step: Option<Step>,
    said: Vec<String>,
}

impl<'a> Run<'a> {
    fn new(board: &'a Board, work: Work) -> Self {
        let run = Self {
            board,
            work,
            step: None,
            said: Vec::new(),
        };
        run.show();
        run
    }

    fn steps(&self) -> Vec<StepView> {
        match self.work {
            Work::Upgrade => Vec::new(),
            Work::Install | Work::Start => steps(self.step),
        }
    }

    fn show(&self) {
        self.board.show(Phase::Working {
            work: self.work,
            title: self.work.title(),
            steps: self.steps(),
            said: self.said.clone(),
        });
    }

    fn step(&mut self, step: Step) {
        match self.work {
            Work::Upgrade => self.say(step.words()),
            Work::Install | Work::Start => {
                self.step = Some(step);
                self.show();
            }
        }
    }

    fn say(&mut self, line: &str) {
        self.said.push(for_page(line));
        self.show();
    }

    /// The install stopped: named at the step it was in.
    fn stopped(&self, error: &lys_install::IdentityError) -> Refusal {
        match self.step {
            Some(step) => Refusal::at_step(step, error),
            None => Refusal::new(
                "work_failed",
                format!("{} did not finish.", self.work.title()),
                "Press Try again.",
                error.to_string(),
            ),
        }
    }
}

/// `line` as the page shows it: the issuer inside Lys is called sign-in,
/// never named.
pub fn for_page(line: &str) -> String {
    line.replace("Rauthy", "sign-in")
        .replace("rauthy", "sign-in")
}

/// Waits, showing the guidance, until the container engine answers.
pub fn engine_ready(board: &Board) -> Result<(), Refusal> {
    let (dirs, _) = engine_path::search_path().map_err(|error| {
        let words = "Lys could not look for the container engine.";
        Refusal::app("engine_path_unknown", words, error)
    })?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let sockets = engine::socket_paths(home.as_deref());
    if engine::answering(&sockets).is_some() {
        return Ok(());
    }
    board.show(Phase::Engine {
        guidance: Guidance::for_mac(&dirs),
    });
    engine_wait::wait(&sockets, &engine::state_folders(home.as_deref())).map(drop)
}

/// Whether every process the install runs is alive.
fn running(layout: &Layout) -> bool {
    upgrade::units(layout)
        .iter()
        .all(|unit| services::alive(&unit.pid))
}

/// Runs the install for `layout` from this app, with `surface` the screens
/// to place when there are any.
fn run_install(
    run: &mut Run<'_>,
    layout: &Layout,
    surface: Option<PathBuf>,
) -> Result<(), Refusal> {
    let options = install::Options {
        root: Some(layout.root.clone()),
        admin_email: BOOTSTRAP_ADMIN.to_string(),
        surface,
    };
    let outcome = install::run(&options, true, &mut |step| run.step(step));
    outcome.map_err(|error| run.stopped(&error))
}

/// Keeps this app's own binary in the install's `bin/`, for the login item
/// and the uninstall helper, and registers the login item.
fn keep_app(layout: &Layout, bundle: &Bundle) -> Result<(), Refusal> {
    upgrade::place_binary(&bundle.binary(APP), &layout.bin_dir(), APP).map_err(|error| {
        let words = "Lys could not keep a copy of itself for starting at login.";
        Refusal::app("app_not_kept", words, error)
    })?;
    login_item::register(layout).map(drop)
}

/// The screens this app carries, refused by name when it carries none;
/// "Try again" cannot help a copy without them.
fn screens(bundle: &Bundle) -> Result<PathBuf, Failure> {
    let surface = bundle.surface();
    if surface.join("index.html").is_file() {
        return Ok(surface);
    }
    Err(Failure {
        refusal: Refusal::app(
            "screens_missing",
            "This copy of Lys is missing its screens.",
            surface.display(),
        ),
        retry: false,
    })
}

fn install_fresh(layout: &Layout, bundle: &Bundle, board: &Board) -> Result<String, Failure> {
    let surface = screens(bundle)?;
    let mut run = Run::new(board, Work::Install);
    run.step(Step::Engine);
    engine_ready(board)?;
    run.show();
    run_install(&mut run, layout, Some(surface))?;
    keep_app(layout, bundle)?;
    Ok(setup_url())
}

fn start(layout: &Layout, board: &Board) -> Result<String, Failure> {
    let mut run = Run::new(board, Work::Start);
    run.step(Step::Engine);
    engine_ready(board)?;
    run.show();
    run_install(&mut run, layout, None)?;
    Ok(Layout::service_url())
}

fn upgrade_to(layout: &Layout, bundle: &Bundle, board: &Board) -> Result<String, Failure> {
    let surface = screens(bundle)?;
    let mut run = Run::new(board, Work::Upgrade);
    engine_ready(board)?;
    run.show();
    run_install(&mut run, layout, None)?;
    let units = upgrade::units(layout);
    let from = &bundle.binaries;
    let outcome = upgrade::upgrade(layout, from, Some(&surface), &units, &mut |line| {
        run.say(line);
    });
    outcome.map_err(|error| {
        Refusal::new(
            "upgrade_failed",
            "Lys could not move to this version.",
            "Press Try again. The version you had is put back whenever it can be.",
            error.to_string(),
        )
    })?;
    keep_app(layout, bundle)?;
    Ok(Layout::service_url())
}

/// One run of the work: what opening the app does now.
fn attempt(layout: &Layout, bundle: &Bundle, board: &Board) -> Result<String, Failure> {
    bundle.require_placed().map_err(|refusal| Failure {
        refusal,
        retry: false,
    })?;
    match bundle::installed(layout)? {
        Installed::Nothing => install_fresh(layout, bundle, board),
        Installed::Build(recorded) => {
            let carried = bundle.build()?;
            if recorded.as_deref() != Some(carried.as_str()) {
                upgrade_to(layout, bundle, board)
            } else if running(layout) {
                Ok(Layout::service_url())
            } else {
                start(layout, board)
            }
        }
    }
}

/// Moves this process's standard output to `log`, so nothing it prints
/// reaches the opening app's pipe after that app has gone.
#[cfg(unix)]
fn quiet(log: &std::fs::File) -> Result<(), Refusal> {
    rustix::stdio::dup2_stdout(log)
        .map_err(|error| Refusal::app("app_start_failed", "Lys could not start.", error))
}

#[cfg(not(unix))]
fn quiet(_log: &std::fs::File) -> Result<(), Refusal> {
    Ok(())
}

/// The app's work, as `lys-app --serve`: claims the install, serves the
/// page, names its port on standard output, and runs until the browser has
/// been handed on.
pub fn serve(root: Option<&Path>) -> Result<(), Refusal> {
    let layout = layout(root)?;
    let bundle = Bundle::current()?;
    let mut instance = match launcher::claim(&layout)? {
        Claim::Theirs(port) => {
            println!("{port}");
            return Ok(());
        }
        Claim::Ours(instance) => instance,
    };
    let not_served = |error: std::io::Error| {
        Refusal::app("page_not_served", "Lys could not show its page.", error)
    };
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(not_served)?;
    let port = listener.local_addr().map_err(not_served)?.port();
    let board = Arc::new(Board::new());
    server::spawn(listener, Arc::clone(&board), bundle.surface()).map_err(not_served)?;
    instance.publish(port)?;
    println!("{port}");
    quiet(&launcher::append(&launcher::log_path(&layout))?)?;
    if let Some(note) = uninstall::take_note(&layout)? {
        board.note(note);
    }
    loop {
        match attempt(&layout, &bundle, &board) {
            Ok(url) => {
                board.show(Phase::Ready {
                    words: "Lys is ready. Taking you there now.",
                    url,
                });
                board.wait_delivered();
                return Ok(());
            }
            Err(failure) => {
                eprintln!("{}", failure.refusal);
                let steps = match board.now().phase {
                    Phase::Working { steps, .. } => steps,
                    _ => Vec::new(),
                };
                board.show(Phase::failed(&failure.refusal, failure.retry, steps));
                if !failure.retry {
                    board.wait_delivered();
                    return Err(failure.refusal);
                }
                board.wait_retry();
            }
        }
    }
}

/// The login item's work, as `lys-app --at-login`: waits for the container
/// engine as the page's work does, then starts what is not running. There
/// is no page; what it does and any refusal go to the app's log.
pub fn at_login(root: Option<&Path>) -> Result<(), Refusal> {
    let layout = layout(root)?;
    if bundle::installed(&layout)? == Installed::Nothing {
        return Err(Refusal::app(
            "not_installed",
            "Lys is not installed, so there is nothing to start.",
            layout.root.display(),
        ));
    }
    let board = Board::new();
    engine_ready(&board)?;
    let mut run = Run::new(&board, Work::Start);
    run_install(&mut run, &layout, None)
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests;
