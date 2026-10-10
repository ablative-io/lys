use std::fs::File;
use std::path::Path;
use std::process::Command;

use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;

use super::identity_support::fixtures::{TestResult, succeeded};
use super::identity_support::processes::output;

/// The prefix every estate's folder carries, so the folder a killed test
/// leaves can be found and swept by the next estate.
pub(super) const PREFIX: &str = "lys-estate-";

/// The file in an estate's folder that records the test process owning it
/// (first line) and its compose project (second line).
pub(super) const OWNER: &str = ".estate-owner";

/// Every detached service an install starts under its root, by pid file:
/// the runner, the directory service, the secrets broker and the model
/// proxy.
pub(super) const SERVICES: [&str; 4] = ["runner", "identity", "secrets", "proxy"];

/// One installed estate owns every detached service until teardown finishes.
pub(super) struct Estate {
    pub root: tempfile::TempDir,
    pub project: String,
    pub rauthy_port: u16,
    pub service_port: u16,
    pub broker_port: u16,
    pub cleaned: bool,
    /// The lines the sweep before it said, one per folder it removed or left.
    pub swept: Vec<String>,
}

fn lock(file: &File, operation: FlockOperation) -> Result<(), Errno> {
    loop {
        match flock(file, operation) {
            Err(Errno::INTR) => {}
            outcome => return outcome,
        }
    }
}

/// Stop the service `pid_file` names if its exit lock is held: `true` when
/// it was running and was killed, `false` when there was nothing to stop.
/// A free exit lock means the service has ended, so its pid, which another
/// process may now have, is never signalled.
fn stop_service(pid_file: &Path) -> TestResult<bool> {
    let pid = match std::fs::read_to_string(pid_file) {
        Ok(pid) => pid.trim().parse::<u32>()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let exit = File::open(pid_file.with_extension("exit"))?;
    let mut stopped = false;
    match lock(&exit, FlockOperation::NonBlockingLockShared) {
        Ok(()) => {}
        Err(Errno::AGAIN) => {
            stopped = true;
            let killed = output(
                Command::new("kill").args(["-KILL", &pid.to_string()]),
                &format!("kill -KILL {pid}"),
            )?;
            succeeded(&killed, &format!("stop owned service {pid}"))?;
            lock(&exit, FlockOperation::LockShared)?;
        }
        Err(error) => return Err(error.into()),
    }
    lock(&exit, FlockOperation::Unlock)?;
    Ok(stopped)
}

/// An estate's own compose project, as its teardown and its logs name it.
fn compose(root: &Path, project: &str) -> Command {
    let mut compose = Command::new("docker");
    compose
        .arg("compose")
        .arg("-f")
        .arg(root.join("deploy/compose.yaml"))
        .arg("--env-file")
        .arg(root.join("state/compose.env"))
        .args(["-p", project, "--profile", "bundled-db"]);
    compose
}

/// Stop every service under `root` and take its compose project down,
/// collecting each failure by name.
fn stop_all(root: &Path, project: &str, failures: &mut Vec<String>) -> Vec<&'static str> {
    let mut stopped = Vec::new();
    for name in SERVICES {
        let path = root.join("run").join(format!("{name}.pid"));
        match stop_service(&path) {
            Ok(true) => stopped.push(name),
            Ok(false) => {}
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }
    if root.join("state/compose.env").exists() {
        match output(
            compose(root, project).args(["down", "-v", "--remove-orphans"]),
            "remove install containers",
        ) {
            Ok(removed) => {
                if let Err(error) = succeeded(&removed, "remove install containers") {
                    failures.push(error.to_string());
                }
            }
            Err(error) => failures.push(error.to_string()),
        }
    }
    stopped
}

/// Whether process `pid` still exists. A process this test may not signal
/// still exists.
fn exists(pid: i32) -> TestResult<bool> {
    let Some(pid) = rustix::process::Pid::from_raw(pid) else {
        return Err(format!("estate owner {pid} is not a process id").into());
    };
    match rustix::process::test_kill_process(pid) {
        Ok(()) | Err(Errno::PERM) => Ok(true),
        Err(Errno::SRCH) => Ok(false),
        Err(error) => Err(format!("estate owner {pid}: {error}").into()),
    }
}

/// Sweep every estate folder under `parent` whose owner no longer runs: the
/// folder a test that was killed before its estate dropped leaves. Its
/// services are stopped through their exit locks only, its compose project
/// is taken down, and it is removed. A folder whose owner runs is left; one
/// with no readable owner record is left and named. One line per folder.
pub(super) fn sweep(parent: &Path) -> TestResult<Vec<String>> {
    let mut said = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(PREFIX) || !entry.file_type()?.is_dir() {
            continue;
        }
        let folder = entry.path();
        let record = match std::fs::read_to_string(folder.join(OWNER)) {
            Ok(record) => record,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                said.push(format!(
                    "estate sweep left {}: it records no owner",
                    folder.display()
                ));
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let mut lines = record.lines();
        let (Some(owner), Some(project)) = (
            lines.next().and_then(|pid| pid.trim().parse::<i32>().ok()),
            lines
                .next()
                .map(str::trim)
                .filter(|project| !project.is_empty()),
        ) else {
            said.push(format!(
                "estate sweep left {}: its owner record is unreadable",
                folder.display()
            ));
            continue;
        };
        if exists(owner)? {
            continue;
        }
        let mut failures = Vec::new();
        let stopped = stop_all(&folder, project, &mut failures);
        if !failures.is_empty() {
            return Err(format!(
                "estate sweep of {} failed: {}",
                folder.display(),
                failures.join("; ")
            )
            .into());
        }
        match std::fs::remove_dir_all(&folder) {
            Ok(()) => said.push(format!(
                "estate sweep removed {} (owner {owner} gone); stopped: {}",
                folder.display(),
                if stopped.is_empty() {
                    "nothing".to_owned()
                } else {
                    stopped.join(", ")
                }
            )),
            // A sweep in another test process removed it first.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => said.push(format!(
                "estate sweep found {} already removed by another sweep",
                folder.display()
            )),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(said)
}

impl Estate {
    /// An estate in the system's temporary folder, made after sweeping what
    /// killed tests left there.
    pub fn new(
        project: String,
        rauthy_port: u16,
        service_port: u16,
        broker_port: u16,
    ) -> TestResult<Self> {
        Self::new_in(
            &std::env::temp_dir(),
            project,
            rauthy_port,
            service_port,
            broker_port,
        )
    }

    /// An estate under `parent`: the sweep first, each line it says
    /// printed, then a prefixed folder recording this test process and the
    /// compose project.
    pub fn new_in(
        parent: &Path,
        project: String,
        rauthy_port: u16,
        service_port: u16,
        broker_port: u16,
    ) -> TestResult<Self> {
        let swept = sweep(parent)?;
        for line in &swept {
            eprintln!("{line}");
        }
        let root = tempfile::Builder::new().prefix(PREFIX).tempdir_in(parent)?;
        std::fs::write(
            root.path().join(OWNER),
            format!("{}\n{project}\n", std::process::id()),
        )?;
        Ok(Self {
            root,
            project,
            rauthy_port,
            service_port,
            broker_port,
            cleaned: false,
            swept,
        })
    }

    /// The install's own compose project, as its teardown and its logs name it.
    fn compose(&self) -> Command {
        compose(self.root.path(), &self.project)
    }

    /// Put every log the estate's services and containers wrote into the
    /// test's output, before teardown removes them with the scratch root, so
    /// a red can be read after it.
    fn testify(&self) {
        let logs = self.root.path().join("logs");
        match std::fs::read_dir(&logs) {
            Ok(entries) => {
                let mut paths: Vec<_> = entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .collect();
                paths.sort();
                for path in paths {
                    match std::fs::read(&path) {
                        Ok(bytes) => eprintln!(
                            "===== {} =====\n{}",
                            path.display(),
                            String::from_utf8_lossy(&bytes)
                        ),
                        Err(error) => eprintln!("===== {} unread: {error}", path.display()),
                    }
                }
            }
            Err(error) => eprintln!("===== {} unread: {error}", logs.display()),
        }
        if !self.root.path().join("state/compose.env").exists() {
            eprintln!("===== no compose project was written; no container logs");
            return;
        }
        match output(
            self.compose().args(["logs", "--no-color", "--timestamps"]),
            "docker compose logs",
        ) {
            Ok(logs) => eprintln!(
                "===== docker compose logs ({}) =====\n{}{}",
                logs.status,
                String::from_utf8_lossy(&logs.stdout),
                String::from_utf8_lossy(&logs.stderr)
            ),
            Err(error) => eprintln!("===== docker compose logs could not run: {error}"),
        }
    }

    fn cleanup(&self) -> TestResult {
        let mut failures = Vec::new();
        stop_all(self.root.path(), &self.project, &mut failures);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("install_teardown_failed: {}", failures.join("; ")).into())
        }
    }

    pub fn close(mut self) -> TestResult {
        self.cleanup()?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for Estate {
    fn drop(&mut self) {
        if !self.cleaned {
            // Not closed: the test failed an assertion or returned an error.
            self.testify();
            if let Err(error) = self.cleanup() {
                eprintln!("install failure cleanup: {error}");
            }
        }
    }
}
