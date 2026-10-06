use std::fs::File;
use std::path::Path;
use std::process::Command;

use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;

use super::identity_support::fixtures::{TestResult, succeeded};

/// One installed estate owns every detached service until teardown finishes.
pub(super) struct Estate {
    pub root: tempfile::TempDir,
    pub project: String,
    pub rauthy_port: u16,
    pub service_port: u16,
    pub broker_port: u16,
    pub cleaned: bool,
}

fn lock(file: &File, operation: FlockOperation) -> Result<(), Errno> {
    loop {
        match flock(file, operation) {
            Err(Errno::INTR) => {}
            outcome => return outcome,
        }
    }
}

fn stop_service(pid_file: &Path) -> TestResult {
    let pid = match std::fs::read_to_string(pid_file) {
        Ok(pid) => pid.trim().parse::<u32>()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let exit = File::open(pid_file.with_extension("exit"))?;
    match lock(&exit, FlockOperation::NonBlockingLockShared) {
        Ok(()) => {}
        Err(Errno::AGAIN) => {
            let stopped = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output()?;
            succeeded(&stopped, &format!("stop owned service {pid}"))?;
            lock(&exit, FlockOperation::LockShared)?;
        }
        Err(error) => return Err(error.into()),
    }
    lock(&exit, FlockOperation::Unlock)?;
    Ok(())
}

impl Estate {
    /// The install's own compose project, as its teardown and its logs name it.
    fn compose(&self) -> Command {
        let mut compose = Command::new("docker");
        compose
            .arg("compose")
            .arg("-f")
            .arg(self.root.path().join("deploy/compose.yaml"))
            .arg("--env-file")
            .arg(self.root.path().join("state/compose.env"))
            .args(["-p", &self.project, "--profile", "bundled-db"]);
        compose
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
        match self
            .compose()
            .args(["logs", "--no-color", "--timestamps"])
            .output()
        {
            Ok(output) => eprintln!(
                "===== docker compose logs ({}) =====\n{}{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) => eprintln!("===== docker compose logs could not run: {error}"),
        }
    }

    fn cleanup(&self) -> TestResult {
        let mut failures = Vec::new();
        for name in ["runner.pid", "identity.pid", "secrets.pid"] {
            let path = self.root.path().join("run").join(name);
            if let Err(error) = stop_service(&path) {
                failures.push(format!("{}: {error}", path.display()));
            }
        }
        if self.root.path().join("state/compose.env").exists() {
            let down = self
                .compose()
                .args(["down", "-v", "--remove-orphans"])
                .output();
            match down {
                Ok(output) => {
                    if let Err(error) = succeeded(&output, "remove install containers") {
                        failures.push(error.to_string());
                    }
                }
                Err(error) => failures.push(format!("remove install containers: {error}")),
            }
        }
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
