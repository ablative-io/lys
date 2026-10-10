#![cfg(test)]
//! AGENTS-002 R2: the runner says, from its own knowledge, which sessions
//! are alive. Two sessions started and one ended: one alive, one ended. A
//! process killed outside Lys is no longer alive once the runner has seen
//! its exit. A session the runner does not hold is refused by name. Every
//! wait below ends on the runner's answer, never a clock.

use std::collections::BTreeMap;
use std::error::Error;
use std::process::Command;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::liveness::LivenessView;
use lys_runner::{Act, Answer, Client, Launch, Options, Runner, RunnerError, Serving};

type TestResult = Result<(), Box<dyn Error>>;

struct Held {
    dir: tempfile::TempDir,
    client: Client,
    serving: Option<Serving>,
}

impl Held {
    fn start() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &dir.path().join("server.key"),
        )?);
        let options = Options {
            socket: dir.path().join("runner.sock"),
            state: dir.path().join("state"),
            server_key: key.public_key_bytes(),
            scrollback: 1 << 16,
        };
        let serving = Runner::open(&options)?.spawn();
        let client = Client::new(options.socket, key);
        Ok(Self {
            dir,
            client,
            serving: Some(serving),
        })
    }

    fn stop(&mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

fn shell(session: &str, directory: &str) -> Launch {
    Launch {
        session: session.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "cat".to_owned()],
        directory: directory.to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

fn started(client: &Client, launch: Launch) -> Result<u32, Box<dyn Error>> {
    match client.ask(&Act::Start {
        lys_mcp: None,
        proxy: None,
        launch: Box::new(launch),
    })? {
        Answer::Started { pid, .. } => Ok(pid),
        other => Err(format!("start answered {other:?}").into()),
    }
}

fn liveness(client: &Client, session: Option<&str>) -> Result<Vec<LivenessView>, Box<dyn Error>> {
    match client.ask(&Act::Liveness {
        session: session.map(str::to_owned),
    })? {
        Answer::Liveness { sessions } => Ok(sessions),
        other => Err(format!("liveness answered {other:?}").into()),
    }
}

fn named<'a>(views: &'a [LivenessView], session: &str) -> Result<&'a LivenessView, String> {
    views
        .iter()
        .find(|view| view.session == session)
        .ok_or_else(|| format!("liveness names no session {session}"))
}

/// Follow session `id`'s output until the runner records its end.
fn until_ended(client: &Client, id: &str) -> TestResult {
    let mut cursor = None;
    loop {
        match client.ask(&Act::ReadBytes {
            session: id.to_owned(),
            cursor,
            follow: true,
        })? {
            Answer::Bytes { output } if output.ended.is_some() => return Ok(()),
            Answer::Bytes { output } => cursor = Some(output.cursor),
            other => return Err(format!("read_bytes answered {other:?}").into()),
        }
    }
}

#[test]
fn of_two_sessions_started_and_one_ended_one_is_alive_and_one_ended() -> TestResult {
    let mut held = Held::start()?;
    let directory = held
        .dir
        .path()
        .to_str()
        .ok_or("the test folder is not UTF-8")?
        .to_owned();
    let first = started(&held.client, shell("first", &directory))?;
    started(&held.client, shell("second", &directory))?;
    match held.client.ask(&Act::End {
        session: "second".to_owned(),
    })? {
        Answer::Ended { .. } => {}
        other => return Err(format!("end answered {other:?}").into()),
    }
    let views = liveness(&held.client, None)?;
    assert_eq!(views.len(), 2);
    let alive = named(&views, "first")?;
    assert!(alive.alive && !alive.ended, "{alive:?}");
    assert_eq!(alive.pid, Some(first));
    assert!(!alive.managed);
    assert!(!alive.turn_active);
    let ended = named(&views, "second")?;
    assert!(!ended.alive && ended.ended, "{ended:?}");
    let only = liveness(&held.client, Some("first"))?;
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].session, "first");
    held.stop()
}

#[test]
fn a_session_killed_outside_lys_is_not_alive_once_its_exit_is_seen() -> TestResult {
    let mut held = Held::start()?;
    let directory = held
        .dir
        .path()
        .to_str()
        .ok_or("the test folder is not UTF-8")?
        .to_owned();
    let pid = started(&held.client, shell("killed", &directory))?;
    assert!(named(&liveness(&held.client, None)?, "killed")?.alive);
    let status = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status()?;
    assert!(status.success());
    until_ended(&held.client, "killed")?;
    let view = liveness(&held.client, Some("killed"))?;
    assert!(!view[0].alive && view[0].ended, "{:?}", view[0]);
    held.stop()
}

#[test]
fn liveness_of_a_session_the_runner_does_not_hold_is_refused_by_name() -> TestResult {
    let mut held = Held::start()?;
    match held.client.ask(&Act::Liveness {
        session: Some("never-started".to_owned()),
    }) {
        Err(RunnerError::Refused { refusal, .. }) => assert_eq!(refusal, "session_unknown"),
        other => return Err(format!("liveness of an unknown session answered {other:?}").into()),
    }
    held.stop()
}
