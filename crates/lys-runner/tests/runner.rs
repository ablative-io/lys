#![cfg(test)]
//! DIRECTORY-050 R1: Lys's own runner holds each session in its own
//! pseudo-terminal, keeps it running whoever connects, records its end, and
//! reports every session it held when it is started again: ended with its
//! exit and instant after a clean stop, and `ended_by_runner_restart` with
//! no exit status after one it lost. It answers only the server's signed
//! requests, on a socket only its owner may open. R5: a session at a usage
//! limit moves to its next account, and stops `accounts_exhausted` at the
//! list's end. Every wait below ends on the runner's answer, never a clock.

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::protocol::{Greeting, Output, Request, StatusView, hex, signed_bytes};
use lys_runner::state::{Kept, KeptSession};
use lys_runner::{
    Act, Answer, Client, EndedHow, Launch, Limit, Options, Rotation, Runner, RunnerError, Serving,
};

type TestResult = Result<(), Box<dyn Error>>;

#[path = "support/restart_child.rs"]
mod restart_child;
use restart_child::PtyChild;

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

    fn options(&self, socket: &Path, scrollback: usize) -> Options {
        Options {
            socket: socket.to_owned(),
            state: self.state(),
            server_key: self.key.public_key_bytes(),
            scrollback,
        }
    }

    fn start(scrollback: usize) -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &dir.path().join("server.key"),
        )?);
        let mut held = Self {
            dir,
            key,
            serving: None,
        };
        let runner = Runner::open(&held.options(&held.socket(), scrollback))?;
        held.serving = Some(runner.spawn());
        Ok(held)
    }

    /// A client as a server started afresh would make one: the same key,
    /// nothing else carried over.
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

fn shell(session: &str, script: &str) -> Launch {
    Launch {
        session: session.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), script.to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

fn started(client: &Client, launch: Launch) -> TestResult {
    match client.ask(&Act::Start {
        launch: Box::new(launch),
    })? {
        Answer::Started { pid, .. } => {
            assert!(pid > 0);
            Ok(())
        }
        other => Err(format!("start answered {other:?}").into()),
    }
}

fn waited(client: &Client, session: &str, pattern: &str) -> Result<String, Box<dyn Error>> {
    let act = Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: pattern.to_owned(),
        regex: false,
    };
    match client.ask(&act)? {
        Answer::Matched { matched, .. } => Ok(matched),
        other => Err(format!("wait answered {other:?}").into()),
    }
}

/// Follow the session's output until it ends, answering what was read of
/// it. Output that scrolled past before it was read is skipped, from the
/// oldest byte still kept.
fn until_ended(client: &Client, session: &str) -> Result<Output, Box<dyn Error>> {
    let mut cursor = None;
    let mut text = String::new();
    loop {
        let act = Act::Read {
            session: session.to_owned(),
            cursor,
            lines: None,
            bytes: None,
            follow: true,
        };
        let output = match client.ask(&act) {
            Ok(Answer::Output { output }) => output,
            Err(RunnerError::Refused { refusal, .. }) if refusal == "cursor_expired" => {
                cursor = None;
                continue;
            }
            other => return Err(format!("read answered {other:?}").into()),
        };
        text.push_str(&output.text);
        cursor = Some(output.cursor);
        if output.ended.is_some() {
            return Ok(Output { text, ..output });
        }
    }
}

fn status(client: &Client) -> Result<StatusView, Box<dyn Error>> {
    match client.ask(&Act::Status { session: None })? {
        Answer::Status { status } => Ok(status),
        other => Err(format!("status answered {other:?}").into()),
    }
}

#[test]
fn a_session_keeps_running_when_its_server_goes_and_is_read_by_the_next() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let first = held.client();
    started(&first, shell("kept", "echo kept-$((20+1)); exec cat"))?;
    assert_eq!(waited(&first, "kept", "kept-21")?, "kept-21");
    drop(first);

    let next = held.client();
    let seen = status(&next)?;
    let kept = seen
        .sessions
        .iter()
        .find(|s| s.session == "kept")
        .ok_or("kept is not held")?;
    assert!(kept.ended.is_none(), "{kept:?}");
    let Answer::Output { output } = next.ask(&Act::Read {
        session: "kept".to_owned(),
        cursor: None,
        lines: None,
        bytes: None,
        follow: false,
    })?
    else {
        return Err("read answered no output".into());
    };
    assert!(output.text.contains("kept-21"), "{output:?}");
    held.stop()
}

#[test]
fn a_clean_restart_reports_each_session_ended_with_its_instant() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    started(&client, shell("done", "exit 3"))?;
    let ended = until_ended(&client, "done")?.ended.ok_or("no end")?;
    assert_eq!((ended.how, ended.status), (EndedHow::Exited, Some(3)));
    started(&client, shell("running", "exec cat"))?;
    held.stop()?;

    let reopened = Runner::open(&held.options(&held.socket(), 1 << 16))?;
    let seen = reopened.sessions().status(None)?;
    assert_eq!(seen.sessions.len(), 2);
    for session in &seen.sessions {
        let end = session.ended.as_ref().ok_or("a session reads unended")?;
        assert_eq!(end.how, EndedHow::Exited, "{session:?}");
        assert!(end.at >= session.started_at, "{session:?}");
    }
    let done = seen
        .sessions
        .iter()
        .find(|s| s.session == "done")
        .ok_or("done")?;
    assert_eq!(done.ended.as_ref().and_then(|end| end.status), Some(3));
    Ok(())
}

/// One session as a lost runner's record keeps it: started, never ended.
fn unended(session: &str, pid: u32, leader_start: Option<lys_runner::peer::Leader>) -> KeptSession {
    KeptSession {
        session: session.to_owned(),
        pid: Some(pid),
        leader_start,
        started_at: 1,
        columns: 80,
        rows: 24,
        ended: None,
    }
}

#[test]
fn a_session_lost_with_its_runner_is_ended_by_the_restart_with_no_status() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    // What a runner that was lost leaves: its record, naming one process
    // group that is gone and one whose process ignored the hang-up and
    // still runs. The live fixture uses a launch's session creation.
    let mut gone = Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .process_group(0)
        .spawn()?;
    let gone_pid = gone.id();
    gone.wait()?;
    let mut left = PtyChild::start()?;
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state)?;
    let record = Kept::new(vec![
        unended("gone", gone_pid, None),
        unended(
            "left",
            left.id(),
            Some(lys_runner::peer::Leader {
                pid: left.id(),
                start: lys_runner::peer::start_identity(left.id())?,
            }),
        ),
    ]);
    std::fs::write(state.join("sessions.json"), serde_json::to_vec(&record)?)?;

    let restarted = Runner::open(&Options {
        socket: dir.path().join("runner.sock"),
        state,
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    })?;
    let seen = restarted.sessions().status(None)?;
    let end = |id: &str| {
        seen.sessions
            .iter()
            .find(|session| session.session == id)
            .and_then(|session| session.ended.clone())
            .ok_or(format!("{id} reads unended"))
    };
    let (gone_end, left_end) = (end("gone")?, end("left")?);
    for ended in [&gone_end, &left_end] {
        assert_eq!(ended.how, EndedHow::EndedByRunnerRestart);
        assert_eq!(ended.status, None, "no exit status is invented");
        assert!(ended.at >= 1);
    }
    assert_eq!(gone_end.signal, None, "nothing was left to end");
    assert_eq!(
        left_end.signal.as_deref(),
        Some("SIGKILL"),
        "what the lost runner left running is ended before it is reported gone"
    );
    let exit = left.wait()?;
    assert_eq!(
        exit.signal(),
        Some(9),
        "the left process really ended: {exit:?}"
    );
    Ok(())
}

#[test]
fn a_second_runner_on_the_same_state_is_refused_by_name() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let other = held.dir.path().join("other.sock");
    let second = Runner::open(&held.options(&other, 1 << 16));
    assert!(
        matches!(second, Err(RunnerError::StateHeld { .. })),
        "a second runner on one record is refused"
    );
    held.stop()?;
    let after = Runner::open(&held.options(&other, 1 << 16));
    assert!(after.is_ok(), "the record is free once its runner stops");
    Ok(())
}

#[test]
fn a_scrollback_that_keeps_nothing_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let refused = Runner::open(&Options {
        socket: dir.path().join("runner.sock"),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 0,
    })
    .err()
    .ok_or("a runner opened with a scrollback of no bytes")?;
    assert_eq!(refused.name(), "scrollback_invalid", "{refused}");
    assert!(
        !dir.path().join("state").exists(),
        "nothing is kept before the refusal"
    );
    Ok(())
}

#[test]
fn a_request_answered_before_a_restart_is_refused_after_it() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let mut connection = held.client().connect()?;
    let greeting = connection.greeting()?;
    let line =
        lys_runner::protocol::sign_request(&held.key, &greeting, &Act::Status { session: None })?;
    let answered: lys_runner::protocol::Reply = serde_json::from_str(&connection.exchange(&line)?)?;
    assert!(matches!(answered.answer, Answer::Status { .. }));
    held.stop()?;

    let reopened = Runner::open(&held.options(&held.socket(), 1 << 16))?;
    held.serving = Some(reopened.spawn());
    let mut again = held.client().connect()?;
    assert_eq!(
        again.greeting()?.runner,
        greeting.runner,
        "the runner keeps its id across the restart"
    );
    let replayed: lys_runner::protocol::Reply = serde_json::from_str(&again.exchange(&line)?)?;
    assert_eq!(refusal(&replayed.answer), "runner_request_replayed");
    held.stop()
}

/// Send the line `make` writes for the greeting of a fresh connection, and
/// read the runner's answer.
fn raw(
    held: &Held,
    make: impl FnOnce(&Greeting) -> Result<String, Box<dyn Error>>,
) -> Result<Answer, Box<dyn Error>> {
    let mut connection = lys_runner::connect(&held.socket())?;
    let greeting = connection.greeting()?;
    let reply = connection.exchange(&make(&greeting)?)?;
    let parsed: lys_runner::protocol::Reply = serde_json::from_str(&reply)?;
    Ok(parsed.answer)
}

fn refusal(answer: &Answer) -> &str {
    match answer {
        Answer::Refused { refusal, .. } => refusal,
        _ => "",
    }
}

#[test]
fn a_request_the_server_did_not_sign_is_refused_by_name() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let act = serde_json::to_string(&Act::Start {
        launch: Box::new(shell("forged", "exec cat")),
    })?;
    let stranger = Ed25519Identity::load_or_generate(&held.dir.path().join("stranger.key"))?;
    let mut refused = 0;
    for forged in [false, true] {
        let answer = raw(&held, |greeting| {
            let signature = if forged {
                hex(&stranger.sign(&signed_bytes(
                    1,
                    &greeting.runner,
                    &greeting.challenge,
                    &act,
                )))
            } else {
                String::new()
            };
            Ok(serde_json::to_string(&Request {
                version: 1,
                runner: greeting.runner.clone(),
                challenge: greeting.challenge.clone(),
                act: act.clone(),
                signature,
            })?)
        })?;
        assert_eq!(refusal(&answer), "runner_request_unsigned", "{answer:?}");
        refused += 1;
    }
    assert_eq!(refused, 2);
    assert!(
        status(&held.client())?.sessions.is_empty(),
        "nothing was started"
    );
    held.stop()
}

#[test]
fn the_socket_is_its_owners_alone() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let mode = std::fs::metadata(held.socket())?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let second = Runner::open(&held.options(&held.socket(), 1 << 16));
    assert!(matches!(second, Err(RunnerError::AlreadyRunning { .. })));
    held.stop()
}

#[test]
fn a_cursor_older_than_the_scrollback_is_refused_naming_the_oldest() -> TestResult {
    let mut held = Held::start(64)?;
    let client = held.client();
    started(&client, shell("long", "printf '%0200d' 0; echo; exit 0"))?;
    let end = until_ended(&client, "long")?.ended.ok_or("no end")?;
    assert_eq!(end.status, Some(0));
    let answer = client.ask(&Act::Read {
        session: "long".to_owned(),
        cursor: Some(0),
        lines: None,
        bytes: None,
        follow: false,
    });
    match answer {
        Err(RunnerError::Refused {
            refusal,
            oldest: Some(oldest),
            words,
        }) => {
            assert_eq!(refusal, "cursor_expired");
            assert!(oldest > 0);
            assert!(words.contains(&oldest.to_string()), "{words}");
        }
        other => return Err(format!("an expired cursor answered {other:?}").into()),
    }
    held.stop()
}

fn rotating(session: &str, script: &str, limit: Limit) -> Launch {
    Launch {
        rotation: Some(Rotation {
            accounts: vec!["h-account-one".to_owned(), "h-account-two".to_owned()],
            variable: "LYS_ACCOUNT_HANDLE".to_owned(),
            limit,
            resume_arguments: Vec::new(),
        }),
        ..shell(session, script)
    }
}

#[test]
fn a_usage_limit_moves_the_session_on_and_the_list_end_stops_it() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo on-$LYS_ACCOUNT_HANDLE; exit 7";
    started(
        &client,
        rotating("rotates", script, Limit::ExitStatus { status: 7 }),
    )?;
    let output = until_ended(&client, "rotates")?;
    assert!(output.text.contains("on-h-account-one"), "{output:?}");
    assert!(output.text.contains("on-h-account-two"), "{output:?}");
    let end = output.ended.ok_or("no end")?;
    assert_eq!(end.how, EndedHow::AccountsExhausted);
    let seen = client.ask(&Act::Status {
        session: Some("rotates".to_owned()),
    })?;
    let Answer::Status { status } = seen else {
        return Err("status answered no status".into());
    };
    let moves = &status.sessions[0].moves;
    assert_eq!(
        moves.len(),
        2,
        "one move on, one at the list's end: {moves:?}"
    );
    assert_eq!(
        (moves[0].from.as_str(), moves[0].to.as_str()),
        ("h-account-one", "h-account-two")
    );
    assert_eq!(moves[1].to, "", "the list does not wrap round");
    held.stop()
}

#[test]
fn words_the_harness_did_not_signal_move_nothing() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo usage limit reached; exit 0";
    started(
        &client,
        rotating("quoted", script, Limit::ExitStatus { status: 7 }),
    )?;
    let end = until_ended(&client, "quoted")?.ended.ok_or("no end")?;
    assert_eq!((end.how, end.status), (EndedHow::Exited, Some(0)));
    held.stop()
}

#[test]
fn declared_words_move_a_harness_with_no_signal() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo \"at $LYS_ACCOUNT_HANDLE: usage limit reached\"; exec cat";
    let limit = Limit::Words {
        words: vec!["usage limit reached".to_owned()],
    };
    started(&client, rotating("words", script, limit))?;
    let output = until_ended(&client, "words")?;
    assert!(output.text.contains("at h-account-two"), "{output:?}");
    assert_eq!(
        output.ended.ok_or("no end")?.how,
        EndedHow::AccountsExhausted
    );
    held.stop()
}

#[test]
fn a_regex_match_after_bytes_that_are_not_text_answers_the_byte_cursor() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    started(&client, shell("bytes", "printf '\\377x-marker'; exec cat"))?;
    let act = Act::Wait {
        session: "bytes".to_owned(),
        cursor: Some(0),
        pattern: "x-mark(er)".to_owned(),
        regex: true,
    };
    let Answer::Matched {
        matched, cursor, ..
    } = client.ask(&act)?
    else {
        return Err("the wait answered no match".into());
    };
    assert_eq!(matched, "x-marker");
    assert_eq!(
        cursor, 9,
        "one byte that is not text, then the 8 of x-marker"
    );
    let Answer::Output { output } = client.ask(&Act::Read {
        session: "bytes".to_owned(),
        cursor: Some(cursor),
        lines: None,
        bytes: None,
        follow: false,
    })?
    else {
        return Err("a read from the match's cursor answered no output".into());
    };
    assert_eq!(output.from, 9);
    held.stop()
}

#[test]
fn raw_terminal_preserves_control_invalid_and_split_character_bytes() -> TestResult {
    let mut held = Held::start(4096)?;
    let client = held.client();
    let id = "raw-terminal";
    started(
        &client,
        shell(
            id,
            r"stty raw -echo; printf '\033[31m\377\342'; dd bs=1 count=2 2>/dev/null",
        ),
    )?;
    let expected = b"\x1b[31m\xff\xe2";
    let mut received = Vec::new();
    let mut cursor = 0;
    while received.len() < expected.len() {
        let Answer::Bytes { output } = client.ask(&Act::ReadBytes {
            session: id.to_owned(),
            cursor: Some(cursor),
            follow: true,
        })?
        else {
            return Err("raw read did not answer bytes".into());
        };
        assert_eq!(output.from, cursor);
        assert_eq!(output.cursor - output.from, output.data.len() as u64);
        cursor = output.cursor;
        received.extend(output.data);
    }
    assert_eq!(received, expected);
    let input = vec![0, 0xfe];
    assert!(matches!(
        client.ask(&Act::InputBytes {
            session: id.to_owned(),
            data: input.clone()
        })?,
        Answer::Delivered { .. }
    ));
    let mut echoed = Vec::new();
    loop {
        let Answer::Bytes { output } = client.ask(&Act::ReadBytes {
            session: id.to_owned(),
            cursor: Some(cursor),
            follow: true,
        })?
        else {
            return Err("raw read did not answer bytes".into());
        };
        assert_eq!(output.from, cursor);
        cursor = output.cursor;
        echoed.extend(output.data);
        if output.ended.is_some() {
            break;
        }
    }
    assert_eq!(echoed, input);
    held.stop()
}
