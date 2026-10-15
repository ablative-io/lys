#![cfg(test)]
//! Codex after-turn notifications reach the collector; unrelated ones send nothing.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;

use lys_runner::notify_line::{Args, hand, run};
use lys_runner::peer::{Collected, PeerAct, PeerRequest};
use lys_runner::protocol::{Answer, Greeting, PROTOCOL_VERSION, reply_line};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn notification() -> Value {
    json!({
        "type": "agent-turn-complete",
        "thread-id": "untrusted-payload-id",
        "turn-id": "turn",
        "cwd": "/work",
        "input-messages": ["whole input"],
        "last-assistant-message": "whole answer",
        "future_member": {"nested": true}
    })
}

fn with_server(
    answer: Answer,
    call: impl FnOnce(&Path) -> TestResult,
) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("notify.sock");
    let listener = UnixListener::bind(&socket)?;
    let server = std::thread::spawn(move || -> Result<String, String> {
        let serve = || -> Result<String, Box<dyn Error>> {
            let (mut stream, _) = listener.accept()?;
            writeln!(stream, "{}", Greeting::fresh(&"ab".repeat(32)).line())?;
            let mut line = String::new();
            BufReader::new(stream.try_clone()?).read_line(&mut line)?;
            writeln!(stream, "{}", reply_line(answer))?;
            Ok(line)
        };
        serve().map_err(|error| error.to_string())
    });
    let called = call(&socket);
    let line = server
        .join()
        .map_err(|error| format!("fixture server panicked: {error:?}"))??;
    called?;
    Ok(line)
}

#[test]
fn agent_turn_complete_is_handed_whole_as_exact_notify_line() -> TestResult {
    let input = notification();
    let line = with_server(
        Answer::Collected {
            words: "kept".to_owned(),
        },
        |socket| {
            assert!(matches!(
                hand(socket, &input.to_string())?,
                Answer::Collected { .. }
            ));
            Ok(())
        },
    )?;
    let expected = PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Collect(Collected::Notify {
            notification: input,
        }),
    };
    assert_eq!(line, format!("{}\n", serde_json::to_string(&expected)?));
    Ok(())
}

#[test]
fn another_type_sends_nothing_and_exits_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("notify.sock");
    let listener = UnixListener::bind(&socket)?;
    listener.set_nonblocking(true)?;
    for notification in [
        json!({"type":"other-event"}),
        json!({"future_member":true}),
        json!({"type":null}),
        json!({"type":17}),
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run(Args {
                socket: &socket,
                notification: &notification.to_string(),
                stdout: &mut stdout,
                stderr: &mut stderr,
            }),
            0
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        match listener.accept() {
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock),
            Ok(_) => return Err("ignored notification opened a connection".into()),
        }
    }
    Ok(())
}

#[test]
fn malformed_notification_and_missing_runner_are_one_line_and_exit_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    for (input, cause) in [
        (
            "not-json-private-input".to_owned(),
            "notify_input_malformed",
        ),
        (notification().to_string(), "runner_unreachable"),
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run(Args {
                socket: &dir.path().join("absent.sock"),
                notification: &input,
                stdout: &mut stdout,
                stderr: &mut stderr,
            }),
            0
        );
        assert!(stdout.is_empty());
        let error = String::from_utf8(stderr)?;
        assert_eq!(error.lines().count(), 1);
        assert!(error.contains(cause), "{error}");
        assert!(!error.contains("private-input"));
    }
    Ok(())
}

#[test]
fn success_and_refusal_both_exit_zero_without_stdout() -> TestResult {
    for answer in [
        Answer::Collected {
            words: "kept".to_owned(),
        },
        Answer::Refused {
            refusal: "not_a_session".to_owned(),
            words: "unproved caller\nsecond line".to_owned(),
            oldest: None,
        },
    ] {
        let refused = matches!(answer, Answer::Refused { .. });
        with_server(answer, |socket| {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(
                run(Args {
                    socket,
                    notification: &notification().to_string(),
                    stdout: &mut stdout,
                    stderr: &mut stderr,
                }),
                0
            );
            assert!(stdout.is_empty());
            let error = String::from_utf8(stderr)?;
            assert_eq!(error.lines().count(), usize::from(refused));
            if refused {
                assert!(error.contains("not_a_session"), "{error}");
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn a_silent_runner_is_refused_on_socket_close(greet: bool) -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("silent.sock");
    let listener = UnixListener::bind(&socket)?;
    let (done, answer) = std::sync::mpsc::channel();
    let client = std::thread::spawn(move || {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(Args {
            socket: &socket,
            notification: &notification().to_string(),
            stdout: &mut stdout,
            stderr: &mut stderr,
        });
        done.send((code, stdout, stderr))
            .map_err(|error| error.to_string())
    });
    let (mut stream, _) = listener.accept()?;
    if greet {
        writeln!(stream, "{}", Greeting::fresh(&"ab".repeat(32)).line())?;
        let mut line = String::new();
        BufReader::new(stream.try_clone()?).read_line(&mut line)?;
        assert!(!line.is_empty());
    }
    assert!(matches!(
        answer.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    drop(stream);
    let (code, stdout, stderr) = answer.recv()?;
    client
        .join()
        .map_err(|error| format!("fixture client panicked: {error:?}"))??;
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    let error = String::from_utf8(stderr)?;
    assert_eq!(error.lines().count(), 1);
    assert!(
        error.contains("runner closed before the line ended"),
        "{error}"
    );
    assert!(error.contains(if greet {
        "reading collector answer"
    } else {
        "reading runner greeting"
    }));
    Ok(())
}

#[test]
fn a_closed_greeting_is_refused_on_its_close_event() -> TestResult {
    a_silent_runner_is_refused_on_socket_close(false)
}

#[test]
fn a_closed_answer_is_refused_on_its_close_event() -> TestResult {
    a_silent_runner_is_refused_on_socket_close(true)
}
