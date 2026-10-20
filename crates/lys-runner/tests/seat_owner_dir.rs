#![cfg(test)]
//! An owner whose folder is gone ends itself (AGENTS-018 R2): the owner
//! under test is the `fake_seat_owner` example (the real owner steps with
//! no harness); its folder is removed under it, and its exit is the event
//! waited on, never a timer.

use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};

use lys_core::Ed25519Identity;
use lys_runner::harness_control::ManagedLaunch;
use lys_runner::seat_owner::protocol::{OwnerBinding, parse_ready_line};
use lys_runner::seat_owner::spawn::OwnerPlan;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn plan(session: &str) -> Result<OwnerPlan, Box<dyn Error>> {
    let conversation = "019c6e27-e55b-73d1-87d8-4e01f1f75044";
    let binding = OwnerBinding {
        seat: "dir-seat".to_owned(),
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

fn fixture() -> Result<std::path::PathBuf, Box<dyn Error>> {
    let current = std::env::current_exe()?;
    let target = current
        .parent()
        .and_then(Path::parent)
        .ok_or("test has no target directory")?;
    let fixture = target.join("examples/fake_seat_owner");
    if !fixture.try_exists()? {
        return Err(format!("owner fixture is absent: {}", fixture.display()).into());
    }
    Ok(fixture)
}

#[test]
fn an_owner_whose_folder_is_removed_ends_itself_with_one_line() -> TestResult {
    let temporary = tempfile::tempdir_in("/tmp")?;
    let root = temporary.path().join("root");
    let session = "op-0123456789abcdef0123456789abcd01";
    let dir = root.join("data/runner/owners").join(session);
    fs::create_dir_all(&dir)?;
    plan(session)?.write(&dir)?;
    let key = Ed25519Identity::load_or_generate(&temporary.path().join("server.key"))?;
    let public_key = temporary.path().join("server.pub");
    fs::write(
        &public_key,
        lys_runner::protocol::hex(&key.public_key_bytes()),
    )?;
    let mut owner = Command::new(fixture()?)
        .args(["runner", "seat-owner", "--dir"])
        .arg(&dir)
        .arg("--server-key")
        .arg(&public_key)
        .args(["--scrollback", "1024"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let pipe = owner.stdout.take().ok_or("owner has no ready pipe")?;
    let mut line = String::new();
    BufReader::new(pipe).read_line(&mut line)?;
    let (_, _, leader, _) = parse_ready_line(&line)?;
    assert_eq!(leader.pid, owner.id());

    // The folder goes; the owner's exit is the event waited on.
    fs::remove_dir_all(&dir)?;
    let status = owner.wait()?;
    let mut words = String::new();
    owner
        .stderr
        .take()
        .ok_or("owner has no stderr")?
        .read_to_string(&mut words)?;
    assert!(status.success(), "the owner exited {status}: {words}");
    assert_eq!(
        words.matches("seat_owner_dir_gone:").count(),
        1,
        "one line, naming the folder: {words}"
    );
    assert!(words.contains(&dir.display().to_string()), "{words}");
    assert!(words.contains("removed"), "{words}");
    Ok(())
}
