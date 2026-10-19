//! `lys identity install`: the whole identity product from one command.
//!
//! The install makes its own data root under the user's application data
//! path and never writes outside it. Inside, it writes the deployment
//! configuration, materialises credentials, starts the compose services,
//! waits until they answer, registers its own client, generates the service
//! key and the secrets broker, makes the directory service's own API key
//! (`directory_key`), writes the directory service's configuration,
//! places the broker's and the service's binaries in its own `bin/` and
//! starts them from there, and records the build running in
//! `install/build.json`. Running it again changes only what is missing;
//! nothing is rotated, and a placed binary is never replaced: that is
//! `lys identity upgrade`'s.
//!
//! Invariants: install first ends an upgrade stopped part-way. It refuses,
//! before stopping anything, to run from a build other than the one placed.
//! A process is restarted when its configuration changed or its binary was
//! just placed. A placed screen package also restarts the directory service
//! to reload its immutable cache.
//!
//! The install makes no administrator and fills nothing about a person from
//! the machine. While no administrator exists it writes a one-time setup
//! code and hands it to the browser in the setup page's address
//! (`setup_code`), where the person makes the administrator. Everything it
//! says names Lys and its parts by what they do, never the issuer.
//! The runner starts beside the service and is not restarted by reinstall;
//! nor is the model proxy, which starts after it (`proxy`).
//!
//! The password policy is Lys's, from the deployment configuration: the
//! install writes it to the sign-in service and into the directory service's
//! configuration, so what the screens show is what is enforced.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_core::Ed25519Identity;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult, in_lys_words};
use super::private_files::Outcome;
use super::upgrade::{self, Compose, adopt, swap};
use super::{configure, prepare, private_files};
use crate::commands::output::Emitter;

pub mod broker_trust;
pub mod directory_key;
pub mod exit_wait;
pub mod layout;
pub mod log_wait;
pub mod login;
pub mod ports;
pub mod proxy;
pub mod server_config;
pub mod services;
pub mod settings;
pub mod setup_code;
pub mod surface;

use layout::{BINARIES, Layout};

mod security;
use security::{operator_token, provider_key};

mod options;
pub use options::{Options, Profile};

fn write_plain(path: &Path, text: &str) -> IdentityResult<()> {
    std::fs::write(path, text).map_err(|error| {
        IdentityError::new(
            ErrorKind::PrivateFileIo,
            "write",
            "install",
            error.to_string(),
        )
        .at(path)
    })
}

fn write_absent(path: &Path, text: &str) -> IdentityResult<bool> {
    if path.exists() {
        return Ok(false);
    }
    write_plain(path, text)?;
    Ok(true)
}

/// The shipped permission model, when the model file at `path` is absent
/// or holds an earlier version of it; none when any other file is there:
/// the running service applies a later version over the one it holds.
#[must_use]
pub fn model_replacement(path: &Path) -> Option<String> {
    let held = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|model| model.get("version").and_then(serde_json::Value::as_u64));
    match held {
        Some(version) if version < lys_identity::grants::SHIPPED_VERSION => {}
        _ if path.exists() => return None,
        _ => {}
    }
    Some(lys_identity::grants::shipped_model())
}

/// Writes the shipped permission model where [`model_replacement`] names
/// it.
fn write_model(path: &Path) -> IdentityResult<bool> {
    let Some(shipped) = model_replacement(path) else {
        return Ok(false);
    };
    write_plain(path, &shipped)?;
    Ok(true)
}

pub(super) fn service_key(layout: &Layout) -> IdentityResult<Ed25519Identity> {
    let path = layout.service_key();
    // Every joined computer's saved server key names this one: one the
    // stores receipt counts is never minted anew (DIRECTORY-107 R3).
    if !path.exists() && super::accept_loss::counts(layout, "service-key")? {
        return Err(IdentityError::new(
            ErrorKind::StoreMissing,
            "generate service key",
            "keys/service.key",
            "the stores receipt counts this key and it is absent, so no new key was made; every joined computer names the old one. Put the key back, or run `lys identity accept-loss --store service-key` to start at a new one",
        )
        .at(&path));
    }
    if let Some(parent) = path.parent() {
        private_files::ensure_dir(parent)?;
    }
    Ed25519Identity::load_or_generate(&path).map_err(|error| {
        IdentityError::new(
            ErrorKind::PrivateFileIo,
            "generate service key",
            "service key",
            error.to_string(),
        )
        .at(&path)
    })
}

/// Start the runner beside the service, as the service is started: it holds
/// each agent the service starts in its own pseudo-terminal, on a Unix socket
/// in the run folder, acting only on the service key's requests. It is never
/// restarted here, since a restart ends every session it holds.
/// Places the runner, `lys`, in the install's `bin/` from the build folder
/// `source` names, with its containment helper beside it when that folder
/// holds one, and says what was placed. The runner starts a contained seat
/// through `lys runner managed-entry` from its own executable; the helper
/// is placed beside it so `bin/` holds the same pair as the host's client
/// release under `clients/`. The runner is started from `bin/`
/// only: a runner started from the build folder re-executes itself from
/// there for every seat it owns, and `lys identity start` found no `bin/lys`
/// after a fresh install on 15 Oct 2026. Answers whether the helper was placed.
pub(super) fn place_runner(
    layout: &Layout,
    source: &dyn Fn(&str) -> IdentityResult<PathBuf>,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<bool> {
    swap::ensure_bin(layout)?;
    swap::place_binary(&source("lys")?, &layout.bin_dir(), "lys")?;
    say(&format!("lys placed in {}", layout.bin_dir().display()));
    let helper = "lys-containment-entry";
    let companion = source(helper)?;
    if companion.is_file() {
        swap::place_binary(&companion, &layout.bin_dir(), helper)?;
        say(&format!(
            "{helper} placed in {}",
            layout.bin_dir().display()
        ));
        return Ok(true);
    }
    say(&format!(
        "{helper} is not in the build folder beside lys, so it was not placed beside lys in {}",
        layout.bin_dir().display()
    ));
    Ok(false)
}

/// A computer that joined another Lys keeps its saved join in the run
/// folder: an install here would write this install's server key over the
/// joined server's, so it is refused by name and nothing is changed
/// (DIRECTORY-107 R3). A join to this install itself, whose saved server
/// key is this install's own, is not another Lys and is left as it is.
fn refuse_saved_join(layout: &Layout, own: &str) -> IdentityResult<()> {
    let saved = layout.run_dir().join("runner-join.json");
    let trusted = std::fs::read_to_string(layout.run_dir().join("runner-server.pub")).ok();
    if trusted.is_some_and(|key| key.trim() == own) {
        return Ok(());
    }
    let bytes = match std::fs::read(&saved) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(IdentityError::new(
                ErrorKind::PrivateFileIo,
                "read saved join",
                "runner-join.json",
                error.to_string(),
            )
            .at(&saved));
        }
    };
    let server = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|join| join["server"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "a server it does not name readably".to_owned());
    Err(IdentityError::new(
        ErrorKind::RunnerJoinPresent,
        "start runner",
        "runner-join.json",
        format!(
            "this computer keeps a saved join to {server} in {}; installing Lys here would turn it into a server of its own, so nothing was changed",
            saved.display()
        ),
    )
    .at(&saved))
}

pub(super) fn start_runner(
    layout: &Layout,
    key: &Arc<Ed25519Identity>,
    program: &Path,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    refuse_saved_join(layout, &lys_runner::protocol::hex(&key.public_key_bytes()))?;
    for directory in [layout.run_dir(), layout.logs_dir(), layout.data_dir()] {
        private_files::ensure_dir(&directory)?;
    }
    // The seat owners the runner starts live under its state and outlive
    // it (AGENTS-004 R1); the directory is made once and kept by upgrades.
    private_files::ensure_dir(
        &layout
            .data_dir()
            .join("runner")
            .join(lys_runner::seat_owner::protocol::OWNERS_DIR),
    )?;
    let log = layout.logs_dir().join("runner.log");
    let pid = layout.run_dir().join("runner.pid");
    let public = layout.run_dir().join("runner-server.pub");
    write_plain(&public, &lys_runner::protocol::hex(&key.public_key_bytes()))?;
    let socket = layout.runner_socket();
    let args = [
        "runner",
        "serve",
        "--socket",
        &socket.display().to_string(),
        "--state",
        &layout.data_dir().join("runner").display().to_string(),
        "--server-key",
        &public.display().to_string(),
        // Where the install's proxy keeps its state (`proxy::unit`).
        "--proxy-state",
        &layout
            .data_dir()
            .join("proxy")
            .join("state")
            .display()
            .to_string(),
    ]
    .map(str::to_string);
    let started = services::start_detached(program, &args, &log, &pid, false)?;
    let client = lys_runner::Client::new(socket.clone(), Arc::clone(key));
    log_wait::wait_until("runner", &log, &pid, &mut || {
        client
            .ask(&lys_runner::Act::Status { session: None })
            .is_ok()
    })?;
    say(&format!(
        "runner {} on {}",
        if started {
            "started"
        } else {
            "already running"
        },
        socket.display()
    ));
    Ok(())
}

/// Start whatever of the install under `layout` is down, with the steps the
/// install starts it with, leaving running whatever runs: an upgrade stopped
/// part-way is ended first, then the compose services, the broker and the
/// service, the runner and the model proxy, each waited on until ready.
/// The inverse of `lys identity stop`.
///
/// # Errors
/// Returns named configuration, key, placement or service-start failures.
pub fn start_installed(
    layout: &Layout,
    ports: ports::Ports,
    units: &[upgrade::Unit],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    super::accept_loss::check_before(layout, "start")?;
    swap::recover(layout, units, &mut Compose, say)?;
    services::require_docker(Path::new("docker"))?;
    let config = DeploymentConfig::load_install(&layout.deployment_config())?;
    services::compose_up(layout, &config)?;
    say("database, sign-in and permission services up");
    services::wait_ready(layout, &config, &mut |line| say(&in_lys_words(line)))?;
    say("database, sign-in and permission services ready");
    for unit in units {
        let started = upgrade::launch(layout, unit, false)?;
        say(&format!(
            "{} {}",
            unit.binary,
            if started {
                "started"
            } else {
                "already running"
            }
        ));
    }
    let key = Arc::new(service_key(layout)?);
    start_runner(layout, &key, &layout.binary("lys"), say)?;
    proxy::start(layout, ports, &layout.binary("lys"), say)
}

/// Makes every file the directory service reads from the state folder that
/// is not there yet, leaving each one that is: the grant model, the
/// providers' key, the provider signing key and the service key. Install
/// and upgrade both run it, so an install made by an earlier build gains
/// what this build's service needs before it starts.
///
/// # Errors
/// Returns named configuration, key, placement or service-start failures.
pub fn server_state(layout: &Layout, config: &DeploymentConfig) -> IdentityResult<()> {
    write_model(&layout.grant_model())?;
    server_keys(layout, config)
}

/// As [`server_state`] but the grant model, which an upgrade places as one
/// of the files it replaces, so that a put-back by any installer returns
/// the model the previous build read.
///
/// # Errors
/// Returns named configuration, key, placement or service-start failures.
pub fn server_keys(layout: &Layout, config: &DeploymentConfig) -> IdentityResult<()> {
    provider_key(config)?;
    service_key(layout)?;
    layout::runner_machine::provide(layout, &mut lys_runner::error::said)?;
    super::import::prepare_credential(layout)?;
    Ok(())
}

/// Runs `lys identity install`. A failure is said in Lys's words, because
/// the person installing reads it.
///
/// # Errors
/// Returns named configuration, key, placement or service-start failures.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
    command::install(options, json).map_err(IdentityError::said_in_lys_words)
}

mod command;

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "install/presence_tests.rs"]
mod presence_tests;

#[cfg(test)]
#[path = "install/runner_join_guard_tests.rs"]
mod runner_join_guard_tests;
