#![cfg(test)]
use super::Sessions;
use super::seat_program_launch_tests::{Fixture, PROGRAM};
use std::error::Error;
use std::sync::atomic::AtomicBool;
type Result = std::result::Result<(), Box<dyn Error>>;

#[test]
fn seat_program_stdin_is_kept_until_end_and_eof_records_zero() -> Result {
    let program = format!("{PROGRAM}\nsys.stdin.read()\nsys.exit(0)\n");
    let fixture = Fixture::new(&program, 0o600)?;
    let _entry = crate::harness_control::process::seat_test_support::entry(fixture.entry)?;
    let sessions = Sessions::open(fixture.dir.path(), 8192)?;
    sessions.start(fixture.launch)?;
    assert!(
        sessions.status(Some("seat-fixture"))?.sessions[0]
            .ended
            .is_none()
    );
    let ended = sessions.end("seat-fixture", &AtomicBool::new(false))?;
    assert_eq!(
        ended.status,
        Some(0),
        "ending the seat must close stdin and record its EOF exit"
    );
    assert!(
        ended.signal.is_none(),
        "EOF must end the seat without a signal"
    );
    Ok(())
}

#[test]
fn seat_program_stdin_records_an_independent_exit_three() -> Result {
    let program = format!("{PROGRAM}\nsys.exit(3)\n");
    let fixture = Fixture::new(&program, 0o600)?;
    let _entry = crate::harness_control::process::seat_test_support::entry(fixture.entry)?;
    let sessions = Sessions::open(fixture.dir.path(), 8192)?;
    sessions.start(fixture.launch)?;
    let ended = sessions.until("seat-fixture", &AtomicBool::new(false), |state, _| {
        state.ended().map(Ok)
    })?;
    assert_eq!(ended.status, Some(3));
    assert!(
        ended
            .reason
            .as_deref()
            .is_some_and(|words| words.contains("seat_program_exited") && words.contains('3')),
        "the independent seat exit must be named"
    );
    Ok(())
}

#[test]
fn seat_program_stdin_refuses_exit_before_the_first_answer() -> Result {
    first_answer_failure(
        "#!/usr/bin/python3\nimport sys\nsys.exit(3)\n",
        "seat_program_answer_missing",
    )
}

#[test]
fn seat_program_stdin_refuses_stdout_close_before_the_first_answer() -> Result {
    first_answer_failure(
        "#!/usr/bin/python3\nimport os, sys\nos.close(1)\nsys.stdin.read()\n",
        "seat_program_answer_missing",
    )
}

fn first_answer_failure(program: &str, name: &str) -> Result {
    let fixture = Fixture::new(program, 0o600)?;
    let _entry = crate::harness_control::process::seat_test_support::entry(fixture.entry)?;
    let sessions = Sessions::open(fixture.dir.path(), 8192)?;
    let result = sessions.start(fixture.launch);
    if result.is_ok() {
        sessions.end("seat-fixture", &AtomicBool::new(false))?;
    }
    let error = result
        .err()
        .ok_or("a seat without a first ACP answer was accepted")?;
    assert_eq!(error.name(), name, "{error}");
    Ok(())
}
