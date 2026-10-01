//! The intent record of an upgrade under way: `install/upgrade.json`.
//!
//! Before an upgrade stops anything it writes which build it moves from and
//! which it moves to, the configuration and compose files it will replace,
//! and whether the compose definition changes. Each step is added to the
//! record as it completes. The record is written whole to a temporary name
//! and renamed over the last one, so a reader finds the record before a step
//! or after it, never a torn one. It is removed when the upgrade ends,
//! finished or put back.
//!
//! Invariants: a record present means an upgrade stopped part-way; every
//! upgrade and every install reads it first and ends it, naming which way,
//! before doing anything else. A step is recorded only after it completed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::layout::Layout;
use super::super::private_files;

/// One step of an upgrade, recorded once it has completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// The service and then the broker were stopped.
    Stopped,
    /// `data/` was copied whole to `data.previous/`.
    DataKept,
    /// `bin/` was moved to `bin.previous/`.
    BinariesKept,
    /// Every new binary was placed in `bin/` and read back.
    BinariesPlaced,
    /// The files to be replaced were copied to `config.previous/`.
    ConfigurationKept,
    /// The new configuration and compose files were placed and read back.
    ConfigurationPlaced,
    /// The screens were moved to `surface.previous/`.
    ScreensKept,
    /// The new screens were placed.
    ScreensPlaced,
    /// The compose services were brought to the new definition.
    ComposeApplied,
    /// The broker and then the service were started and ready.
    Started,
}

/// A configuration or compose file the upgrade replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// Its name in `config.previous/`.
    pub name: String,
    /// Where it is placed.
    pub target: PathBuf,
    /// Whether it existed before the upgrade; one that did not is removed
    /// when the previous build is put back.
    pub existed: bool,
    /// Whether it is written owner-only.
    pub private: bool,
}

/// The record of one upgrade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    /// Each binary's commit before the upgrade.
    pub from: BTreeMap<String, String>,
    /// Each binary's commit after it.
    pub to: BTreeMap<String, String>,
    /// Whether the upgrade places screens.
    pub screens: bool,
    /// Whether screens were placed before it.
    pub screens_existed: bool,
    /// The configuration and compose files it replaces.
    pub files: Vec<Kept>,
    /// Whether the compose definition it places differs from the one before.
    pub compose_changed: bool,
    /// Each step completed, in order.
    pub steps: Vec<Step>,
}

fn invalid(detail: impl Into<String>, layout: &Layout) -> IdentityError {
    IdentityError::new(
        ErrorKind::ConfigInvalid,
        "read upgrade record",
        "upgrade.json",
        detail,
    )
    .at(&layout.upgrade_intent())
}

/// Each commit of `build`, once when every binary names the same one.
fn commits(build: &BTreeMap<String, String>) -> String {
    let mut distinct: Vec<&String> = build.values().collect();
    distinct.dedup();
    match distinct.as_slice() {
        [one] => (*one).clone(),
        _ => build
            .iter()
            .map(|(name, commit)| format!("{name} {commit}"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

impl Intent {
    /// The record of the upgrade stopped part-way under `layout`, if any.
    pub fn read(layout: &Layout) -> IdentityResult<Option<Self>> {
        let Some(bytes) = private_files::read(&layout.upgrade_intent())? else {
            return Ok(None);
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| invalid(error.to_string(), layout))
    }

    /// Writes the record as it stands, whole, by a temporary name and a
    /// rename.
    pub fn write(&self, layout: &Layout) -> IdentityResult<()> {
        let text = serde_json::to_vec_pretty(self).map_err(|error| {
            IdentityError::new(
                ErrorKind::RenderFailed,
                "render",
                "upgrade.json",
                error.to_string(),
            )
        })?;
        private_files::ensure_dir(&layout.install_dir())?;
        private_files::write(&layout.upgrade_intent(), &text)?;
        Ok(())
    }

    /// Records `step` as completed.
    pub fn done(&mut self, layout: &Layout, step: Step) -> IdentityResult<()> {
        self.steps.push(step);
        self.write(layout)
    }

    /// Whether `step` was recorded as completed.
    pub fn has(&self, step: Step) -> bool {
        self.steps.contains(&step)
    }

    /// Removes the record: the upgrade has ended.
    pub fn clear(layout: &Layout) -> IdentityResult<()> {
        let path = layout.upgrade_intent();
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(IdentityError::new(
                ErrorKind::PrivateFileIo,
                "remove upgrade record",
                "upgrade.json",
                error.to_string(),
            )
            .at(&path)),
        }
    }

    /// The upgrade named by its builds: `from A to B`.
    pub fn describe(&self) -> String {
        format!("from {} to {}", commits(&self.from), commits(&self.to))
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
