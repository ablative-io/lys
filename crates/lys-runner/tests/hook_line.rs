#![cfg(test)]
//! Hook intake preserves the whole input and cannot block the harness on a refusal.

use std::error::Error;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;

use lys_runner::hook_line::{Args, hand, run};
use lys_runner::peer::{Collected, PeerAct, PeerRequest};
use lys_runner::protocol::{Answer, Greeting, PROTOCOL_VERSION, reply_line};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn input(event: &str) -> Value {
    json!({
        "hook_event_name": event,
        "session_id": "untrusted-payload-id",
        "transcript_path": "/work/transcript.jsonl",
        "cwd": "/work",
        "tool_input": {"command": "private input\nsecond line"},
        "future_member": [true, null, {"nested": "whole"}]
    })
}

fn with_server(
    answer: Answer,
    call: impl FnOnce(&Path) -> TestResult,
) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("hook.sock");
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

fn collected() -> Answer {
    Answer::Collected {
        words: "kept".to_owned(),
    }
}

fn event_is_handed_whole(event: &str) -> TestResult {
    let input = input(event);
    let expected = PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Collect(Collected::Hook {
            event: event.to_owned(),
            input: input.clone(),
        }),
    };
    let line = with_server(collected(), |socket| {
        assert_eq!(hand(socket, &input.to_string(), None)?, collected());
        Ok(())
    })?;
    assert_eq!(line, format!("{}\n", serde_json::to_string(&expected)?));
    Ok(())
}

macro_rules! hook_case {
    ($name:ident, $event:literal) => {
        #[test]
        fn $name() -> TestResult {
            event_is_handed_whole($event)
        }
    };
}

hook_case!(session_start, "SessionStart");
hook_case!(user_prompt_submit, "UserPromptSubmit");
hook_case!(stop, "Stop");
hook_case!(session_end, "SessionEnd");
hook_case!(pre_compact, "PreCompact");
hook_case!(post_compact, "PostCompact");
hook_case!(post_tool_use, "PostToolUse");
hook_case!(post_tool_use_failure, "PostToolUseFailure");

#[test]
fn missing_socket_exits_zero_with_empty_stdout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run(Args {
        socket: &dir.path().join("absent.sock"),
        event: None,
        stdin: &mut Cursor::new(input("Stop").to_string()),
        stdout: &mut stdout,
        stderr: &mut stderr,
    });
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    let error = String::from_utf8(stderr)?;
    assert_eq!(error.lines().count(), 1);
    assert!(error.contains("runner_unreachable"), "{error}");
    Ok(())
}

#[test]
fn malformed_stdin_exits_zero_with_one_stderr_line() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run(Args {
        socket: &dir.path().join("absent.sock"),
        event: None,
        stdin: &mut Cursor::new("not-json-private-input"),
        stdout: &mut stdout,
        stderr: &mut stderr,
    });
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    let error = String::from_utf8(stderr)?;
    assert_eq!(error.lines().count(), 1);
    assert!(error.contains("hook_input_malformed"), "{error}");
    assert!(!error.contains("private-input"));
    assert!(!error.contains("runner_unreachable"));
    Ok(())
}

#[test]
fn missing_or_invalid_event_and_non_object_input_are_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    for input in [
        "{}",
        r#"{"hook_event_name":null}"#,
        r#"{"hook_event_name":17}"#,
        r#"{"hook_event_name":" "}"#,
        "[]",
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run(Args {
                socket: &dir.path().join("absent.sock"),
                event: None,
                stdin: &mut Cursor::new(input),
                stdout: &mut stdout,
                stderr: &mut stderr,
            }),
            0
        );
        assert!(stdout.is_empty());
        let error = String::from_utf8(stderr)?;
        assert_eq!(error.lines().count(), 1);
        assert!(error.contains("hook_input_malformed"), "{error}");
        assert!(!error.contains("runner_unreachable"));
    }
    Ok(())
}

#[test]
fn explicit_event_overrides_the_payload_without_changing_it() -> TestResult {
    for input in [input("Stop"), json!({"future_member": true})] {
        let line = with_server(collected(), |socket| {
            hand(socket, &input.to_string(), Some("PostCompact"))?;
            Ok(())
        })?;
        let expected = PeerRequest {
            version: PROTOCOL_VERSION,
            peer: PeerAct::Collect(Collected::Hook {
                event: "PostCompact".to_owned(),
                input,
            }),
        };
        assert_eq!(line, format!("{}\n", serde_json::to_string(&expected)?));
    }
    Ok(())
}

#[test]
fn success_and_any_other_answer_leave_stdout_empty() -> TestResult {
    for answer in [collected(), Answer::GrantChannel] {
        with_server(answer, |socket| {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(
                run(Args {
                    socket,
                    event: None,
                    stdin: &mut Cursor::new(input("Stop").to_string()),
                    stdout: &mut stdout,
                    stderr: &mut stderr,
                }),
                0
            );
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn runner_refusal_is_one_stderr_line_and_exit_zero() -> TestResult {
    with_server(
        Answer::Refused {
            refusal: "not_a_session".to_owned(),
            words: "unproved caller\nsecond line\rthird line".to_owned(),
            oldest: None,
        },
        |socket| {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(
                run(Args {
                    socket,
                    event: None,
                    stdin: &mut Cursor::new(input("Stop").to_string()),
                    stdout: &mut stdout,
                    stderr: &mut stderr,
                }),
                0
            );
            assert!(stdout.is_empty());
            let error = String::from_utf8(stderr)?;
            assert_eq!(error.lines().count(), 1);
            assert!(error.contains("not_a_session"), "{error}");
            Ok(())
        },
    )?;
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
            event: None,
            stdin: &mut Cursor::new(input("Stop").to_string()),
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
