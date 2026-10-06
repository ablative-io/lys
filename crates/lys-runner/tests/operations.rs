#![cfg(test)]
//! DIRECTORY-051 R3: an operation is asked under the server's stable id and
//! done once. Asked again under that id it is answered as it stands; the id
//! naming another act is refused. A stop is confirmed only on the exit the
//! runner saw, and a stopped session does nothing more. An operation a
//! runner left being typed comes back uncertain and is never typed again;
//! one it left waiting comes back refused. Every wait ends on the runner's
//! answer, never a clock.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::operations::{
    FORMAT, Operation, OperationOutcome, OperationRequest, OperationState, TextDigest,
};
use lys_runner::{Act, Answer, Client, Launch, Options, Runner, RunnerError, Serving};

type TestResult = Result<(), Box<dyn Error>>;

struct Held {
    dir: tempfile::TempDir,
    key: Arc<Ed25519Identity>,
    serving: Option<Serving>,
}

impl Held {
    fn socket(&self) -> PathBuf {
        self.dir.path().join("runner.sock")
    }

    fn state(&self) -> PathBuf {
        self.dir.path().join("state")
    }

    fn start() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &dir.path().join("server.key"),
        )?);
        let mut held = Self {
            dir,
            key,
            serving: None,
        };
        held.serve()?;
        Ok(held)
    }

    fn serve(&mut self) -> TestResult {
        let options = Options {
            socket: self.socket(),
            state: self.state(),
            server_key: self.key.public_key_bytes(),
            scrollback: 1 << 16,
        };
        self.serving = Some(Runner::open(&options)?.spawn());
        Ok(())
    }

    fn client(&self) -> Client {
        Client::new(self.socket(), Arc::clone(&self.key))
    }

    fn stop(&mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

/// A session that says each line typed to it back once, started and ready.
fn cat(client: &Client, session: &str) -> TestResult {
    let launch = Launch {
        session: session.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "stty -echo; echo ready; cat".to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    };
    match client.ask(&Act::Start {
        lys_mcp: None,
        proxy: None,
        launch: Box::new(launch),
    })? {
        Answer::Started { .. } => {}
        other => return Err(format!("start answered {other:?}").into()),
    }
    shown(client, session, "ready")?;
    Ok(())
}

fn operation(id: &str, session: &str, request: OperationRequest) -> Operation {
    Operation {
        operation: id.to_owned(),
        session: session.to_owned(),
        request,
    }
}

fn notice(text: &str) -> OperationRequest {
    OperationRequest::Notice {
        text: text.to_owned(),
    }
}

fn operate(client: &Client, asked: Operation) -> Result<OperationOutcome, RunnerError> {
    match client.ask(&Act::Operate { operation: asked })? {
        Answer::Operation { outcome } => Ok(outcome),
        other => Err(RunnerError::refused("unexpected", format!("{other:?}"))),
    }
}

fn outcome(client: &Client, id: &str) -> Result<OperationOutcome, RunnerError> {
    let act = Act::Outcome {
        operation: id.to_owned(),
    };
    match client.ask(&act)? {
        Answer::Operation { outcome } => Ok(outcome),
        other => Err(RunnerError::refused("unexpected", format!("{other:?}"))),
    }
}

/// The session's output until `pattern` shows, from its start.
fn shown(client: &Client, session: &str, pattern: &str) -> Result<u64, Box<dyn Error>> {
    let act = Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: pattern.to_owned(),
        regex: false,
    };
    match client.ask(&act)? {
        Answer::Matched { cursor, .. } => Ok(cursor),
        other => Err(format!("wait answered {other:?}").into()),
    }
}

/// Everything the session showed until it ended.
fn until_ended(client: &Client, session: &str) -> Result<String, Box<dyn Error>> {
    let (mut cursor, mut text) = (Some(0), String::new());
    loop {
        let act = Act::Read {
            session: session.to_owned(),
            cursor,
            lines: None,
            bytes: None,
            follow: true,
        };
        let Answer::Output { output } = client.ask(&act)? else {
            return Err("a read answered something else".into());
        };
        text.push_str(&output.text);
        cursor = Some(output.cursor);
        if output.ended.is_some() {
            return Ok(text);
        }
    }
}

fn refusal(result: Result<OperationOutcome, RunnerError>) -> Result<String, Box<dyn Error>> {
    match result {
        Err(RunnerError::Refused { refusal, .. }) => Ok(refusal),
        other => Err(format!("expected a refusal, answered {other:?}").into()),
    }
}

#[test]
fn an_operation_asked_twice_is_typed_once() -> TestResult {
    let mut held = Held::start()?;
    let client = held.client();
    cat(&client, "s1")?;
    let first = operate(&client, operation("op-1", "s1", notice("context at 80%")))?;
    assert_eq!(first.request, "notice");
    assert_eq!(first.text, Some(TextDigest::of("context at 80%")));
    shown(&client, "s1", "context at 80%")?;
    let again = operate(&client, operation("op-1", "s1", notice("context at 80%")))?;
    assert_eq!(again.state, OperationState::Delivered, "{again:?}");
    assert_eq!(again.at, outcome(&client, "op-1")?.at);
    operate(&client, operation("op-2", "s1", OperationRequest::Stop))?;
    let text = until_ended(&client, "s1")?;
    assert_eq!(text.matches("context at 80%").count(), 1, "{text:?}");
    held.stop()
}

#[test]
fn an_id_naming_another_act_is_refused() -> TestResult {
    let mut held = Held::start()?;
    let client = held.client();
    cat(&client, "s1")?;
    operate(&client, operation("op-1", "s1", notice("one")))?;
    let reused = refusal(operate(&client, operation("op-1", "s1", notice("two"))))?;
    assert_eq!(reused, "operation_reused");
    assert_eq!(refusal(outcome(&client, "op-9"))?, "operation_unknown");
    held.stop()
}

#[test]
fn a_stop_is_confirmed_on_the_exit_and_the_session_does_nothing_more() -> TestResult {
    let mut held = Held::start()?;
    let client = held.client();
    cat(&client, "s1")?;
    let asked = operate(&client, operation("stop-1", "s1", OperationRequest::Stop))?;
    assert_ne!(asked.state, OperationState::Confirmed, "{asked:?}");
    until_ended(&client, "s1")?;
    let stopped = outcome(&client, "stop-1")?;
    assert_eq!(stopped.state, OperationState::Confirmed, "{stopped:?}");
    assert!(stopped.ended.is_some(), "a confirmed stop names its exit");
    let after = operate(&client, operation("op-2", "s1", notice("too late")))?;
    assert_eq!(after.state, OperationState::Refused, "{after:?}");
    assert!(after.words.starts_with("session_ended"), "{after:?}");
    held.stop()
}

fn kept(state: OperationState, id: &str) -> OperationOutcome {
    OperationOutcome {
        operation: id.to_owned(),
        session: "s1".to_owned(),
        request: "reminder".to_owned(),
        state,
        at: 1,
        words: "as left".to_owned(),
        text: Some(TextDigest::of("stand-up in 5 minutes")),
        ended: None,
    }
}

fn leave(state_dir: &Path, operations: &[OperationOutcome]) -> TestResult {
    std::fs::create_dir_all(state_dir)?;
    let record = serde_json::json!({ "format": FORMAT, "operations": operations });
    std::fs::remove_file(state_dir.join("operations.v2.journal"))?;
    std::fs::remove_file(state_dir.join("operations.v2.snapshot"))?;
    std::fs::write(state_dir.join("operations.json"), record.to_string())?;
    Ok(())
}

#[test]
fn a_runner_stopped_while_typing_leaves_it_uncertain_and_never_types_it_again() -> TestResult {
    let mut held = Held::start()?;
    held.stop()?;
    leave(
        &held.state(),
        &[
            kept(OperationState::Delivering, "op-typing"),
            kept(OperationState::Accepted, "op-waiting"),
        ],
    )?;
    held.serve()?;
    let client = held.client();
    let typing = outcome(&client, "op-typing")?;
    assert_eq!(typing.state, OperationState::Uncertain, "{typing:?}");
    let waiting = outcome(&client, "op-waiting")?;
    assert_eq!(waiting.state, OperationState::Uncertain, "{waiting:?}");
    assert!(
        waiting.words.contains("control_readback_unavailable"),
        "{waiting:?}"
    );
    cat(&client, "s1")?;
    let asked = operation(
        "op-typing",
        "s1",
        OperationRequest::Reminder {
            text: "stand-up in 5 minutes".to_owned(),
        },
    );
    let again = operate(&client, asked)?;
    assert_eq!(again.state, OperationState::Uncertain, "{again:?}");
    operate(&client, operation("op-stop", "s1", OperationRequest::Stop))?;
    let text = until_ended(&client, "s1")?;
    assert!(
        !text.contains("stand-up"),
        "an uncertain operation was typed: {text:?}"
    );
    held.stop()
}
