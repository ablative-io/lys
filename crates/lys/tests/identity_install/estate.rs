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
    fn cleanup(&self) -> TestResult {
        let mut failures = Vec::new();
        for name in ["runner.pid", "identity.pid", "secrets.pid"] {
            let path = self.root.path().join("run").join(name);
            if let Err(error) = stop_service(&path) {
                failures.push(format!("{}: {error}", path.display()));
            }
        }
        if self.root.path().join("state/compose.env").exists() {
            let down = Command::new("docker")
                .arg("compose")
                .arg("-f")
                .arg(self.root.path().join("deploy/compose.yaml"))
                .arg("--env-file")
                .arg(self.root.path().join("state/compose.env"))
                .args(["-p", &self.project, "--profile", "bundled-db"])
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
            if let Err(error) = self.cleanup() {
                eprintln!("install failure cleanup: {error}");
            }
        }
    }
}
