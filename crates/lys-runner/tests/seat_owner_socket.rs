#![cfg(test)]
//! Owners migrate old plans and sockets before binding the short endpoint.

use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use lys_core::Ed25519Identity;
use lys_runner::harness_control::ManagedLaunch;
use lys_runner::seat_owner::protocol::{LOG, OwnerBinding, owner_socket, parse_ready_line};
use lys_runner::seat_owner::spawn::OwnerPlan;
use lys_runner::{Options, Runner};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn plan(session: &str) -> Result<OwnerPlan, Box<dyn Error>> {
    let conversation = "019c6e27-e55b-73d1-87d8-4e01f1f75043";
    let binding = OwnerBinding {
        seat: "socket-seat".to_owned(),
        session: session.to_owned(),
        conversation: conversation.to_owned(),
        generation: 1,
    };
    let managed: ManagedLaunch = serde_json::from_value(json!({
        "launch": {"session": session, "program": "/bin/cat", "arguments": [],
            "directory": "/", "environment": {}, "columns": 80, "rows": 24},
        "transport": {"mode": "claude"}, "conversation": conversation,
        "requires_controls": false
    }))?;
    Ok(OwnerPlan::new(binding, managed, None))
}

struct OwnerProcess(Child);

impl Drop for OwnerProcess {
    /// Ends the owner with its process group and reaps it (AGENTS-018 R1):
    /// the owner leads a group of its own once it has detached.
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => eprintln!("owner_cleanup_status: {error}"),
        }
        if let Some(group) = i32::try_from(self.0.id())
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        {
            match rustix::process::kill_process_group(group, rustix::process::Signal::KILL) {
                Ok(()) | Err(rustix::io::Errno::SRCH) => {}
                Err(error) => eprintln!("owner_cleanup_group: {error}"),
            }
        }
        if let Err(error) = self.0.kill() {
            eprintln!("owner_cleanup_signal: {error}");
        }
        if let Err(error) = self.0.wait() {
            eprintln!("owner_cleanup_reap: {error}");
        }
    }
}

fn owner_listens(dir: &Path, socket: &Path, key: &Path) -> TestResult {
    let current = std::env::current_exe()?;
    let target = current
        .parent()
        .and_then(Path::parent)
        .ok_or("test has no target directory")?;
    let fixture = target.join("examples/fake_seat_owner");
    if !fixture.try_exists()? {
        return Err(format!("owner fixture is absent: {}", fixture.display()).into());
    }
    let mut owner = OwnerProcess(
        Command::new(fixture)
            .args(["runner", "seat-owner", "--dir"])
            .arg(dir)
            .arg("--server-key")
            .arg(key)
            .args(["--scrollback", "1024"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let pipe = owner.0.stdout.take().ok_or("owner has no ready pipe")?;
    let mut line = String::new();
    BufReader::new(pipe).read_line(&mut line)?;
    let (_, announced, leader, _) = parse_ready_line(&line)?;
    assert_eq!(announced, socket);
    assert_eq!(leader.pid, owner.0.id());
    UnixStream::connect(socket)?;
    Ok(())
}

#[test]
fn seat_owner_old_plan_binds_short_socket_under_long_root() -> TestResult {
    let temporary = tempfile::tempdir_in("/tmp")?;
    let root = temporary
        .path()
        .join("identity-root-with-enough-padding-for-the-legacy-limit");
    let session = "op-0123456789abcdef0123456789abcdef";
    let dir = root.join("data/runner/owners").join(session);
    fs::create_dir_all(&dir)?;
    plan(session)?.write(&dir)?;
    let legacy = dir.join("owner.sock");
    assert!(legacy.as_os_str().len() >= 104);
    let loaded = OwnerPlan::read(&dir)?;
    let socket = loaded.socket_in(&dir);
    let expected = owner_socket(&root.join("run/owners"), session);
    assert_eq!(socket, expected);
    assert!(socket.as_os_str().len() < 104);
    let key = Ed25519Identity::load_or_generate(&temporary.path().join("server.key"))?;
    let runner = Runner::open(&Options {
        socket: socket.clone(),
        state: dir.join("runner"),
        server_key: key.public_key_bytes(),
        scrollback: 1024,
    })?;
    let serving = runner.spawn();
    let connected = UnixStream::connect(&socket);
    serving.stop()?;
    connected?;
    let public_key = temporary.path().join("server.pub");
    fs::write(
        &public_key,
        lys_runner::protocol::hex(&key.public_key_bytes()),
    )?;
    owner_listens(&dir, &socket, &public_key)?;
    Ok(())
}

#[test]
fn seat_owner_stale_legacy_socket_is_removed_and_logged_once() -> TestResult {
    let temporary = tempfile::tempdir_in("/tmp")?;
    let root = temporary.path();
    let session = "old";
    let dir = root.join("data/runner/owners").join(session);
    fs::create_dir_all(&dir)?;
    plan(session)?.write(&dir)?;
    let legacy = dir.join("owner.sock");
    drop(UnixListener::bind(&legacy)?);
    OwnerPlan::read(&dir)?;
    assert!(
        !legacy.exists(),
        "stale legacy socket remained: {}",
        legacy.display()
    );
    let log = fs::read_to_string(dir.join(LOG))?;
    assert_eq!(
        log.matches(&legacy.display().to_string()).count(),
        1,
        "{log}"
    );
    OwnerPlan::read(&dir)?;
    assert_eq!(fs::read_to_string(dir.join(LOG))?, log);
    Ok(())
}

#[test]
fn seat_owner_listening_legacy_socket_refuses_a_second_owner() -> TestResult {
    let temporary = tempfile::tempdir_in("/tmp")?;
    let dir = temporary.path().join("data/runner/owners/old");
    fs::create_dir_all(&dir)?;
    plan("old")?.write(&dir)?;
    let legacy = dir.join("owner.sock");
    let listener = UnixListener::bind(&legacy)?;
    let error = OwnerPlan::read(&dir).expect_err("a listening legacy owner was accepted");
    assert_eq!(error.name(), "seat_owner_legacy_listening");
    assert!(error.to_string().contains(&legacy.display().to_string()));
    assert!(legacy.exists());
    drop(listener);
    Ok(())
}
