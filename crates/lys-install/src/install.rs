//! `lys identity install`: the whole identity product from one command.
//!
//! The install makes its own data root under the user's application data
//! path and never writes outside it. Inside, it writes the deployment
//! configuration, materialises credentials, starts the compose services,
//! waits until they answer, registers both clients, generates the service
//! key and the secrets broker, writes the directory service's configuration,
//! places the broker's and the service's binaries in its own `bin/` and
//! starts them from there, and records the build running in
//! `install/build.json`. Running it again changes only what is missing;
//! nothing is rotated, a placed binary is never replaced (that is
//! `lys identity upgrade`'s), and nothing is restarted unless its
//! configuration changed.

use std::path::{Path, PathBuf};

use base64::Engine;
use lys_core::Ed25519Identity;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::prepare::{API_KEY_NAME, API_KEY_SECRET, read_secret};
use super::private_files::Outcome;
use super::rauthy::RauthyApi;
use super::steps::Step;
use super::{configure, prepare, private_files, upgrade};
use crate::output::Emitter;

pub mod engine_path;
pub mod exit_wait;
pub mod layout;
pub mod server_config;
pub mod services;
pub mod surface;

use layout::{BINARIES, Layout};

/// What the operator chose.
#[derive(Debug)]
pub struct Options {
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
    /// The administrator's email, the identity Rauthy bootstraps.
    pub admin_email: String,
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

fn administrator_subject(config: &DeploymentConfig) -> IdentityResult<String> {
    let credential = read_secret(&config.state_dir(), API_KEY_SECRET)?;
    let api = RauthyApi::new(&config.issuer.admin_url, Some(credential))?;
    api.user_id(&config.deployment.admin_email)?.ok_or_else(|| {
        IdentityError::new(
            ErrorKind::Unready,
            "find administrator",
            "rauthy",
            format!(
                "Rauthy holds no user for {}; it bootstraps one on its first start only",
                config.deployment.admin_email
            ),
        )
    })
}

/// How a start is reported: a process that was replaced was restarted.
fn started_word(started: bool, replace: bool) -> &'static str {
    match (started, replace) {
        (true, true) => "restarted with its new configuration",
        (true, false) => "started",
        (false, _) => "already running",
    }
}

/// Places each binary the install runs into `bin/` from beside the running
/// `lys`, when it is not there yet. One already there is never replaced:
/// that is `lys identity upgrade`'s.
fn place_binaries(layout: &Layout, emitter: &mut Emitter) -> IdentityResult<()> {
    private_files::ensure_dir(&layout.bin_dir())?;
    for name in BINARIES {
        if layout.binary(name).is_file() {
            continue;
        }
        let source = services::sibling(name)?;
        upgrade::place_binary(&source, &layout.bin_dir(), name)?;
        emitter.note(&format!("{name} placed in {}", layout.bin_dir().display()));
    }
    Ok(())
}

/// Starts the broker and then the service from `bin/`, each waited on for
/// ready, restarting a running one when `replace` asks.
fn start_units(layout: &Layout, emitter: &mut Emitter, replace: bool) -> IdentityResult<()> {
    for unit in upgrade::units(layout) {
        let started = upgrade::launch(layout, &unit, replace)?;
        let at = match unit.ready.answers {
            Some((port, _)) => format!(" on 127.0.0.1:{port}"),
            None => String::new(),
        };
        emitter.note(&format!(
            "{} {}{at}",
            unit.binary,
            started_word(started, replace)
        ));
    }
    Ok(())
}

/// Runs the install: `lys identity install`, and Lys.app's in its own
/// process. `step` hears each [`Step`] as it begins, in [`Step::ALL`]'s
/// order; a failure belongs to the last step it heard, and running the
/// install again resumes it, since only what is missing is made.
pub fn run(options: &Options, json: bool, step: &mut dyn FnMut(Step)) -> IdentityResult<()> {
    let layout = match &options.root {
        Some(root) => Layout::at(root.clone()),
        None => Layout::discover()?,
    };
    let mut emitter = Emitter::new(json);
    step(Step::Engine);
    services::require_docker(Path::new(engine_path::DOCKER))?;
    step(Step::Directory);
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
        &layout::render_deployment(&options.admin_email),
    )? {
        emitter.note("deployment.toml written");
    }
    let config = DeploymentConfig::load(&config_path)?;
    prepare::run(&config_path, json)?;
    step(Step::SignIn);
    services::compose_up(&layout, &config)?;
    emitter.note("compose services up");
    services::wait_ready(&layout, &config, &mut |line| println!("{line}"))?;
    emitter.note("postgres, rauthy and spicedb ready");
    step(Step::Clients);
    configure::run(&config_path, json)?;
    step(Step::Keys);
    let key = service_key(&layout)?;
    let public = base64::engine::general_purpose::STANDARD.encode(key.public_key_bytes());
    emitter.note(&format!("service key public {public}"));
    if services::broker_init(&layout)? {
        emitter.note("secrets broker made");
    }
    step(Step::Screens);
    if let Some(package) = &options.surface {
        let manifest = surface::place(package, &layout.surface_dir())?;
        emitter.note(&format!("screens placed from commit {}", manifest.commit));
    }
    let surface_present = layout.surface_dir().join("index.html").is_file();
    step(Step::Start);
    write_absent(&layout.grant_model(), layout::GRANT_MODEL)?;
    write_providers_key(&config)?;
    let subject = administrator_subject(&config)?;
    let rendered = server_config::render(&layout, &config, &subject, surface_present);
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
    place_binaries(&layout, &mut emitter)?;
    start_units(&layout, &mut emitter, changed)?;
    step(Step::Build);
    let build = upgrade::record_build(&layout, &BINARIES, &mut |line| emitter.note(line))?;
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
    emitter.field(
        "administrator",
        "administrator",
        config.deployment.admin_email.clone(),
    );
    emitter.field(
        "administrator password file",
        "administrator_password_file",
        config
            .state_dir()
            .join("rauthy-admin-password")
            .display()
            .to_string(),
    );
    if !surface_present {
        emitter.note("no screens placed: pass --surface with a compiled screens package");
    }
    emitter.finish();
    Ok(())
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
