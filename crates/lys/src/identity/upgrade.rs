//! `lys identity upgrade`: a running install moved to a newer build, with
//! the previous build kept to return to.
//!
//! The install runs the secrets broker and the directory service from its
//! own `bin/`, with the configuration and compose files its build renders.
//! An upgrade first ends any upgrade stopped part-way ([`swap::recover`]).
//! It then reads every binary's `--version`, the new and the installed (an
//! install made before builds were named is adopted, [`adopt`]), and renders
//! the new build's four configuration and compose files ([`render`]):
//! `deploy/compose.yaml`, `deploy/postgres-init.sql`, `compose.env` in the
//! state directory and the directory service's `identity.json`. It does so
//! before it stops anything; it refuses by name when the root holds no
//! install, when the folder lacks a binary, or when a new binary's version
//! cannot be read. It writes its intent record ([`intent`]) and swaps the
//! build in, each step recorded as it completes ([`swap`]): the service and
//! the broker stopped, each waited on for its exit; `bin/` kept in
//! `bin.previous/` and the new binaries placed; those four files kept in
//! `config.previous/` and the new ones placed; the screens kept in
//! `surface.previous/` and the new ones placed; the compose services
//! brought to a changed definition; and the broker and then the service
//! started, each waited on for ready.
//! The runner participates in the same placement and rollback. Its Status
//! is read before recovery and again before stopping it: live sessions or
//! an unreadable Status refuse the upgrade. Its replacement must say it
//! listens and answer Status while still holding its own exit lock.
//!
//! Invariants: nothing is stopped until every input has been read, checked
//! and rendered. When a step, a start or a readiness fails, what was started
//! is stopped, the previous binaries, files and screens are put back and
//! started and waited on for ready, and the upgrade fails naming the binary
//! and its log. The client releases are placed inside the same transaction,
//! after the intent record and before anything is stopped, and a put-back
//! removes exactly the files and directories the attempt made, so a refused
//! upgrade leaves `clients/` as it found it (DIRECTORY-115 R1). An upgrade
//! writes only `bin/`, `bin.previous/`, those four rendered files,
//! `config.previous/`, the screens, `clients/`, the logs, the process
//! files, `install/` and, once, `data.previous-organisation/`, the copy of
//! the directory and grant logs made before the first boot under
//! organisations rewrites them (DIRECTORY-109 R2): never `data/`, a
//! credential file or `deployment.toml`. `compose.env` carries the credentials as they are
//! stored; it is rendered from them and is not itself a credential file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::install::layout::{BINARIES, Layout};
use super::install::log_wait::{self, LogCursor};
use super::install::ports::Ports;
use super::install::{self, login, services, surface};
use super::private_files;

pub mod adopt;
mod command;
mod companion;
#[cfg(test)]
use command::ClientRender;
pub mod data_kept;
pub mod intent;
mod record;
pub mod render;
mod runner;
pub mod swap;
pub use command::{Options, run};
pub use record::{BuildRecord, SurfaceBuild, record_build};

use intent::{Intent, Step};
use render::Render;

/// How a started process is known to be ready: its log gains `says` after
/// it starts, and then, when `answers` names a loopback port and path, that
/// port answers the path. Both are checked once on each change to the log.
#[derive(Debug, Clone)]
pub struct Ready {
    /// The text the process writes once it is listening.
    pub says: String,
    /// The loopback port and path that answer once it is ready.
    pub answers: Option<(u16, &'static str)>,
}

/// One process the install runs from its `bin/`.
#[derive(Debug, Clone)]
pub struct Unit {
    /// The binary's name in `bin/`.
    pub binary: &'static str,
    /// Its arguments.
    pub args: Vec<String>,
    /// Where its output is appended.
    pub log: PathBuf,
    /// Where its pid is kept, its exit lock and its log offset beside it.
    pub pid: PathBuf,
    /// How it is known to be ready.
    pub ready: Ready,
}

/// The broker and the service as the install runs them, in start order.
///
/// # Errors
/// Returns named install-state, service-control or upgrade-phase failures.
pub fn units(layout: &Layout) -> IdentityResult<Vec<Unit>> {
    Ok(units_at(layout, Ports::load(layout)?))
}

/// The broker and service with explicitly selected listeners.
pub fn units_at(layout: &Layout, ports: Ports) -> Vec<Unit> {
    let broker_args = [
        "serve",
        "--root",
        &layout.broker_root().display().to_string(),
        "--keys",
        &layout.broker_keys().display().to_string(),
        "--listen",
        &format!("127.0.0.1:{}", ports.broker),
        "--directory-config",
        &layout.service_config().display().to_string(),
    ]
    .map(str::to_string);
    vec![
        Unit {
            binary: BINARIES[0],
            args: broker_args.to_vec(),
            log: layout.logs_dir().join("secrets.log"),
            pid: layout.run_dir().join("secrets.pid"),
            ready: Ready {
                says: "listening on".to_string(),
                answers: Some((ports.broker, "/")),
            },
        },
        Unit {
            binary: BINARIES[1],
            args: vec![layout.service_config().display().to_string()],
            log: layout.logs_dir().join("identity.log"),
            pid: layout.run_dir().join("identity.pid"),
            ready: Ready {
                says: "listening on".to_string(),
                answers: Some((ports.service, "/api/authority")),
            },
        },
    ]
}

/// What an upgrade asks of the engine that runs the compose services.
pub trait Engine {
    /// Brings the compose services to the definition now in place,
    /// making every service again from it, and waits for them ready.
    ///
    /// # Errors
    /// Returns named install-state, service-control or upgrade-phase failures.
    fn apply(&mut self, layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<()>;
    /// Once the new build is running: writes what `deployment.toml` declares
    /// to the sign-in service, as an install does, so an upgrade never leaves
    /// the sign-in service on the policy of an earlier build. On 15 Oct 2026
    /// a lowered password length stayed at the old fifteen through an upgrade.
    ///
    /// # Errors
    /// Returns named install-state, service-control or upgrade-phase failures.
    fn settle(&mut self, layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<()>;
}

/// The compose services through `docker compose`, every one made again from
/// its rendered definition, so a changed network reaches each of them.
#[derive(Debug)]
pub struct Compose;

impl Engine for Compose {
    fn apply(&mut self, layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
        let config = DeploymentConfig::load_install(&layout.deployment_config())?;
        services::compose_recreate(layout, &config)?;
        services::wait_ready(layout, &config, say)?;
        say("compose services on their rendered definition and ready");
        Ok(())
    }

    fn settle(&mut self, layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
        let config = DeploymentConfig::load_install(&layout.deployment_config())?;
        let policy = &config.password_policy;
        super::configure::apply_password_policy(&config)?;
        say(&format!(
            "password policy on the sign-in service: {} to {} characters, as deployment.toml declares",
            policy.length_min, policy.length_max
        ));
        Ok(())
    }
}

/// What an upgrade works with besides its inputs.
pub struct Parts<'a> {
    /// Runner executable in the incoming build; absent for service-only fixtures.
    pub runner: Option<&'a Path>,
    /// The broker and the service, in start order.
    pub units: &'a [Unit],
    /// The engine that runs the compose services.
    pub engine: &'a mut dyn Engine,
    /// The new build's templates.
    pub render: &'a dyn Render,
    /// The client releases to place inside the transaction; absent for
    /// service-only fixtures.
    pub clients: Option<&'a install::layout::ClientSet>,
}

/// Read deployment configuration and verify the installed issuer authority.
pub(super) fn verified_config(layout: &Layout) -> IdentityResult<DeploymentConfig> {
    let config = DeploymentConfig::load_install(&layout.deployment_config())?;
    install::directory_key::verify(&config)?;
    Ok(config)
}

fn refuse(
    kind: ErrorKind,
    action: &'static str,
    resource: &str,
    detail: impl Into<String>,
) -> IdentityError {
    IdentityError::new(kind, action, resource, detail)
}

fn io(action: &'static str, path: &Path, error: &std::io::Error) -> IdentityError {
    refuse(
        ErrorKind::PrivateFileIo,
        action,
        "binaries",
        error.to_string(),
    )
    .at(path)
}

/// Starts `unit` from `bin/` and waits for it ready. A process already
/// running is left alone unless `replace` asks for it to be started afresh.
/// Its log is read forward from the offset it opened at, kept beside its
/// pid file; one started before offsets were kept is read from the start,
/// once. `true` when it was started.
///
/// # Errors
/// Returns named install-state, service-control or upgrade-phase failures.
pub fn launch(layout: &Layout, unit: &Unit, replace: bool) -> IdentityResult<bool> {
    let before = std::fs::metadata(&unit.log).map_or(0, |meta| meta.len());
    let program = layout.binary(unit.binary);
    let started = services::start_detached(&program, &unit.args, &unit.log, &unit.pid, replace)?;
    let offset_file = unit.pid.with_extension("offset");
    let from = if started {
        private_files::write(&offset_file, before.to_string().as_bytes())?;
        before
    } else {
        std::fs::read_to_string(&offset_file)
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0)
    };
    let mut cursor = LogCursor::at(&unit.log, from);
    // A service that refused to start for a lost store stays up to say so
    // and never says it is listening: its refusal ends the wait as a
    // failure, in its own words (DIRECTORY-107 R2).
    let mut refusal = LogCursor::at(&unit.log, from);
    let mut refused = false;
    let ready = &unit.ready;
    log_wait::wait_until(unit.binary, &unit.log, &unit.pid, &mut || {
        refused = refusal.says(REFUSED_TO_START);
        refused
            || cursor.says(&ready.says)
                && ready
                    .answers
                    .is_none_or(|(port, path)| services::answers(port, path))
    })?;
    if refused {
        return Err(refuse(
            ErrorKind::StoreLost,
            "start",
            unit.binary,
            refused_line(&unit.log, from),
        )
        .at(&unit.log));
    }
    Ok(started)
}

/// The words a service refused to start with, in its log.
const REFUSED_TO_START: &str = "refused_to_start: Lys refused to start";

/// The line of `log` after `from` that says the start was refused.
fn refused_line(log: &Path, from: u64) -> String {
    let text = std::fs::read(log).unwrap_or_default();
    let after = usize::try_from(from)
        .ok()
        .and_then(|from| text.get(from..))
        .unwrap_or_default();
    String::from_utf8_lossy(after)
        .lines()
        .find(|line| line.contains(REFUSED_TO_START))
        .map_or_else(
            || format!("it refused to start; its words are in {}", log.display()),
            |line| line.trim().to_owned(),
        )
}

/// Runs `program --version`, reads `name VERSION (COMMIT)` and answers what
/// the parentheses hold: a commit, perhaps with `; dirty`, or the words of a
/// build with no commit.
///
/// # Errors
/// Returns named install-state, service-control or upgrade-phase failures.
pub fn version(program: &Path, name: &str) -> IdentityResult<String> {
    let unreadable = |detail: String| {
        refuse(ErrorKind::VersionUnreadable, "read version", name, detail).at(program)
    };
    let environment = login::login()?;
    let output = Command::new(program)
        .env_clear()
        .envs(environment.variables())
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| unreadable(format!("--version could not run: {error}")))?;
    if !output.status.success() {
        return Err(unreadable(format!("--version exited {}", output.status)));
    }
    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    line.strip_prefix(&format!("{name} "))
        .and_then(|rest| rest.split_once(" ("))
        .and_then(|(_, inside)| inside.strip_suffix(')'))
        .filter(|commit| !commit.is_empty())
        .map(str::to_string)
        .ok_or_else(|| unreadable(format!("`{line}` is not `{name} VERSION (COMMIT)`")))
}

/// Refuses by name unless `layout` holds an install.
fn require_install(layout: &Layout) -> IdentityResult<()> {
    let needed = [layout.deployment_config(), layout.service_config()];
    match needed.into_iter().find(|path| !path.is_file()) {
        None => Ok(()),
        Some(missing) => Err(refuse(
            ErrorKind::NotInstalled,
            "find install",
            "install",
            "no install here to upgrade; run `lys identity install` from the new build first",
        )
        .at(&missing)),
    }
}

/// Each new binary's commit, refusing a missing binary or an unreadable
/// version by name.
fn incoming(from: &Path, names: &[&'static str]) -> IdentityResult<BTreeMap<String, String>> {
    let mut build = BTreeMap::new();
    for &name in names {
        let new = from.join(name);
        if !new.is_file() {
            return Err(refuse(
                ErrorKind::BinaryMissing,
                "find new binary",
                name,
                format!("{} holds no {name}", from.display()),
            )
            .at(&new));
        }
        build.insert(name.to_string(), version(&new, name)?);
    }
    Ok(build)
}

/// Upgrades the install under `layout` to the binaries in `from` (and the
/// screens in `package`), and returns the build now running. `say` hears
/// every line the upgrade prints.
///
/// # Errors
/// Returns named install-state, service-control or upgrade-phase failures.
pub fn upgrade(
    layout: &Layout,
    from: &Path,
    package: Option<&Path>,
    parts: &mut Parts<'_>,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    super::accept_loss::check_before(layout, "upgrade")?;
    if parts.runner.is_some() {
        companion::check(layout, from)?;
    }
    let mut restart = recovered(layout, parts, say)?;
    let mut all_units = parts.units.to_vec();
    if let Some(restart) = &restart {
        all_units.push(restart.unit.clone());
    }
    let names: Vec<&'static str> = all_units.iter().map(|unit| unit.binary).collect();
    require_install(layout)?;
    let to = incoming(from, &names)?;
    if let Some(program) = parts.runner
        && program != from.join("lys")
    {
        return Err(refuse(
            ErrorKind::VersionUnreadable,
            "find new binary",
            "lys",
            "the runner must come from the incoming build",
        )
        .at(program));
    }
    if let Some(package) = package {
        let (manifest, _) = surface::verify(package)?;
        say(&format!("screens: new {}", manifest.commit));
    }
    let installed = adopt::installed(layout, &all_units, say)?;
    for (name, new) in &to {
        let old = installed.get(name).map_or(adopt::UNSTAMPED, String::as_str);
        say(&format!("{name}: installed {old}, new {new}"));
    }
    let screens_existed = layout.surface_dir().join("index.html").is_file();
    let files = parts
        .render
        .render(layout, &to, screens_existed || package.is_some())?;
    let mut intent = planned_intent(installed, to, package.is_some(), screens_existed, &files);
    if let Some(restart) = &restart {
        restart.retain()?;
    }
    if let Err(error) = intent
        .write(layout)
        .and_then(|()| place_clients(layout, parts.clients, &mut intent, say))
    {
        if let Some(restart) = &restart {
            restart.cancel()?;
        }
        Intent::clear(layout)?;
        return Err(error);
    }
    let plan = swap::Plan {
        from,
        names: &names,
        package,
        files: &files,
    };
    let result = swap::forward(
        layout,
        &plan,
        &mut intent,
        parts.units,
        parts.engine,
        restart.as_mut(),
        say,
    );
    let Err(failure) = result else {
        return finish_upgrade(layout, &names, parts.engine, say);
    };
    say(&format!("upgrade failed: {failure}"));
    say("putting the previous build back");
    if let Err(again) = swap::back(
        layout,
        &intent,
        parts.units,
        parts.engine,
        restart.as_mut(),
        say,
    ) {
        return Err(refuse(
            ErrorKind::UpgradeFailed,
            "upgrade",
            "install",
            format!(
                "{failure}; the previous build did not come back either: {again}; \
                 install/upgrade.json keeps the unfinished upgrade for the next run"
            ),
        ));
    }
    record_build(layout, &names, say)?;
    Intent::clear(layout)?;
    Err(refuse(
        ErrorKind::UpgradeFailed,
        "upgrade",
        "install",
        format!("{failure}; the previous build is back and running"),
    ))
}

#[cfg(test)]
#[path = "upgrade/scratch_tests.rs"]
mod scratch;

#[cfg(test)]
#[path = "upgrade/stores_receipt_tests.rs"]
mod stores_receipt_tests;

#[cfg(test)]
#[path = "upgrade_tests.rs"]
mod tests;

fn planned_intent(
    installed: BTreeMap<String, String>,
    to: BTreeMap<String, String>,
    screens: bool,
    screens_existed: bool,
    files: &[render::RenderedFile],
) -> Intent {
    Intent {
        from: installed,
        to,
        screens,
        screens_existed,
        files: files
            .iter()
            .map(|file| intent::Kept {
                name: file.name.to_string(),
                target: file.target.clone(),
                existed: file.target.is_file(),
                private: file.private,
            })
            .collect(),
        compose_changed: files.iter().any(|file| file.compose && !file.in_place()),
        steps: Vec::new(),
        organisation_copy: None,
        clients_placed: install::layout::PlacedClients::default(),
    }
}

/// Ends an upgrade stopped part-way, with the runner read first when the
/// incoming build has one, and answers that runner for the swap.
fn recovered(
    layout: &Layout,
    parts: &mut Parts<'_>,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<Option<runner::Restart>> {
    let mut restart = parts
        .runner
        .map(|_| runner::Restart::prepare(layout))
        .transpose()?;
    if let Some(restart) = &mut restart {
        swap::recover_with_runner(layout, parts.units, parts.engine, restart, say)?;
    } else {
        swap::recover(layout, parts.units, parts.engine, say)?;
    }
    Ok(restart)
}

/// Places the client releases as a step of the transaction, after the
/// intent record is written and before anything is stopped. What the
/// attempt made is on the record once the step is; a refusal here removes
/// it, so nothing placed outlives the attempt (DIRECTORY-115 R1).
fn place_clients(
    layout: &Layout,
    clients: Option<&install::layout::ClientSet>,
    intent: &mut Intent,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let Some(clients) = clients else {
        return Ok(());
    };
    let placed = clients.place(layout, &mut intent.clients_placed, say);
    if let Err(error) = placed.and_then(|()| intent.done(layout, Step::ClientsPlaced)) {
        intent.clients_placed.remove(say)?;
        return Err(error);
    }
    Ok(())
}

fn finish_upgrade(
    layout: &Layout,
    names: &[&str],
    engine: &mut dyn Engine,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    let record = record_build(layout, names, say)?;
    Intent::clear(layout)?;
    // After the record is written and the fence cleared: a refusal here never leaves an unfinished upgrade.
    engine.settle(layout, say).map_err(|error| {
        refuse(
            ErrorKind::UpgradeFailed,
            "settle",
            "password_policy",
            format!(
                "the new build is running, but the sign-in service did not take Lys's password policy: {error}"
            ),
        )
    })?;
    Ok(record)
}
