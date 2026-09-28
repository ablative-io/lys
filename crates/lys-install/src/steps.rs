//! The install's steps, in the order they begin, each with the plain words
//! a person reads while it runs: what Lys.app's progress page lists.
//!
//! Invariants: the words name what is happening for the person, never a
//! program, a port, a file or the issuer inside Lys; every step has its own
//! words and its own stable name, which is what a page keys on.

use serde::Serialize;

/// One step of the install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// The container engine is asked whether it is installed and running.
    Engine,
    /// The data folder, its configuration and its credentials are made.
    Directory,
    /// The database, sign-in and permissions services are started and
    /// waited on until each answers.
    SignIn,
    /// Lys's own sign-in clients and their styling are registered.
    Clients,
    /// The service key and the secrets store are made.
    Keys,
    /// The screens are checked and placed.
    Screens,
    /// The secrets broker and the directory service are started.
    Start,
    /// The build now running is recorded.
    Build,
}

impl Step {
    /// Every step, in the order the install begins them.
    pub const ALL: [Step; 8] = [
        Step::Engine,
        Step::Directory,
        Step::SignIn,
        Step::Clients,
        Step::Keys,
        Step::Screens,
        Step::Start,
        Step::Build,
    ];

    /// The words a person reads while the step runs.
    pub fn words(self) -> &'static str {
        match self {
            Step::Engine => "Checking the container engine",
            Step::Directory => "Preparing your directory",
            Step::SignIn => "Starting sign-in",
            Step::Clients => "Connecting sign-in to Lys",
            Step::Keys => "Making Lys's keys",
            Step::Screens => "Placing Lys's screens",
            Step::Start => "Starting Lys",
            Step::Build => "Recording what is installed",
        }
    }

    /// The step's stable name, as its serialised form spells it.
    pub fn name(self) -> &'static str {
        match self {
            Step::Engine => "engine",
            Step::Directory => "directory",
            Step::SignIn => "sign_in",
            Step::Clients => "clients",
            Step::Keys => "keys",
            Step::Screens => "screens",
            Step::Start => "start",
            Step::Build => "build",
        }
    }
}

#[cfg(test)]
#[path = "steps_tests.rs"]
mod tests;
