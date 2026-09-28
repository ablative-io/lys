//! What the progress page shows, held in one place and handed to every page
//! that is watching as it changes.
//!
//! The app's work (install, upgrade, start, the wait for the container
//! engine) writes a [`Phase`] to the [`Board`]; each page watching reads it
//! through [`Board::next_after`], which blocks until the board has moved
//! past what that page last saw. Nothing is asked again on a schedule: a
//! watcher sleeps on the board's condition and is woken by the write.
//!
//! Invariants: the board's version only rises, so a watcher never misses
//! the latest phase and never sees one twice; a phase names no program,
//! port, password file or issuer (a [`Refusal`]'s detail stays in the log);
//! and the work waits for a person's "Try again" only after a failure, never
//! otherwise.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use lys_install::steps::Step;
use serde::Serialize;

use crate::engine::Guidance;
use crate::refusal::Refusal;

/// What the app is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Work {
    /// Installing Lys for the first time.
    Install,
    /// Starting an install that is stopped.
    Start,
    /// Moving the install to the build this app carries.
    Upgrade,
}

impl Work {
    /// The page's heading while the work runs.
    pub fn title(self) -> &'static str {
        match self {
            Work::Install => "Installing Lys",
            Work::Start => "Starting Lys",
            Work::Upgrade => "Updating Lys",
        }
    }
}

/// One step as the page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepView {
    /// The step's stable name.
    pub name: &'static str,
    /// Its plain words.
    pub words: &'static str,
    /// `done`, `now` or `waiting`.
    pub state: &'static str,
}

/// Every step of the install, with `current` the one running now.
pub fn steps(current: Option<Step>) -> Vec<StepView> {
    let at = current.and_then(|step| Step::ALL.iter().position(|each| *each == step));
    Step::ALL
        .iter()
        .enumerate()
        .map(|(index, step)| StepView {
            name: step.name(),
            words: step.words(),
            state: match at {
                Some(at) if index < at => "done",
                Some(at) if index == at => "now",
                _ => "waiting",
            },
        })
        .collect()
}

/// What the page shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    /// The app is finding out what to do.
    Opening,
    /// Work is running: its title, its steps, and the lines it has said.
    Working {
        /// The work.
        work: Work,
        /// The heading.
        title: &'static str,
        /// The install's steps, when the work runs them.
        steps: Vec<StepView>,
        /// What the work has said, in plain words, in order.
        said: Vec<String>,
    },
    /// The container engine is missing or stopped; the work continues by
    /// itself when it answers.
    Engine {
        /// What the page tells the person.
        guidance: Guidance,
    },
    /// The work stopped: what, the next thing to do, and whether "Try again"
    /// resumes it.
    Failed {
        /// The refusal's name.
        refusal: &'static str,
        /// What happened.
        words: String,
        /// The one next thing to do.
        next: String,
        /// Whether pressing "Try again" runs the work again.
        retry: bool,
        /// The steps as they stood.
        steps: Vec<StepView>,
    },
    /// Lys is ready: the page goes to `url`.
    Ready {
        /// What the page says as it goes.
        words: &'static str,
        /// Where the browser is handed.
        url: String,
    },
}

impl Phase {
    /// Whether the work ends once a page has been sent this phase: Lys is
    /// ready, or the work stopped where "Try again" cannot help.
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Phase::Ready { .. } | Phase::Failed { retry: false, .. }
        )
    }

    /// The failed phase for `refusal`, with the steps as they stood.
    pub fn failed(refusal: &Refusal, retry: bool, steps: Vec<StepView>) -> Self {
        Phase::Failed {
            refusal: refusal.name,
            words: refusal.words.clone(),
            next: refusal.next.clone(),
            retry,
            steps,
        }
    }
}

/// What the page is sent: the phase, and the note from the last uninstall
/// when there is one to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct View {
    /// The board's version when this was read.
    pub version: u64,
    /// The phase.
    #[serde(flatten)]
    pub phase: Phase,
    /// What the last uninstall recorded, shown once on the next open.
    pub note: Option<String>,
}

#[derive(Debug)]
struct Held {
    version: u64,
    phase: Phase,
    note: Option<String>,
    retry: bool,
    delivered: bool,
}

/// The one board a running app keeps.
#[derive(Debug)]
pub struct Board {
    held: Mutex<Held>,
    changed: Condvar,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    /// A board showing [`Phase::Opening`].
    pub fn new() -> Self {
        Self {
            held: Mutex::new(Held {
                version: 1,
                phase: Phase::Opening,
                note: None,
                retry: false,
                delivered: false,
            }),
            changed: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn wait<'a>(&self, guard: MutexGuard<'a, Held>) -> MutexGuard<'a, Held> {
        self.changed
            .wait(guard)
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Shows `phase`, waking every page watching.
    pub fn show(&self, phase: Phase) {
        let mut held = self.lock();
        held.version += 1;
        held.phase = phase;
        held.delivered = false;
        self.changed.notify_all();
    }

    /// Adds the note the next pages show beside the phase.
    pub fn note(&self, note: String) {
        let mut held = self.lock();
        held.version += 1;
        held.note = Some(note);
        self.changed.notify_all();
    }

    /// The phase now.
    pub fn now(&self) -> View {
        let held = self.lock();
        View {
            version: held.version,
            phase: held.phase.clone(),
            note: held.note.clone(),
        }
    }

    /// The first view newer than `seen`, waiting on the board until there is
    /// one.
    pub fn next_after(&self, seen: u64) -> View {
        let mut held = self.lock();
        while held.version <= seen {
            held = self.wait(held);
        }
        View {
            version: held.version,
            phase: held.phase.clone(),
            note: held.note.clone(),
        }
    }

    /// A page has been sent the view at `version`; when that view's phase
    /// [`Phase::is_final`], the app's work is done.
    pub fn delivered(&self, version: u64) {
        let mut held = self.lock();
        if held.version == version && held.phase.is_final() {
            held.delivered = true;
            self.changed.notify_all();
        }
    }

    /// Waits until a page has been sent the final phase now shown.
    pub fn wait_delivered(&self) {
        let mut held = self.lock();
        while !held.delivered {
            held = self.wait(held);
        }
    }

    /// A person pressed "Try again": the failed work runs again.
    pub fn ask_retry(&self) -> bool {
        let mut held = self.lock();
        let failed = matches!(held.phase, Phase::Failed { retry: true, .. });
        if failed {
            held.retry = true;
            self.changed.notify_all();
        }
        failed
    }

    /// Waits until a person presses "Try again", and takes the press.
    pub fn wait_retry(&self) {
        let mut held = self.lock();
        while !held.retry {
            held = self.wait(held);
        }
        held.retry = false;
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
