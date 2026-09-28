//! `lys identity install`: the whole identity product from one command.
//!
//! The install makes its own data root under the user's application data
//! path and never writes outside it. Inside, it writes the deployment
//! configuration, materialises credentials, starts the compose services,
//! waits until they answer, registers its own client, generates the service
//! key and the secrets broker, writes the directory service's configuration,
//! places the broker's and the service's binaries in its own `bin/` and
//! starts them from there, and records the build running in
//! `install/build.json`. Running it again changes only what is missing;
//! nothing is rotated, and a placed binary is never replaced: that is
//! `lys identity upgrade`'s.
//!
//! Invariants: install first ends an upgrade stopped part-way. It refuses,
//! before stopping anything, to run from a build other than the one placed.
//! A process is restarted when its configuration changed or its binary was
//! just placed, so `install/build.json` always names what runs.
//!
//! The install makes no administrator and fills nothing about a person from
//! the machine. While no administrator exists it writes a one-time setup
//! code and hands it to the browser in the setup page's address
//! (`setup_code`), where the person makes the administrator. Everything it
//! says names Lys and its parts by what they do, never the issuer.

use std::path::{Path, PathBuf};

use base64::Engine;
use lys_core::Ed25519Identity;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult, in_lys_words};
use super::prepare::{API_KEY_NAME, API_KEY_SECRET, read_secret};
use super::private_files::Outcome;
use super::upgrade::{self, Compose, adopt, swap};
use super::{configure, prepare, private_files};
use crate::commands::output::Emitter;

pub mod exit_wait;
pub mod layout;
pub mod log_wait;
pub mod server_config;
pub mod services;
pub mod setup_code;
pub mod surface;

use layout::{BINARIES, Layout};

/// What the operator chose.
#[derive(Debug)]
pub struct Options {
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
    /// For an unattended install, the administrator's email: the only one
    /// the setup page then takes. Never read from the machine.
    pub admin_email: Option<String>,
    /// A compiled screens package to verify and place.
    pub surface: Option<PathBuf>,
}

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

/// The key Lys signs products' ID tokens with, kept in the state folder.
fn provider_key(config: &DeploymentConfig) -> IdentityResult<()> {
    let path = config.state_dir().join(server_config::PROVIDER_KEY_FILE);
    Ed25519Identity::load_or_generate(&path).map_err(|error| {
        IdentityError::new(
            ErrorKind::PrivateFileIo,
            "generate provider key",
            "provider key",
            error.to_string(),
        )
        .at(&path)
    })?;
    Ok(())
}

fn service_key(layout: &Layout) -> IdentityResult<Ed25519Identity> {
    let path = layout.service_key();
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

/// Writes the API key the sign-in providers route calls Rauthy with, as
/// Rauthy reads it: the key's name, a dollar sign and its secret.
fn write_providers_key(config: &DeploymentConfig) -> IdentityResult<()> {
    let state = config.state_dir();
    let credential = read_secret(&state, API_KEY_SECRET)?;
    let joined = zeroize::Zeroizing::new(format!("{API_KEY_NAME}${}", credential.expose()));
    private_files::write(
        &state.join(server_config::PROVIDERS_KEY_FILE),
        joined.as_bytes(),
    )?;
    Ok(())
}

/// Runs `lys identity install`. A failure is said in Lys's words, because
/// the person installing reads it.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
    install(options, json).map_err(IdentityError::said_in_lys_words)
}

fn install(options: &Options, json: bool) -> IdentityResult<()> {
    let layout = match &options.root {
        Some(root) => Layout::at(root.clone()),
        None => Layout::discover()?,
    };
    let mut emitter = Emitter::new(json);
    let units = upgrade::units(&layout);
    swap::recover(&layout, &units, &mut Compose, &mut |line| {
        emitter.note(line);
    })?;
    adopt::check_placed_build(&layout, &BINARIES, &services::sibling)?;
    services::require_docker(Path::new("docker"))?;
    private_files::ensure_dir(&layout.root)?;
    for dir in [
        layout.deploy_dir(),
        layout.logs_dir(),
        layout.run_dir(),
        layout.data_dir(),
    ] {
        private_files::ensure_dir(&dir)?;
    }
    emitter.field("root", "root", layout.root.display().to_string());
    write_plain(
        &layout.deploy_dir().join("compose.yaml"),
        layout::COMPOSE_YAML,
    )?;
    write_plain(
        &layout.deploy_dir().join("postgres-init.sql"),
        layout::POSTGRES_INIT_SQL,
    )?;
    let config_path = layout.deployment_config();
    if write_absent(
        &config_path,
        &layout::render_deployment(options.admin_email.as_deref()),
    )? {
        emitter.note("deployment.toml written");
    }
    let config = DeploymentConfig::load(&config_path)?;
    let materialised = prepare::materialise_all(&config)?;
    emitter.note(&format!(
        "{} private files ready (only you can read them)",
        materialised.len()
    ));
    services::compose_up(&layout, &config)?;
    emitter.note("database, sign-in and permission services up");
    services::wait_ready(&layout, &config, &mut |line| {
        println!("{}", in_lys_words(line));
    })?;
    emitter.note("database, sign-in and permission services ready");
    configure::reconcile(&config)?;
    emitter.note("sign-in clients registered");
    let key = service_key(&layout)?;
    let public = base64::engine::general_purpose::STANDARD.encode(key.public_key_bytes());
    emitter.note(&format!("service key public {public}"));
    if services::broker_init(&layout)? {
        emitter.note("secrets broker made");
    }
    if let Some(package) = &options.surface {
        let manifest = surface::place(package, &layout.surface_dir())?;
        emitter.note(&format!("screens placed from commit {}", manifest.commit));
    }
    let surface_present = layout.surface_dir().join("index.html").is_file();
    write_absent(&layout.grant_model(), layout::GRANT_MODEL)?;
    write_providers_key(&config)?;
    provider_key(&config)?;
    let carried = server_config::carried(&layout)?.unwrap_or_default();
    let rendered = server_config::render(&layout, &config, &carried, surface_present);
    let encoded = serde_json::to_vec_pretty(&rendered).map_err(|error| {
        IdentityError::new(
            ErrorKind::RenderFailed,
            "render",
            "identity.json",
            error.to_string(),
        )
    })?;
    let configuration = private_files::write(&layout.service_config(), &encoded)?;
    let changed = configuration != Outcome::Unchanged;
    let code = if setup_code::has_administrator(&layout)? {
        None
    } else {
        let code = setup_code::generate();
        let first_run = setup_code::Purpose::FirstRun;
        setup_code::write_pending(&config.state_dir(), first_run, code.expose())?;
        Some(code)
    };
    let build = adopt::settle(&layout, &units, &services::sibling, changed, &mut |line| {
        emitter.note(line);
    })?;
    let build = serde_json::to_value(&build).map_err(|error| {
        IdentityError::new(
            ErrorKind::RenderFailed,
            "render",
            "build.json",
            error.to_string(),
        )
    })?;
    if emitter.is_json() {
        emitter.field("build", "build", build);
    }
    emitter.field("open", "url", Layout::service_url());
    emitter.field("sign-in for products", "issuer", Layout::service_url());
    if let Some(code) = code {
        emitter.field("setup", "setup", Layout::setup_url());
        setup_code::hand_over(
            &layout,
            code.expose(),
            &setup_code::open_in_browser,
            &mut emitter,
        )?;
    }
    if !surface_present {
        emitter.note("no screens placed: pass --surface with a compiled screens package");
    }
    emitter.finish();
    Ok(())
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
