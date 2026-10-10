//! DIRECTORY-092 R2: the estate folder a killed test leaves is swept, by
//! name, when the next estate is made. Each test sweeps a parent folder of
//! its own, so no other test's sweep can reach its fixtures.

use std::fs::File;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Child, Command};

use rustix::fs::{FlockOperation, flock};

use super::TestResult;
use super::estate::{Estate, OWNER, PREFIX};

/// A child the test started, killed and reaped however the test ends.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => eprintln!("fixture state unreadable: {error}"),
        }
        if let Err(error) = self.0.kill() {
            eprintln!("fixture termination failed: {error}");
        }
        if let Err(error) = self.0.wait() {
            eprintln!("fixture reaping failed: {error}");
        }
    }
}

/// A process id that no longer names a process: a child already reaped.
fn gone_pid() -> TestResult<u32> {
    let mut child = Command::new("/usr/bin/true").spawn()?;
    let pid = child.id();
    child.wait()?;
    Ok(pid)
}

/// An estate folder under `parent` recording `owner`, with its run folder.
fn left_estate(parent: &Path, name: &str, owner: Option<u32>) -> TestResult<std::path::PathBuf> {
    let folder = parent.join(format!("{PREFIX}{name}"));
    std::fs::create_dir_all(folder.join("run"))?;
    if let Some(owner) = owner {
        std::fs::write(folder.join(OWNER), format!("{owner}\nsweep-{name}\n"))?;
    }
    Ok(folder)
}

/// A model proxy stand-in in `folder`: it holds run/proxy.exit for its life,
/// as `lys proxy serve` does, and its pid is in run/proxy.pid.
fn held_proxy(folder: &Path) -> TestResult<Reaped> {
    let run = folder.join("run");
    let lock = File::create(run.join("proxy.exit"))?;
    flock(&lock, FlockOperation::LockExclusive)?;
    let child = Command::new("/bin/sleep")
        .arg("600")
        .stdin(lock)
        .process_group(0)
        .spawn()?;
    std::fs::write(run.join("proxy.pid"), child.id().to_string())?;
    Ok(Reaped(child))
}

fn lock_free(path: &Path) -> TestResult<bool> {
    let file = File::open(path)?;
    Ok(flock(&file, FlockOperation::NonBlockingLockExclusive).is_ok())
}

#[test]
fn the_estate_a_killed_test_left_is_swept_and_named() -> TestResult {
    let parent = tempfile::tempdir()?;
    let folder = left_estate(parent.path(), "killed", Some(gone_pid()?))?;
    let mut proxy = held_proxy(&folder)?;
    let estate = Estate::new_in(parent.path(), "sweep-next".to_owned(), 1, 2, 3)?;
    let status = proxy.0.wait()?;
    assert_eq!(
        status.signal(),
        Some(9),
        "the left proxy was killed: {status:?}"
    );
    assert!(!folder.exists(), "the left folder was removed");
    let named: Vec<&String> = estate
        .swept
        .iter()
        .filter(|line| line.contains(&folder.display().to_string()))
        .collect();
    assert_eq!(named.len(), 1, "{:?}", estate.swept);
    assert!(
        named[0].contains("removed") && named[0].contains("proxy"),
        "{}",
        named[0]
    );
    estate.close()
}

#[test]
fn an_estate_whose_owner_still_runs_is_left_alone() -> TestResult {
    let parent = tempfile::tempdir()?;
    let folder = left_estate(parent.path(), "running", Some(std::process::id()))?;
    let proxy = held_proxy(&folder)?;
    let estate = Estate::new_in(parent.path(), "sweep-next".to_owned(), 1, 2, 3)?;
    assert!(folder.exists(), "a running owner's folder was removed");
    assert!(
        !lock_free(&folder.join("run/proxy.exit"))?,
        "a running owner's proxy was stopped"
    );
    assert!(
        estate
            .swept
            .iter()
            .all(|line| !line.contains(&folder.display().to_string())),
        "{:?}",
        estate.swept
    );
    drop(proxy);
    estate.close()
}

#[test]
fn an_estate_with_no_owner_record_is_left_and_named() -> TestResult {
    let parent = tempfile::tempdir()?;
    let folder = left_estate(parent.path(), "unowned", None)?;
    let estate = Estate::new_in(parent.path(), "sweep-next".to_owned(), 1, 2, 3)?;
    assert!(folder.exists(), "a folder with no owner was removed");
    let named: Vec<&String> = estate
        .swept
        .iter()
        .filter(|line| line.contains(&folder.display().to_string()))
        .collect();
    assert_eq!(named.len(), 1, "{:?}", estate.swept);
    assert!(named[0].contains("records no owner"), "{}", named[0]);
    estate.close()
}

#[test]
fn a_service_whose_exit_lock_is_free_is_never_signalled() -> TestResult {
    let parent = tempfile::tempdir()?;
    let folder = left_estate(parent.path(), "reused", Some(gone_pid()?))?;
    // The proxy ended; its pid now names an unrelated process.
    let mut unrelated = Reaped(
        Command::new("/bin/sleep")
            .arg("600")
            .process_group(0)
            .spawn()?,
    );
    let run = folder.join("run");
    File::create(run.join("proxy.exit"))?;
    std::fs::write(run.join("proxy.pid"), unrelated.0.id().to_string())?;
    let estate = Estate::new_in(parent.path(), "sweep-next".to_owned(), 1, 2, 3)?;
    assert!(!folder.exists(), "the left folder was removed");
    assert!(
        unrelated.0.try_wait()?.is_none(),
        "a process under a free exit lock was signalled"
    );
    estate.close()
}
