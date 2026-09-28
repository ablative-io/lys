//! A refusal as the app shows it: a stable name, the plain words a person
//! reads, the one next thing to do, and the detail kept for the log.
//!
//! Invariants: `words` and `next` name what happened for a person, never a
//! program, a port, a password file or the issuer inside Lys; the detail,
//! which may name any of those, is written to the app's log and never put
//! on a page. Every failure the app meets becomes one of these by name.

use std::fmt;

use lys_install::IdentityError;
use lys_install::steps::Step;

/// A named refusal, in a person's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The stable name a page and a test key on.
    pub name: &'static str,
    /// What happened, in plain words.
    pub words: String,
    /// The one next thing to do.
    pub next: String,
    /// What went wrong in full, for the log only.
    pub detail: String,
}

impl Refusal {
    /// A refusal named `name`.
    pub fn new(
        name: &'static str,
        words: impl Into<String>,
        next: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            name,
            words: words.into(),
            next: next.into(),
            detail: detail.into(),
        }
    }

    /// The install stopped at `step`: its words, the next thing to do, and
    /// the install's own error kept for the log.
    pub fn at_step(step: Step, error: &IdentityError) -> Self {
        Self::new(
            "install_step_failed",
            format!("{} did not finish.", step.words()),
            "Press Try again. Lys picks up where it stopped; nothing already done is done twice.",
            format!("{}: {error}", step.name()),
        )
    }

    /// Lys was opened from the disk image or from a copy the system has
    /// moved aside, where nothing it installs could stay.
    pub fn translocated(path: &str) -> Self {
        Self::new(
            "app_not_in_applications",
            "Lys is running from the disk image, not from your Applications folder.",
            "Quit Lys, drag it to Applications, then open it from there.",
            format!("the app is at {path}"),
        )
    }

    /// Something the app needs to run at all could not be read or made.
    pub fn app(name: &'static str, words: &str, detail: impl fmt::Display) -> Self {
        Self::new(
            name,
            words,
            "Quit Lys and open it again. If this page comes back, download Lys again.",
            detail.to_string(),
        )
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({})", self.name, self.words, self.detail)
    }
}

impl std::error::Error for Refusal {}
