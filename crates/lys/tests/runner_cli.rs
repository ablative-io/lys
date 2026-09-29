#![cfg(test)]
//! DIRECTORY-050 R1: `lys runner serve` killed outright, with no chance to
//! record anything, and started again reports every session it held as
//! `ended_by_runner_restart`, at the instant the restart found it gone and
//! with no exit status; a process that ignored the hang-up and outlived it
//! is ended by the restart before it is reported gone. Asked to stop by
//! SIGTERM, the runner ends every session and waits for each exit, so the
//! next runner reports each one exited. Each runner is known to be
//! answering by the line it prints once it listens, and a process known to
//! be gone by the end of a pipe it held, never by a clock.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::protocol::{SessionView, hex};
use lys_runner::{Act, Answer, Client, EndedHow, Launch};

type TestResult = Result<(), Box<dyn Error>>;

/// Start `lys runner serve` in `dir`, answering once it says it listens.
fn serve(dir: &Path) -> Result<Child, Box<dyn Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(["runner", "serve", "--socket"])
        .arg(dir.join("runner.sock"))
        .arg("--state")
        .arg(dir.join("state"))
        .arg("--server-key")
        .arg(dir.join("server.pub"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let out = child.stdout.take().ok_or("the runner has no output")?;
    let mut line = String::new();
    BufReader::new(out).read_line(&mut line)?;
    assert!(line.starts_with("listening "), "{line}");
    Ok(child)
}

/// A server key in `dir`, its public half where `serve` reads it, and a
/// client of the runner there.
fn client(dir: &Path) -> Result<Client, Box<dyn Error>> {
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.join("server.key"))?);
    std::fs::write(dir.join("server.pub"), hex(&key.public_key_bytes()))?;
    Ok(Client::new(dir.join("runner.sock"), key))
}

/// Start session `id` running `script`, answering once it prints `ready`.
fn started(client: &Client, id: &str, script: &str) -> TestResult {
    let launch = Launch {
        session: id.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), format!("echo re''ady; {script}")],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        columns: 80,
        rows: 24,
        rotation: None,
    };
    assert!(matches!(
        client.ask(&Act::Start { launch })?,
        Answer::Started { .. }
    ));
    let waited = client.ask(&Act::Wait {
        session: id.to_owned(),
        cursor: Some(0),
        pattern: "ready".to_owned(),
        regex: false,
    })?;
    assert!(matches!(waited, Answer::Matched { .. }), "{waited:?}");
    Ok(())
}

fn held(client: &Client) -> Result<Vec<SessionView>, Box<dyn Error>> {
    let Answer::Status { status } = client.ask(&Act::Status { session: None })? else {
        return Err("status answered no status".into());
    };
    Ok(status.sessions)
}

fn find<'a>(sessions: &'a [SessionView], id: &str) -> Result<&'a SessionView, Box<dyn Error>> {
    sessions
        .iter()
        .find(|session| session.session == id)
        .ok_or_else(|| format!("{id} is not held").into())
}

#[test]
fn a_killed_runner_restarted_reports_its_sessions_ended_by_the_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let client = client(dir.path())?;
    let fifo = dir.path().join("held.fifo");
    assert!(Command::new("mkfifo").arg(&fifo).status()?.success());

    let mut first = serve(dir.path())?;
    started(&client, "held", "exec cat")?;
    // It ignores the hang-up its terminal closing sends, and holds the
    // pipe open for writing for as long as it lives.
    started(
        &client,
        "stubborn",
        &format!("trap '' HUP; exec cat <> {}", fifo.display()),
    )?;
    let mut pipe = std::fs::File::open(&fifo)?;
    first.kill()?;
    first.wait()?;

    let mut second = serve(dir.path())?;
    let sessions = held(&client)?;
    for id in ["held", "stubborn"] {
        let session = find(&sessions, id)?;
        let ended = session
            .ended
            .as_ref()
            .ok_or("a lost session reads running")?;
        assert_eq!(ended.how, EndedHow::EndedByRunnerRestart, "{id}");
        assert_eq!(ended.status, None, "no exit status is invented: {id}");
        assert!(ended.at >= session.started_at, "{id}");
    }
    let stubborn = find(&sessions, "stubborn")?.ended.clone();
    assert_eq!(
        stubborn.and_then(|ended| ended.signal).as_deref(),
        Some("SIGKILL"),
        "the process that outlived the lost runner is ended by the restart"
    );
    let mut rest = Vec::new();
    pipe.read_to_end(&mut rest)?;
    assert!(
        rest.is_empty(),
        "the pipe ends only once its writer is gone"
    );
    second.kill()?;
    second.wait()?;
    Ok(())
}

#[test]
fn a_runner_asked_to_stop_ends_every_session_and_the_next_reports_each_exit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let client = client(dir.path())?;
    let mut first = serve(dir.path())?;
    started(&client, "one", "exec cat")?;
    started(&client, "two", "exec cat")?;
    let asked = Command::new("kill")
        .args(["-TERM", &first.id().to_string()])
        .status()?;
    assert!(asked.success());
    let stopped = first.wait()?;
    assert!(stopped.success(), "the runner stops cleanly: {stopped:?}");
    assert!(
        !dir.path().join("runner.sock").exists(),
        "the stopped runner removed its socket"
    );

    let mut second = serve(dir.path())?;
    let sessions = held(&client)?;
    let mut exited = 0;
    for id in ["one", "two"] {
        let ended = find(&sessions, id)?
            .ended
            .clone()
            .ok_or("a session reads running")?;
        assert_eq!(ended.how, EndedHow::Exited, "its exit was seen: {id}");
        exited += 1;
    }
    assert_eq!(exited, 2);
    second.kill()?;
    second.wait()?;
    Ok(())
}
