//! `lys identity install`: the whole identity product from one command.
//!
//! The install makes its own data root under the user's application data
//! path and never writes outside it. Inside, it writes the deployment
//! configuration, materialises credentials, starts the compose services,
//! waits until they answer, registers its own client, generates the service
//! key and the secrets broker, makes the directory service's own API key
//! (`directory_key`), writes the directory service's configuration and
//! starts the broker and the service. Running it again changes only
//! what is missing; nothing is rotated or restarted.
//!
//! The install makes no administrator and fills nothing about a person from
//! the machine. While no administrator exists it writes a one-time setup
//! code and hands it to the browser in the setup page's address
//! (`setup_code`), where the person makes the administrator. Everything it
//! says names Lys and its parts by what they do, never the issuer.
//!
//! The password policy is Lys's, from the deployment configuration: the
//! install writes it to the sign-in service and into the directory service's
//! configuration, so what the screens show is what is enforced.

use std::path::{Path, PathBuf};

use base64::Engine;
use lys_core::Ed25519Identity;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult, in_lys_words};
use super::private_files::Outcome;
use super::{configure, prepare, private_files};
use crate::commands::output::Emitter;

pub mod directory_key;
pub mod exit_wait;
pub mod layout;
pub mod server_config;
pub mod services;
pub mod setup_code;
pub mod surface;

use layout::{BROKER_PORT, Layout, SERVICE_PORT};
use server_config::Carried;

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

/// The administrator and the registered products an earlier run's service
/// configuration already named, so running the install again loses neither.
fn earlier(layout: &Layout) -> IdentityResult<Carried> {
    let path = layout.service_config();
    let Some(bytes) = private_files::read(&path)? else {
        return Ok(Carried::default());
    };
    let earlier: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "read",
            "identity.json",
            error.to_string(),
        )
        .at(&path)
    })?;
    let named = |value: Option<&serde_json::Value>| value.filter(|value| !value.is_null()).cloned();
    Ok(Carried {
        administrator: named(earlier.get("administrator")),
        products: named(earlier.pointer("/provider/clients")),
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
    configure::apply_password_policy(&config)?;
    emitter.note("Lys's password policy set");
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
    let service_key_file = directory_key::provide(&config)?;
    provider_key(&config)?;
    let carried = earlier(&layout)?;
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
    let changed = configuration != Outcome::Unchanged || service_key_file != Outcome::Unchanged;
    let code = if setup_code::has_administrator(&layout)? {
        None
    } else {
        let code = setup_code::generate();
        let first_run = setup_code::Purpose::FirstRun;
        setup_code::write_pending(&config.state_dir(), first_run, code.expose())?;
        Some(code)
    };
    start_broker(&layout, &mut emitter, changed)?;
    start_service(&layout, &mut emitter, changed)?;
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
