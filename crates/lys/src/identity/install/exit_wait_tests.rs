#![cfg(test)]

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::super::services;
use super::{ExitWatch, lock_path};
use crate::identity::error::ErrorKind;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// Starts a scratch service that blocks opening a fifo nobody writes, so it
/// lives until it is told to stop and waits on no clock.
fn scratch(dir: &Path) -> TestResult<PathBuf> {
    let never = dir.join("never");
    services::run_to_end(Path::new("mkfifo"), &[never.display().to_string()], "fifo")?;
    let pid = dir.join("scratch.pid");
    let args = [
        "-c".to_string(),
        r#"read line < "$1""#.to_string(),
        "scratch".to_string(),
        never.display().to_string(),
    ];
    let log = dir.join("scratch.log");
    assert!(services::start_detached(
        Path::new("/bin/sh"),
        &args,
        &log,
        &pid,
        false
    )?);
    Ok(pid)
}

#[test]
fn a_stop_returns_on_the_exit_event_and_asks_nothing_after_the_kill() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let pid = scratch(dir.path())?;
    assert!(
        !ExitWatch::open(&pid)?.exited()?,
        "a running service holds its lock"
    );
    let mut asked = 0;
    let stopped = services::stop_with(&pid, &mut |pid_file| {
        asked += 1;
        services::alive(pid_file)
    })?;
    assert!(stopped);
    assert_eq!(
        asked, 1,
        "alive is asked once, before the kill, and never polled"
    );
    assert!(ExitWatch::open(&pid)?.exited()?, "the exit has happened");
    assert!(!services::alive(&pid));
    Ok(())
}

#[test]
fn a_pid_file_naming_a_process_already_gone_answers_at_once() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let mut gone = Command::new("/usr/bin/true").spawn()?;
    gone.wait()?;
    let pid = dir.path().join("gone.pid");
    std::fs::write(&pid, gone.id().to_string())?;
    let mut asked = 0;
    let stopped = services::stop_with(&pid, &mut |pid_file| {
        asked += 1;
        services::alive(pid_file)
    })?;
    assert!(!stopped);
    assert_eq!(asked, 1);
    assert!(!lock_path(&pid).exists(), "nothing was watched");
    Ok(())
}

#[test]
fn a_live_process_without_an_exit_lock_is_refused_naming_its_pid() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let mut held = Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;
    let pid = dir.path().join("held.pid");
    std::fs::write(&pid, held.id().to_string())?;
    let outcome = services::stop(&pid);
    held.kill()?;
    held.wait()?;
    let refused = outcome.err().ok_or("stopped without an exit lock")?;
    assert_eq!(refused.kind(), ErrorKind::Unready);
    let named = format!("process {} has no exit lock", held.id());
    assert!(refused.to_string().contains(&named), "{refused}");
    Ok(())
}
