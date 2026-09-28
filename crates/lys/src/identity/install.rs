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
//! nothing is rotated, and a placed binary is never replaced: that is
//! `lys identity upgrade`'s.
//!
//! Invariants: install first ends an upgrade stopped part-way. It refuses,
//! before stopping anything, to run from a build other than the one placed.
//! A process is restarted when its configuration changed or its binary was
//! just placed, so `install/build.json` always names what runs.

use std::path::{Path, PathBuf};

use base64::Engine;
use lys_core::Ed25519Identity;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::prepare::{API_KEY_NAME, API_KEY_SECRET, read_secret};
use super::private_files::Outcome;
use super::rauthy::RauthyApi;
use super::upgrade::{self, Compose, adopt, swap};
use super::{configure, prepare, private_files};
use crate::commands::output::Emitter;

pub mod exit_wait;
pub mod layout;
pub mod log_wait;
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

/// Runs `lys identity install`.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
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
        &layout::render_deployment(&options.admin_email),
    )? {
        emitter.note("deployment.toml written");
    }
    let config = DeploymentConfig::load(&config_path)?;
    prepare::run(&config_path, json)?;
    services::compose_up(&layout, &config)?;
    emitter.note("compose services up");
    services::wait_ready(&layout, &config, &mut |line| println!("{line}"))?;
    emitter.note("postgres, rauthy and spicedb ready");
    configure::run(&config_path, json)?;
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
