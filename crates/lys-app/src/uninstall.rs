//! Uninstalling Lys, as `lys-app --uninstall`: the helper the directory
//! service starts, detached, when an administrator presses Uninstall.
//!
//! The helper outlives the service it stops. It first asks the container
//! engine, since containers stopped while the engine is not answering come
//! back when it next starts; then it stops the directory service and the
//! secrets broker, each waited on through its exit lock, stops and removes
//! the compose services (their volumes too only when the person ticked to
//! remove their data), removes the login item, and removes the install's
//! binaries. With the tick it removes the whole data folder. Without it the
//! data folder, the credentials, the configuration and the database volume
//! stay, so installing again signs the same people in.
//!
//! What happened is recorded beside the data folder, in Lys's own data
//! path, where the app shows it the next time it is opened, and removes it
//! once shown.
//!
//! Invariants: nothing is stopped or removed while the engine does not
//! answer; the data folder is removed only when the person asked for it;
//! and every outcome, done or refused, is recorded in plain words.

use std::path::{Path, PathBuf};

use lys_install::config::DeploymentConfig;
use lys_install::install::layout::Layout;
use lys_install::install::{engine_path, services};
use lys_install::upgrade;
use serde::{Deserialize, Serialize};

use crate::engine;
use crate::flow;
use crate::login_item;
use crate::refusal::Refusal;

/// The argument the helper runs with.
pub const UNINSTALL: &str = "--uninstall";

/// The argument that removes the data folder as well.
pub const REMOVE_DATA: &str = "--remove-data";

/// What the last uninstall did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Whether it finished.
    pub done: bool,
    /// Whether the data folder was removed.
    pub removed_data: bool,
    /// What the app shows, in plain words.
    pub words: String,
}

/// Where the last uninstall is recorded: beside the data folder, so it
/// stays when the folder is removed.
pub fn record_path(layout: &Layout) -> PathBuf {
    layout
        .root
        .parent()
        .unwrap_or(&layout.root)
        .join("last-uninstall.json")
}

/// The record of `outcome`.
pub fn record_of(outcome: &Result<(), Refusal>, removed_data: bool) -> Record {
    let words = match (outcome, removed_data) {
        (Ok(()), false) => "Lys was uninstalled. Your data folder was kept, so installing Lys \
                            again signs the same people in."
            .to_string(),
        (Ok(()), true) => "Lys was uninstalled and its data folder was removed.".to_string(),
        (Err(refusal), _) => format!(
            "Lys could not finish uninstalling. {} {}",
            refusal.words, refusal.next
        ),
    };
    Record {
        done: outcome.is_ok(),
        removed_data: removed_data && outcome.is_ok(),
        words,
    }
}

fn failed(words: &str, detail: impl std::fmt::Display) -> Refusal {
    Refusal::new(
        "uninstall_failed",
        words,
        "Open Lys and press Uninstall again.",
        detail.to_string(),
    )
}

fn remove_dir(path: &Path) -> Result<(), Refusal> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(failed(
            "Lys's files could not all be removed.",
            format!("{}: {error}", path.display()),
        )),
    }
}

/// Stops and removes Lys under `layout`, and its data when `remove_data`.
fn uninstall(layout: &Layout, remove_data: bool) -> Result<(), Refusal> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if engine::answering(&engine::socket_paths(home.as_deref())).is_none() {
        return Err(Refusal::new(
            "engine_not_running",
            "Nothing was removed, because the container engine is not running.",
            "Open Docker Desktop, then press Uninstall again.",
            "no engine socket answered",
        ));
    }
    let config = DeploymentConfig::load(&layout.deployment_config())
        .map_err(|error| failed("Lys's configuration could not be read.", error))?;
    for unit in upgrade::units(layout).iter().rev() {
        services::stop(&unit.pid).map_err(|error| failed("Lys could not be stopped.", error))?;
    }
    let mut args = services::compose_args(layout, &config);
    args.push("down".to_string());
    if remove_data {
        args.push("--volumes".to_string());
    }
    let mut docker = engine_path::docker_command()
        .map_err(|error| failed("Lys's services could not be stopped.", error))?;
    services::run_command(&mut docker, &args, "compose")
        .map_err(|error| failed("Lys's services could not be stopped.", error))?;
    login_item::unregister()?;
    remove_dir(&layout.bin_dir())?;
    remove_dir(&layout.bin_previous_dir())?;
    if remove_data {
        remove_dir(&layout.root)?;
    }
    Ok(())
}

/// Writes `record` where the app shows it next.
fn write_record(layout: &Layout, record: &Record) -> Result<(), Refusal> {
    let path = record_path(layout);
    let text = serde_json::to_vec_pretty(record)
        .map_err(|error| failed("The uninstall could not be recorded.", error))?;
    std::fs::write(&path, text).map_err(|error| {
        failed(
            "The uninstall could not be recorded.",
            format!("{}: {error}", path.display()),
        )
    })
}

/// Runs `lys-app --uninstall`, recording what it did.
pub fn run(root: Option<&Path>, remove_data: bool) -> Result<(), Refusal> {
    let layout = flow::layout(root)?;
    let outcome = uninstall(&layout, remove_data);
    if let Err(refusal) = &outcome {
        eprintln!("{refusal}");
    }
    write_record(&layout, &record_of(&outcome, remove_data))?;
    outcome
}

/// What the last uninstall recorded, in plain words, taken so it is shown
/// once.
pub fn take_note(layout: &Layout) -> Result<Option<String>, Refusal> {
    let path = record_path(layout);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(failed(
                "The last uninstall's record could not be read.",
                format!("{}: {error}", path.display()),
            ));
        }
    };
    let record: Record = serde_json::from_str(&text).map_err(|error| {
        failed(
            "The last uninstall's record could not be read.",
            format!("{}: {error}", path.display()),
        )
    })?;
    std::fs::remove_file(&path).map_err(|error| {
        failed(
            "The last uninstall's record could not be cleared.",
            format!("{}: {error}", path.display()),
        )
    })?;
    Ok(Some(record.words))
}

#[cfg(test)]
#[path = "uninstall_tests.rs"]
mod tests;
