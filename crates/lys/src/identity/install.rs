//! `lys identity install`: the whole identity product from one command.
//!
//! The install makes its own data root under the user's application data
//! path and never writes outside it. Inside, it writes the deployment
//! configuration, materialises credentials, starts the compose services,
//! waits until they answer, registers both clients, generates the service
//! key and the secrets broker, writes the directory service's configuration
//! and starts the broker, the runner and the service. Running it again
//! changes only what is missing; nothing is rotated or restarted, and the
//! runner least of all, since a restart of it ends every session it holds.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use lys_core::Ed25519Identity;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::prepare::{API_KEY_NAME, API_KEY_SECRET, read_secret};
use super::private_files::Outcome;
use super::rauthy::RauthyApi;
use super::{configure, prepare, private_files};
use crate::commands::output::Emitter;

pub mod exit_wait;
pub mod layout;
pub mod server_config;
pub mod services;
pub mod surface;

use layout::{BROKER_PORT, Layout, SERVICE_PORT};

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

fn start_broker(layout: &Layout, emitter: &mut Emitter, replace: bool) -> IdentityResult<()> {
    let program = services::sibling("lys-secrets")?;
    let log = layout.logs_dir().join("secrets.log");
    let pid = layout.run_dir().join("secrets.pid");
    let args = [
        "serve",
        "--root",
        &layout.broker_root().display().to_string(),
        "--keys",
        &layout.broker_keys().display().to_string(),
        "--listen",
        &format!("127.0.0.1:{BROKER_PORT}"),
        "--directory-config",
        &layout.service_config().display().to_string(),
    ]
    .map(str::to_string);
    let started = services::start_detached(&program, &args, &log, &pid, replace)?;
    services::wait_answering(BROKER_PORT, "/", &pid, &log)?;
    emitter.note(&format!(
        "secrets broker {} on 127.0.0.1:{BROKER_PORT}",
        started_word(started, replace)
    ));
    Ok(())
}

/// Start the runner beside the service, as the service is started: it holds
/// each agent the service starts in its own pseudo-terminal, on a Unix socket
/// in the run folder, acting only on the service key's requests. It is never
/// restarted here, since a restart ends every session it holds.
fn start_runner(
    layout: &Layout,
    key: &Arc<Ed25519Identity>,
    emitter: &mut Emitter,
) -> IdentityResult<()> {
    let program = services::sibling("lys")?;
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
    ]
    .map(str::to_string);
    let started = services::start_detached(&program, &args, &log, &pid, false)?;
    let client = lys_runner::Client::new(socket.clone(), Arc::clone(key));
    services::wait_until("runner", &log, &pid, &mut || {
        client
            .ask(&lys_runner::Act::Status { session: None })
            .is_ok()
    })?;
    emitter.note(&format!(
        "runner {} on {}",
        started_word(started, false),
        socket.display()
    ));
    Ok(())
}

fn start_service(layout: &Layout, emitter: &mut Emitter, replace: bool) -> IdentityResult<()> {
    let program = services::sibling("lys-identity-server")?;
    let log = layout.logs_dir().join("identity.log");
    let pid = layout.run_dir().join("identity.pid");
    let args = [layout.service_config().display().to_string()];
    let started = services::start_detached(&program, &args, &log, &pid, replace)?;
    services::wait_answering(SERVICE_PORT, "/api/authority", &pid, &log)?;
    emitter.note(&format!(
        "identity service {} on {}",
        started_word(started, replace),
        Layout::service_url()
    ));
    Ok(())
}

/// Runs `lys identity install`.
pub fn run(options: &Options, json: bool) -> IdentityResult<()> {
    let layout = match &options.root {
        Some(root) => Layout::at(root.clone()),
        None => Layout::discover()?,
    };
    let mut emitter = Emitter::new(json);
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
    let key = Arc::new(service_key(&layout)?);
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
    start_broker(&layout, &mut emitter, changed)?;
    start_runner(&layout, &key, &mut emitter)?;
    start_service(&layout, &mut emitter, changed)?;
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
