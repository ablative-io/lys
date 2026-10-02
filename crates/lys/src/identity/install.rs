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
//! The runner starts beside the service and is not restarted by reinstall.
//!
//! The password policy is Lys's, from the deployment configuration: the
//! install writes it to the sign-in service and into the directory service's
//! configuration, so what the screens show is what is enforced.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
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
pub mod ports;
pub mod server_config;
pub mod services;
pub mod setup_code;
pub mod surface;

use layout::{BINARIES, Layout};

/// Which kind of install this is. A service install keeps no operator token:
/// nothing on disk can act as the administrator. A development install keeps
/// one for the person developing on it (Tom, 3 Oct 2026: "not just like a
/// dev install, but like a particular profile").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Profile {
    /// The install people depend on.
    #[default]
    Service,
    /// An install being developed on.
    Development,
}

impl Profile {
    /// The word the service's configuration carries.
    pub fn word(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Development => "development",
        }
    }

    /// The profile a configuration's word names; none for any other word.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "service" => Some(Self::Service),
            "development" => Some(Self::Development),
            _ => None,
        }
    }
}

/// What the operator chose.
#[derive(Debug)]
pub struct Options {
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
    /// Which kind of install this is; absent, an earlier install's profile
    /// is kept, else service.
    pub profile: Option<Profile>,
    /// For an unattended install, the administrator's email: the only one
    /// the setup page then takes. Never read from the machine.
    pub admin_email: Option<String>,
    /// A compiled screens package to verify and place.
    pub surface: Option<PathBuf>,
    /// The message service connection to keep in the configuration.
    pub message_service: Option<PathBuf>,
    /// An explicit local identity listener; otherwise keep the recorded listener.
    pub service_port: Option<u16>,
    /// An explicit local broker listener; otherwise keep the recorded listener.
    pub broker_port: Option<u16>,
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

/// The shipped permission model, when the model file at `path` is absent
/// or holds an earlier version of it; none when any other file is there:
/// the running service applies a later version over the one it holds.
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

pub(super) fn service_key(layout: &Layout) -> IdentityResult<Ed25519Identity> {
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

/// Start the runner beside the service, as the service is started: it holds
/// each agent the service starts in its own pseudo-terminal, on a Unix socket
/// in the run folder, acting only on the service key's requests. It is never
/// restarted here, since a restart ends every session it holds.
pub(super) fn start_runner(
    layout: &Layout,
    key: &Arc<Ed25519Identity>,
    program: &Path,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    for directory in [layout.run_dir(), layout.logs_dir(), layout.data_dir()] {
        private_files::ensure_dir(&directory)?;
    }
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

/// Makes every file the directory service reads from the state folder that
/// is not there yet, leaving each one that is: the grant model, the
/// providers' key, the provider signing key and the service key. Install
/// and upgrade both run it, so an install made by an earlier build gains
/// what this build's service needs before it starts.
pub fn server_state(layout: &Layout, config: &DeploymentConfig) -> IdentityResult<()> {
    write_model(&layout.grant_model())?;
    server_keys(layout, config)
}

/// As [`server_state`] but the grant model, which an upgrade places as one
/// of the files it replaces, so that a put-back by any installer returns
/// the model the previous build read.
pub fn server_keys(layout: &Layout, config: &DeploymentConfig) -> IdentityResult<()> {
    provider_key(config)?;
    service_key(layout)?;
    super::import::prepare_credential(layout)?;
    Ok(())
}

/// The install's operator token: under the development profile, made once
/// and kept owner-only; under the service profile, removed if one stands,
/// so nothing on disk can act as the administrator. Says what it did.
fn operator_token(
    config: &DeploymentConfig,
    profile: Profile,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let path = config.state_dir().join(server_config::OPERATOR_TOKEN_FILE);
    match (profile, path.exists()) {
        (Profile::Development, true) | (Profile::Service, false) => Ok(()),
        (Profile::Development, false) => {
            let token = setup_code::generate();
            private_files::write(&path, token.expose().as_bytes())?;
            say("operator token made: development profile");
            Ok(())
        }
        (Profile::Service, true) => {
            std::fs::remove_file(&path).map_err(|error| {
                IdentityError::new(
                    ErrorKind::PrivateFileIo,
                    "remove",
                    "operator token",
                    error.to_string(),
                )
                .at(&path)
            })?;
            say("operator token removed: a service install keeps none");
            Ok(())
        }
    }
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
    let ports = ports::Ports::load(&layout)?.chosen(options.service_port, options.broker_port)?;
    let units = upgrade::units_at(&layout, ports);
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
    let deployment = if ports.service == layout::SERVICE_PORT {
        layout::render_deployment(options.admin_email.as_deref())
    } else {
        layout::render_deployment_at(options.admin_email.as_deref(), ports.service)
    };
    if write_absent(&config_path, &deployment)? {
        emitter.note("deployment.toml written");
    }
    let config = DeploymentConfig::load_install(&config_path)?;
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
    let key = Arc::new(service_key(&layout)?);
    let public = base64::engine::general_purpose::STANDARD.encode(key.public_key_bytes());
    emitter.note(&format!("service key public {public}"));
    if services::broker_init(&layout)? {
        emitter.note("secrets broker made");
    }
    broker_trust::trust(&layout, server_config::SERVICE_NAME)?;
    emitter.note("secrets broker trusts the identity service");
    if let Some(package) = &options.surface {
        let manifest = surface::place(package, &layout.surface_dir())?;
        emitter.note(&format!("screens placed from commit {}", manifest.commit));
    }
    let surface_present = layout.surface_dir().join("index.html").is_file();
    server_state(&layout, &config)?;
    let service_key_file = directory_key::provide(&config)?;
    private_files::write(
        &layout.data_dir().join("estate-approval.json"),
        layout::ESTATE_PLAN.as_bytes(),
    )?;
    let mut carried = server_config::carried(&layout)?.unwrap_or_default();
    carried.ports = ports;
    let profile = options.profile.or(carried.profile).unwrap_or_default();
    carried.profile = Some(profile);
    emitter.note(&format!("profile: {}", profile.word()));
    operator_token(&config, profile, &mut |line| emitter.note(line))?;
    if let Some(path) = &options.message_service {
        carried.message_service = Some(server_config::messages_from(path)?);
        emitter.note("message service connection kept");
    }
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
    let build = adopt::settle(
        &layout,
        &units,
        &services::sibling,
        changed,
        options.surface.is_some(),
        &mut |line| {
            emitter.note(line);
        },
    )?;
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
    start_runner(&layout, &key, &services::sibling("lys")?, &mut |line| {
        emitter.note(line);
    })?;
    emitter.field("open", "url", ports.service_url());
    emitter.field("sign-in for products", "issuer", ports.service_url());
    if let Some(code) = code {
        emitter.field("setup", "setup", ports.setup_url());
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
