//! The processes the install starts and waits on: the compose services,
//! the secrets broker and the directory service.
//!
//! Every wait is on a condition, never a clock: the install asks again
//! until the thing answers, saying what is still missing as it goes.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::super::config::DeploymentConfig;
use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::health;
use super::super::loopback_http::{self, Authority, Request};
use super::super::private_files;
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

/// `docker compose up -d --wait` for the deployment under `layout`.
pub fn compose_up(layout: &Layout, config: &DeploymentConfig) -> IdentityResult<()> {
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
    args.extend(["up", "-d", "--wait"].map(str::to_string));
    run_to_end(Path::new("docker"), &args, "compose").map(|_| ())
}

/// Asks the three services until every one is ready, saying which are not
/// each time round.
pub fn wait_ready(config: &DeploymentConfig, say: &mut dyn FnMut(&str)) {
    let mut round = 0u32;
    loop {
        let checks = health::check_all(config);
        let unready: Vec<String> = checks
            .iter()
            .filter_map(|check| {
                check
                    .failure
                    .as_ref()
                    .map(|(name, _)| format!("{} {name}", check.service))
            })
            .collect();
        if unready.is_empty() {
            return;
        }
        if round % 5 == 0 {
            say(&format!("waiting for {}", unready.join(", ")));
        }
        round += 1;
        std::thread::sleep(Duration::from_secs(1));
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
/// but not yet been reaped (a zombie) counts as gone.
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
    !state.is_empty() && !state.starts_with('Z')
}

/// Stops the process a pid file names and waits until it is gone. `false`
/// when none was alive.
pub fn stop(pid_file: &Path) -> IdentityResult<bool> {
    if !alive(pid_file) {
        return Ok(false);
    }
    let pid = std::fs::read_to_string(pid_file)
        .map_err(|error| refuse("stop", "service", error.to_string()).at(pid_file))?;
    Command::new("kill")
        .arg(pid.trim())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| refuse("stop", "service", error.to_string()))?;
    while alive(pid_file) {
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(true)
}

/// Starts `program` detached, its output appended to `log`, its pid kept in
/// `pid_file`. A process the pid file already names alive is left alone,
/// unless `replace` asks for it to be stopped and started afresh.
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
    let open = |path: &Path| -> IdentityResult<File> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| refuse("open log", "service", error.to_string()).at(path))
    };
    let out = open(log)?;
    let err = open(log)?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err);
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
    loopback_http::exchange(&authority, Duration::from_secs(2), &request).is_ok()
}

/// Asks the loopback service on `port` until it answers `path`, or the
/// process behind the pid file is gone, in which case the log is named.
pub fn wait_answering(port: u16, path: &str, pid_file: &Path, log: &Path) -> IdentityResult<()> {
    loop {
        if answers(port, path) {
            return Ok(());
        }
        if !alive(pid_file) {
            return Err(refuse(
                "wait for service",
                &format!("127.0.0.1:{port}"),
                format!("the process exited; its output is in {}", log.display()),
            ));
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
