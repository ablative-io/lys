use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command, Stdio};

use rustix::fs::{FlockOperation, flock};

use super::{Estate, TestResult};

struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        if let Err(error) = self.0.kill() {
            eprintln!("fixture termination failed: {error}");
        }
        if let Err(error) = self.0.wait() {
            eprintln!("fixture reaping failed: {error}");
        }
    }
}

#[test]
fn an_install_failure_leaves_no_owned_service_running() -> TestResult {
    let root = tempfile::tempdir()?;
    let run = root.path().join("run");
    std::fs::create_dir(&run)?;
    let blocked = root.path().join("blocked");
    assert!(Command::new("mkfifo").arg(&blocked).status()?.success());
    let mut children = Vec::new();
    for name in ["runner", "identity", "secrets"] {
        let lock = std::fs::File::create(run.join(format!("{name}.exit")))?;
        flock(&lock, FlockOperation::LockExclusive)?;
        let child = Command::new("/bin/sh")
            .args([
                "-c",
                r#"trap '' TERM; printf 'ready\n'; read line < "$1""#,
                "lys-fixture",
            ])
            .arg(&blocked)
            .stdin(lock)
            .stdout(Stdio::piped())
            .spawn()?;
        let mut held = Reaped(child);
        std::fs::write(run.join(format!("{name}.pid")), held.0.id().to_string())?;
        let stdout = held.0.stdout.take().ok_or("fixture stdout missing")?;
        let mut ready = String::new();
        BufReader::new(stdout).read_line(&mut ready)?;
        assert_eq!(ready, "ready\n");
        children.push(held);
    }
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))?;
    let failed = std::panic::catch_unwind(|| {
        let estate = Estate {
            root,
            project: "cleanup-fixture".to_owned(),
            rauthy_port: 1,
        };
        let guard = estate;
        std::panic::panic_any(guard.root.path().to_path_buf());
    });
    assert!(failed.is_err());
    let mut running = 0;
    for held in &mut children {
        if held.0.try_wait()?.is_none() {
            running += 1;
        }
    }
    assert_eq!(
        running, 0,
        "a failed install left owned Lys services running"
    );
    Ok(())
}

#[test]
fn an_install_can_select_its_own_loopback_ports() -> TestResult {
    let help = Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(["identity", "install", "--help"])
        .output()?;
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout)?;
    assert!(text.contains("--service-port"), "{text}");
    assert!(text.contains("--broker-port"), "{text}");
    Ok(())
}
