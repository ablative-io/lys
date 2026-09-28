//! The processes the install starts and waits on: the compose services,
//! the secrets broker and the directory service.
//!
//! Every wait is on an event, never a clock, and never a question asked
//! again on a schedule. A service is checked once when the wait begins and
//! once on each change to its output: a detached service's log file through
//! the platform's file-change notice, as [`super::log_wait`] describes, the
//! compose services through their followed output. The wait ends ready when
//! the check passes and is refused by name, with where the output is, when
//! the process exits first. A stop waits on the exit itself, as
//! [`super::exit_wait`] describes.

use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::super::config::DeploymentConfig;
use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::health;
use super::super::loopback_http::{self, Authority, Request};
use super::super::private_files;
use super::exit_wait::{self, ExitWatch};
use super::layout::Layout;

const COMPOSE_ENV: &str = "compose.env";

fn refuse(action: &'static str, resource: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(ErrorKind::Unready, action, resource, detail)
}

/// Runs `program` with `args` to completion, refusing by name when it is
/// missing or exits unsuccessfully.
pub fn run_to_end(program: &Path, args: &[String], resource: &str) -> IdentityResult<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            refuse(
                "start",
                resource,
                format!("{} could not start: {error}", program.display()),
            )
        })?;
    if !output.status.success() {
        return Err(refuse(
            "run",
            resource,
            format!(
                "{} exited with {}: {}",
                program.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Refuses, naming the fix, unless Docker with its compose plugin is
/// installed and its engine is running. `docker` names the program to ask.
pub fn require_docker(docker: &Path) -> IdentityResult<()> {
    let version = Command::new(docker)
        .args(["compose", "version"])
        .stdin(Stdio::null())
        .output();
    match version {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(refuse(
                "start",
                "docker",
                "Docker is not installed; install Docker Desktop, open it once, then run this install again",
            ));
        }
        Err(error) => {
            return Err(refuse(
                "start",
                "docker",
                format!("{} could not start: {error}", docker.display()),
            ));
        }
        Ok(output) if !output.status.success() => {
            return Err(refuse(
                "start",
                "docker",
                "Docker is installed without its compose plugin; install Docker Desktop, which carries it, then run this install again",
            ));
        }
        Ok(_) => {}
    }
    let engine = Command::new(docker)
        .arg("info")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            refuse(
                "start",
                "docker",
                format!("{} could not start: {error}", docker.display()),
            )
        })?;
    if engine.status.success() {
        return Ok(());
    }
    Err(refuse(
        "start",
        "docker",
        "Docker is installed but its engine is not running; open Docker Desktop, wait for it to say it is running, then run this install again",
    ))
}

/// The `docker compose` arguments naming the deployment under `layout`.
fn compose_args(layout: &Layout, config: &DeploymentConfig) -> Vec<String> {
    let mut args = vec![
        "compose".to_string(),
        "-f".to_string(),
        layout
            .deploy_dir()
            .join("compose.yaml")
            .display()
            .to_string(),
        "--env-file".to_string(),
        config.state_dir().join(COMPOSE_ENV).display().to_string(),
    ];
    if config.database.bundled {
        args.push("--profile".to_string());
        args.push("bundled-db".to_string());
    }
    args
}

/// `docker compose up -d --wait` for the deployment under `layout`.
pub fn compose_up(layout: &Layout, config: &DeploymentConfig) -> IdentityResult<()> {
    let mut args = compose_args(layout, config);
    args.extend(["up", "-d", "--wait"].map(str::to_string));
    run_to_end(Path::new("docker"), &args, "compose").map(|_| ())
}

/// The declared services of `config` that are not ready now.
fn unready_services(config: &DeploymentConfig) -> Vec<String> {
    health::check_all(config)
        .iter()
        .filter(|check| check.failure.is_some())
        .map(|check| check.service.to_string())
        .collect()
}

/// Waits until the three compose services are ready. They are checked once
/// now; when any is not, the compose output is followed and they are
/// checked once on each piece of it that arrives. `say` hears one line
/// naming what the wait is for, and one as each service becomes ready.
pub fn wait_ready(
    layout: &Layout,
    config: &DeploymentConfig,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let waiting = unready_services(config);
    if waiting.is_empty() {
        return Ok(());
    }
    say(&format!("waiting for {}", waiting.join(", ")));
    let mut args = compose_args(layout, config);
    args.extend(["logs", "--follow", "--no-color"].map(str::to_string));
    let mut follower = Command::new("docker")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            refuse(
                "start",
                "compose",
                format!("docker compose logs could not start: {error}"),
            )
        })?;
    let outcome = match follower.stdout.take() {
        Some(mut output) => {
            ready_on_output(&mut output, waiting, &mut || unready_services(config), say)
        }
        None => Err(refuse(
            "follow",
            "compose",
            "docker compose logs gave no output",
        )),
    };
    follower
        .kill()
        .and_then(|()| follower.wait().map(drop))
        .map_err(|error| {
            refuse(
                "stop",
                "compose",
                format!("docker compose logs did not stop: {error}"),
            )
        })?;
    outcome
}

/// Checks `unready` once on each piece of `output` that arrives, until
/// nothing in `waiting` is unready; `say` hears each service as it becomes
/// ready. The output ending first is refused, naming what was not ready.
pub fn ready_on_output(
    output: &mut dyn Read,
    mut waiting: Vec<String>,
    unready: &mut dyn FnMut() -> Vec<String>,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let mut piece = [0_u8; 8192];
    loop {
        match output.read(&mut piece) {
            Ok(0) => {
                return Err(refuse(
                    "wait for service",
                    "compose",
                    format!(
                        "the compose output ended with {} not ready; see `docker compose logs`",
                        waiting.join(", ")
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(refuse(
                    "follow",
                    "compose",
                    format!("reading the compose output: {error}"),
                ));
            }
        }
        let still = unready();
        for service in waiting.iter().filter(|service| !still.contains(service)) {
            say(&format!("{service} ready"));
        }
        if still.is_empty() {
            return Ok(());
        }
        waiting = still;
    }
}

/// The sibling binary `name` beside the running `lys`.
pub fn sibling(name: &str) -> IdentityResult<PathBuf> {
    let own = std::env::current_exe().map_err(|error| {
        refuse(
            "locate binaries",
            name,
            format!("own path unknown: {error}"),
        )
    })?;
    let path = own.with_file_name(name);
    if !path.is_file() {
        return Err(refuse(
            "locate binaries",
            name,
            format!("{} is not installed beside lys", path.display()),
        ));
    }
    Ok(path)
}

/// Makes the secrets broker's store when there is none yet.
pub fn broker_init(layout: &Layout) -> IdentityResult<bool> {
    if layout.broker_root().join("audit").exists() || layout.broker_root().join("store").exists() {
        return Ok(false);
    }
    let program = sibling("lys-secrets")?;
    run_to_end(
        &program,
        &[
            "init".to_string(),
            "--root".to_string(),
            layout.broker_root().display().to_string(),
            "--keys".to_string(),
            layout.broker_keys().display().to_string(),
        ],
        "secrets broker",
    )?;
    Ok(true)
}

/// Whether the process a pid file names is alive. A process that has exited
/// but not yet been reaped (a zombie) counts as gone, and so does one whose
/// exit lock is free: the kernel releases it as the exit begins.
pub fn alive(pid_file: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(pid_file) else {
        return false;
    };
    let Ok(pid) = text.trim().parse::<u32>() else {
        return false;
    };
    let Ok(output) = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .stderr(Stdio::null())
        .output()
    else {
        return false;
    };
    let state = String::from_utf8_lossy(&output.stdout);
    let state = state.trim();
    if state.is_empty() || state.starts_with('Z') {
        return false;
    }
    match ExitWatch::open(pid_file) {
        Ok(watch) => !matches!(watch.exited(), Ok(true)),
        Err(_) => true,
    }
}

/// Stops the process a pid file names and waits for its exit. `false`
/// when none was alive.
pub fn stop(pid_file: &Path) -> IdentityResult<bool> {
    stop_with(pid_file, &mut alive)
}

/// [`stop`], asking `alive` whether the process lives. It is asked once,
/// before the process is told to stop; the exit is then waited on as an
/// event and nothing is asked again.
pub fn stop_with(pid_file: &Path, alive: &mut dyn FnMut(&Path) -> bool) -> IdentityResult<bool> {
    if !alive(pid_file) {
        return Ok(false);
    }
    let exit = ExitWatch::open(pid_file)?;
    let pid = std::fs::read_to_string(pid_file)
        .map_err(|error| refuse("stop", "service", error.to_string()).at(pid_file))?;
    Command::new("kill")
        .arg(pid.trim())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| refuse("stop", "service", error.to_string()))?;
    exit.wait()?;
    Ok(true)
}

/// Starts `program` detached, its output appended to `log`, its pid kept in
/// `pid_file` and its exit lock held through its standard input. A process
/// the pid file already names alive is left alone, unless `replace` asks
/// for it to be stopped and started afresh.
pub fn start_detached(
    program: &Path,
    args: &[String],
    log: &Path,
    pid_file: &Path,
    replace: bool,
) -> IdentityResult<bool> {
    if alive(pid_file) {
        if !replace {
            return Ok(false);
        }
        stop(pid_file)?;
    }
    let open = |path: &Path| -> IdentityResult<std::fs::File> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| refuse("open log", "service", error.to_string()).at(path))
    };
    let out = open(log)?;
    let err = open(log)?;
    let lock = exit_wait::hold(pid_file)?;
    let mut command = Command::new(program);
    command.args(args).stdin(lock).stdout(out).stderr(err);
    detach(&mut command);
    let child = command.spawn().map_err(|error| {
        refuse(
            "start",
            "service",
            format!("{} could not start: {error}", program.display()),
        )
    })?;
    private_files::write(pid_file, child.id().to_string().as_bytes())?;
    Ok(true)
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn detach(_command: &mut Command) {}

/// Whether the loopback service on `port` answers `path` with a status.
pub fn answers(port: u16, path: &str) -> bool {
    let authority = Authority {
        host: "127.0.0.1".to_string(),
        port,
    };
    let request = Request {
        method: "GET",
        path,
        headers: &[],
        body: &[],
    };
    loopback_http::exchange(&authority, &request).is_ok()
}
