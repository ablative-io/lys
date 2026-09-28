//! An install made before builds were named, moved to one that is, and the
//! processes an install runs, stopped and restarted so what runs is always
//! what `bin/` holds.
//!
//! An install made before this card has no `install/build.json`, may have no
//! `bin/` (its broker and service ran from beside the `lys` that installed
//! them) and may run processes started without an exit lock. Its binaries
//! are read as they are: one that cannot answer `--version` is recorded as
//! built before build stamps. A binary missing from `bin/` is adopted from
//! the file its running process was started from, placed and read back like
//! any other. A process with an exit lock is stopped through it; one without
//! is stopped through its pid, its exit waited on as an event (a pidfd on
//! Linux, `caffeinate -w` over kqueue on macOS, `pwait` on the BSDs), and
//! each stop says which way it went.
//!
//! Invariants: install never runs a process from a binary it replaced: a
//! process whose binary it placed is stopped and started again, and install
//! refuses by name to place a build other than the one already placed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::exit_wait;
use super::super::install::layout::Layout;
use super::super::install::services;
use super::{BuildRecord, Unit, launch, record_build, swap, version};

/// What a placed binary that cannot answer `--version` is recorded as.
pub const UNSTAMPED: &str = "built before build stamps";

fn refuse(kind: ErrorKind, what: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(kind, "adopt install", what, detail)
}

/// The commit `program`'s `--version` names, or [`UNSTAMPED`] when it
/// cannot answer.
pub fn commit_or_unstamped(program: &Path, name: &str) -> IdentityResult<String> {
    match version(program, name) {
        Ok(commit) => Ok(commit),
        Err(error) if error.kind() == ErrorKind::VersionUnreadable => Ok(UNSTAMPED.to_string()),
        Err(error) => Err(error),
    }
}

/// The pid a pid file names.
fn read_pid(pid_file: &Path) -> IdentityResult<u32> {
    std::fs::read_to_string(pid_file)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .ok_or_else(|| {
            refuse(
                ErrorKind::Unready,
                "service",
                "the pid file names no process",
            )
            .at(pid_file)
        })
}

/// The file the process `pid` was started from.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn executable(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).ok()
}

/// The file the process `pid` was started from.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn executable(pid: u32) -> Option<PathBuf> {
    let output = Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    path.is_absolute().then_some(path)
}

/// Places into `bin/` the file `unit`'s running process was started from,
/// and returns that file: the copy is read back equal to it byte for byte.
fn adopt_running(
    layout: &Layout,
    unit: &Unit,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<PathBuf> {
    let placed = layout.binary(unit.binary);
    let missing = |detail: String| refuse(ErrorKind::NotInstalled, unit.binary, detail).at(&placed);
    if !services::alive(&unit.pid) {
        return Err(missing(format!(
            "bin/ holds no {} and none is running to adopt; run `lys identity install` from the new build",
            unit.binary
        )));
    }
    let pid = read_pid(&unit.pid)?;
    let program = executable(pid)
        .filter(|path| path.is_file() && path.file_name().is_some_and(|name| name == unit.binary))
        .ok_or_else(|| {
            missing(format!(
                "the running {} (pid {pid}) was started from a file that cannot be found",
                unit.binary
            ))
        })?;
    swap::ensure_bin(layout)?;
    swap::place_binary(&program, &layout.bin_dir(), unit.binary)?;
    say(&format!(
        "{} adopted into bin/ from {}",
        unit.binary,
        program.display()
    ));
    Ok(program)
}

/// Each installed binary's commit. An install that records its build
/// refuses a binary whose `--version` cannot be read; one made before
/// builds were recorded names it [`UNSTAMPED`]. A binary `bin/` lacks is
/// adopted from its running process first, and its build is read from the
/// file that process runs, which the placed copy equals.
pub fn installed(
    layout: &Layout,
    units: &[Unit],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BTreeMap<String, String>> {
    let recorded = layout.build_record().is_file();
    let mut build = BTreeMap::new();
    for unit in units {
        let placed = layout.binary(unit.binary);
        let program = if placed.is_file() {
            placed
        } else {
            adopt_running(layout, unit, say)?
        };
        let commit = if recorded {
            version(&program, unit.binary)?
        } else {
            commit_or_unstamped(&program, unit.binary)?
        };
        build.insert(unit.binary.to_string(), commit);
    }
    Ok(build)
}

/// Sends the process `pid` the stop signal.
fn signal(pid: u32, pid_file: &Path) -> IdentityResult<()> {
    Command::new("kill")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(drop)
        .map_err(|error| refuse(ErrorKind::Unready, "service", error.to_string()).at(pid_file))
}

/// Stops the process `pid` and waits for its exit on the pidfd the kernel
/// makes readable when it exits, opened before the signal is sent.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn stop_through_pid(pid: u32, pid_file: &Path) -> IdentityResult<()> {
    use mio::unix::SourceFd;
    use mio::{Events, Interest, Poll, Token};
    use rustix::process::{Pid, PidfdFlags, pidfd_open};
    use std::os::fd::AsRawFd;

    let failed = |detail: String| refuse(ErrorKind::Unready, "service", detail).at(pid_file);
    let target = i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or_else(|| failed(format!("{pid} is not a process id")))?;
    let exit = pidfd_open(target, PidfdFlags::empty())
        .map_err(|error| failed(format!("watching process {pid}: {error}")))?;
    signal(pid, pid_file)?;
    let watching = |error: std::io::Error| failed(format!("waiting on process {pid}: {error}"));
    let mut poll = Poll::new().map_err(watching)?;
    let fd = exit.as_raw_fd();
    poll.registry()
        .register(&mut SourceFd(&fd), Token(0), Interest::READABLE)
        .map_err(watching)?;
    let mut events = Events::with_capacity(1);
    loop {
        if let Err(error) = poll.poll(&mut events, None) {
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(watching(error));
        }
        if !events.is_empty() {
            return Ok(());
        }
    }
}

/// The tool that blocks until a process exits, and its arguments before the
/// pid.
#[cfg(target_os = "macos")]
const EXIT_WAITER: (&str, &[&str]) = ("caffeinate", &["-w"]);

/// The tool that blocks until a process exits, and its arguments before the
/// pid.
#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_os = "macos"))
))]
const EXIT_WAITER: (&str, &[&str]) = ("pwait", &[]);

/// Stops the process `pid` and waits for its exit through the platform's
/// exit waiter, which blocks on the kernel's exit event and returns at once
/// for a process already gone. Its success is the exit: nothing asks
/// after it, since an exiting process can still be listed for a moment
/// after its exit event.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
fn stop_through_pid(pid: u32, pid_file: &Path) -> IdentityResult<()> {
    let (waiter, args) = EXIT_WAITER;
    signal(pid, pid_file)?;
    let status = Command::new(waiter)
        .args(args)
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| {
            refuse(
                ErrorKind::Unready,
                "service",
                format!("{waiter} could not run: {error}"),
            )
            .at(pid_file)
        })?;
    if !status.success() {
        return Err(refuse(
            ErrorKind::Unready,
            "service",
            format!("{waiter} could not wait on process {pid}: it returned {status}"),
        )
        .at(pid_file));
    }
    Ok(())
}

/// Without Unix there is no process to stop through its pid.
#[cfg(not(unix))]
fn stop_through_pid(pid: u32, pid_file: &Path) -> IdentityResult<()> {
    Err(refuse(
        ErrorKind::Unready,
        "service",
        format!("stopping process {pid} needs a Unix host"),
    )
    .at(pid_file))
}

/// Stops `unit`'s process, through its exit lock when it has one and
/// through its pid when it has none, and says which.
pub fn stop(unit: &Unit, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
    if exit_wait::lock_path(&unit.pid).exists() {
        if services::stop(&unit.pid)? {
            say(&format!("{} stopped through its exit lock", unit.binary));
        }
        return Ok(());
    }
    if !services::alive(&unit.pid) {
        return Ok(());
    }
    let pid = read_pid(&unit.pid)?;
    stop_through_pid(pid, &unit.pid)?;
    say(&format!(
        "{} stopped through its pid {pid}: it was started without an exit lock",
        unit.binary
    ));
    Ok(())
}

/// Refuses, before anything is stopped, when `bin/` holds a build other
/// than the one `source` names for each binary: install never restarts a
/// placed build on a new configuration, and an upgrade is the way to a new
/// build.
pub fn check_placed_build(
    layout: &Layout,
    names: &[&str],
    source: &dyn Fn(&str) -> IdentityResult<PathBuf>,
) -> IdentityResult<()> {
    for &name in names {
        let placed = layout.binary(name);
        if !placed.is_file() {
            continue;
        }
        let new = source(name)?;
        let placed_commit = commit_or_unstamped(&placed, name)?;
        let new_commit = version(&new, name)?;
        if placed_commit != new_commit {
            let from = new.parent().unwrap_or(&new).display().to_string();
            return Err(IdentityError::new(
                ErrorKind::InstallBuildDiffers,
                "install",
                name,
                format!(
                    "the placed {name} is {placed_commit} and this install's is {new_commit}; \
                     install never restarts a placed build on a new configuration: run \
                     `lys identity upgrade --from {from}` to move to the new build"
                ),
            )
            .at(&placed));
        }
    }
    Ok(())
}

/// How a start is reported: a process that was replaced was restarted.
fn started_word(started: bool, replace: bool) -> &'static str {
    match (started, replace) {
        (true, true) => "restarted",
        (true, false) => "started",
        (false, _) => "already running",
    }
}

/// Places each binary `bin/` lacks from `source`, then starts the units in
/// order, each waited on for ready: one whose binary was just placed, or
/// every one when `changed` says the configuration changed, is stopped and
/// started again. Records and returns the build that then runs.
pub fn settle(
    layout: &Layout,
    units: &[Unit],
    source: &dyn Fn(&str) -> IdentityResult<PathBuf>,
    changed: bool,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    swap::ensure_bin(layout)?;
    let mut placed = Vec::new();
    for unit in units {
        if layout.binary(unit.binary).is_file() {
            continue;
        }
        swap::place_binary(&source(unit.binary)?, &layout.bin_dir(), unit.binary)?;
        say(&format!(
            "{} placed in {}",
            unit.binary,
            layout.bin_dir().display()
        ));
        placed.push(unit.binary);
    }
    for unit in units {
        let replace = changed || placed.contains(&unit.binary);
        if replace {
            stop(unit, say)?;
        }
        let started = launch(layout, unit, false)?;
        let at = match unit.ready.answers {
            Some((port, _)) => format!(" on 127.0.0.1:{port}"),
            None => String::new(),
        };
        say(&format!(
            "{} {}{at}",
            unit.binary,
            started_word(started, replace)
        ));
    }
    let names: Vec<&str> = units.iter().map(|unit| unit.binary).collect();
    record_build(layout, &names, say)
}

#[cfg(test)]
#[path = "adopt_tests.rs"]
mod tests;
