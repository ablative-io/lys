//! Uninstalling Lys from its own screens: the administrator's account
//! screen reads what an uninstall would remove, and asks for it.
//!
//! The service cannot remove itself, so it starts the install's uninstall
//! helper, the `lys-app` Lys.app keeps beside this binary in the install's
//! `bin/`, detached and in its own process group, and answers at once. The
//! helper outlives the service: it stops the service and the broker, the
//! compose services and the login item, removes the binaries, removes the
//! data folder only when asked, and records what it did where the app
//! shows it on its next opening.
//!
//! Invariants: only the administrator reads or asks; removing the data
//! folder needs the person's confirmation that they have read what is
//! lost; the helper is given none of the service's descriptors (its
//! standard input is not the service's exit lock, which it must see
//! released); and an install that keeps no helper, as one made from the
//! command line, is refused by name, never partly uninstalled.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// The uninstall helper's binary, beside this one in the install's `bin/`.
pub const HELPER: &str = "lys-app";

/// What removing the data folder loses, as the confirmation names it.
pub const LOST: [&str; 5] = [
    "every person's sign-in account and password",
    "the people, agents and teams in the directory",
    "every permission and role that was given",
    "the signed record of every change",
    "every secret Lys keeps",
];

/// What the administrator asks.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UninstallBody {
    /// Whether the data folder is removed as well.
    pub remove_data: bool,
    /// Whether the person confirmed what removing the data folder loses.
    #[serde(default)]
    pub confirmed: bool,
}

/// The uninstall routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/uninstall", get(read).post(uninstall))
}

/// The helper and the data root for the binary at `own`, which runs from
/// the install's `bin/`.
pub fn placement(own: &Path) -> Option<(PathBuf, PathBuf)> {
    let bin = own.parent()?;
    let root = bin.parent()?;
    Some((bin.join(HELPER), root.to_path_buf()))
}

/// The helper's arguments for the data root `root`.
pub fn helper_args(root: &Path, remove_data: bool) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["--uninstall".into(), "--root".into(), root.into()];
    if remove_data {
        args.push("--remove-data".into());
    }
    args
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::UninstallUnavailable {
        reason: reason.into(),
    }
}

/// The helper and the data root for this running service, refused by name
/// when the install keeps no helper.
fn helper() -> Result<(PathBuf, PathBuf), ServerError> {
    let own = std::env::current_exe()
        .map_err(|error| unavailable(format!("this service's own path is unknown: {error}")))?;
    let (helper, root) = placement(&own)
        .ok_or_else(|| unavailable(format!("{} is not in an install's bin", own.display())))?;
    if !helper.is_file() {
        return Err(unavailable(
            "this install keeps no uninstall helper, as an install made from the command line \
             does not; open Lys.app once and it keeps one",
        ));
    }
    Ok((helper, root))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let (_, root) = helper()?;
    Ok(Json(json!({
        "data_folder": root.display().to_string(),
        "lost": LOST,
    })))
}

async fn uninstall(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<UninstallBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    if body.remove_data && !body.confirmed {
        return Err(ServerError::RequestMalformed {
            reason: "removing the data folder needs confirmed: true, sent once the person has \
                     read what is lost"
                .to_string(),
        });
    }
    let (helper, root) = helper()?;
    let mut command = Command::new(&helper);
    command
        .args(helper_args(&root, body.remove_data))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    let child = command
        .spawn()
        .map_err(|error| unavailable(format!("{} could not start: {error}", helper.display())))?;
    (state.say)(&format!(
        "uninstall helper started as process {}, data folder {}",
        child.id(),
        if body.remove_data { "removed" } else { "kept" }
    ));
    Ok(Json(json!({
        "uninstalling": true,
        "remove_data": body.remove_data,
    })))
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn detach(_command: &mut Command) {}

#[cfg(test)]
#[path = "uninstall_api_tests.rs"]
mod tests;
