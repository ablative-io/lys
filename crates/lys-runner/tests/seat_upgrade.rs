//! AGENTS-004 R3 (amendment 1): an upgrade of the owner binary moves no
//! harness. A seat started before the upgrade keeps the owner process it
//! started with, and says its build; a seat started after is served by the
//! new binary.

use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::protocol::hex;
use lys_runner::seat_owner::recovery;
use lys_runner::seat_owner::sessions::OwnedSeat;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture() -> Result<PathBuf, Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    let debug = exe
        .parent()
        .and_then(Path::parent)
        .ok_or("the test binary has no target directory")?;
    let fixture = debug.join("examples").join("fake_seat_owner");
    if !fixture.exists() {
        return Err(format!("{} was not built beside the tests", fixture.display()).into());
    }
    Ok(fixture)
}

fn kill(pid: u32) -> Result<(), Box<dyn Error>> {
    let pid = rustix::process::Pid::from_raw(i32::try_from(pid)?).ok_or("pid 0")?;
    rustix::process::kill_process(pid, rustix::process::Signal::KILL)?;
    Ok(())
}

fn alive(pid: u32) -> bool {
    rustix::process::Pid::from_raw(i32::try_from(pid).unwrap_or(0))
        .is_some_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}

fn runner_with_owner(
    state: &Path,
    session: &str,
    key: &Arc<Ed25519Identity>,
) -> Result<(OwnedSeat, Child), Box<dyn Error>> {
    let mut child = Command::new(fixture()?)
        .arg("--as-runner")
        .arg(state)
        .arg(session)
        .arg(hex(&key.public_key_bytes()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("no pipe")?;
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    if line.is_empty() {
        let status = child.wait()?;
        return Err(format!("the runner fixture exited {status} before its seat").into());
    }
    Ok((serde_json::from_str(line.trim_end())?, child))
}

#[test]
fn seat_started_before_an_upgrade_keeps_its_owner() -> TestResult {
    let dir = tempfile::tempdir()?;
    let state = dir.path().join("runner");
    fs::create_dir_all(&state)?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    // The installed binary, before the upgrade.
    fs::write(state.join("owner-build"), "0.9.0-before\n")?;
    let (before, mut old_runner) = runner_with_owner(&state, "before", &key)?;
    assert_eq!(before.endpoint.build, "0.9.0-before");
    let owner_before = before.endpoint.owner;
    assert!(alive(owner_before.pid));

    // The upgrade: the runner is replaced; the owner binary on disk is the
    // new one. Nothing is sent to the old owner.
    kill(old_runner.id())?;
    old_runner.wait()?;
    fs::write(state.join("owner-build"), "1.0.0-after\n")?;
    let (after, mut new_runner) = runner_with_owner(&state, "after", &key)?;
    assert_eq!(after.endpoint.build, "1.0.0-after");
    assert_ne!(after.endpoint.owner.pid, owner_before.pid);

    // The first seat is still served by its original owner process, at the
    // build it started with; the second by the new binary.
    assert!(
        alive(owner_before.pid),
        "the old owner outlived the upgrade"
    );
    let (found, counts) = recovery::recover(&state)?;
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(recovery::Found::live), "{found:?}");
    let by_session = |session: &str| {
        found
            .iter()
            .find(|owner| owner.seat.binding.session == session)
            .map(|owner| &owner.seat.endpoint)
    };
    let kept = by_session("before").ok_or("the first seat was not found")?;
    assert_eq!(kept.owner, owner_before);
    assert_eq!(kept.build, "0.9.0-before");
    let new = by_session("after").ok_or("the second seat was not found")?;
    assert_eq!(new.build, "1.0.0-after");
    assert!(counts.record_visits <= 2, "{counts:?}");

    kill(owner_before.pid)?;
    kill(after.endpoint.owner.pid)?;
    kill(new_runner.id())?;
    new_runner.wait()?;
    Ok(())
}
