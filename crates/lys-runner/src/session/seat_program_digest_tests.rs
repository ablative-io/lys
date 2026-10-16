#![cfg(test)]
use super::Sessions;
use super::seat_program_launch_tests::{Fixture, PROGRAM};
use serde_json::Value;
use std::error::Error;
use std::sync::atomic::AtomicBool;
type Result = std::result::Result<(), Box<dyn Error>>;

#[test]
fn seat_program_changed_seat_file_starts_and_marks_reviewed_and_found_digests() -> Result {
    changed(false)
}
#[test]
fn seat_program_changed_executable_starts_and_marks_reviewed_and_found_digests() -> Result {
    changed(true)
}
fn changed(executable: bool) -> Result {
    let program = format!("{PROGRAM}\nsys.stdin.read()\n");
    let fixture = Fixture::new(&program, 0o600)?;
    let path = if executable {
        &fixture.spec.executable
    } else {
        &fixture.spec.seat_file
    };
    let bytes = if executable {
        format!("{program}\n# a changed fixture executable\n").into_bytes()
    } else {
        b"changed opaque seat bytes".to_vec()
    };
    std::fs::write(path, &bytes)?;
    let found = lys_home::record::blocks::Hash::of(&bytes)
        .as_str()
        .to_owned();
    let admission = crate::harness_control::process::seat_test_support::entry(fixture.entry)?;
    let sessions = Sessions::open(fixture.dir.path(), 16384)?;
    sessions.start(fixture.launch)?;
    let status = sessions.status(Some("seat-fixture"))?;
    let marked = status.sessions[0].qualification.clone();
    sessions.end("seat-fixture", &AtomicBool::new(false))?;
    let marked: Value = serde_json::from_str(
        &marked.ok_or("changed reviewed bytes started without a digest mark")?,
    )?;
    assert_eq!(marked["contract"], "seat-program/v1");
    assert_eq!(
        marked["reviewed_executable_sha256"],
        fixture.spec.executable_sha256
    );
    assert_eq!(
        marked["reviewed_seat_file_sha256"],
        fixture.spec.seat_file_sha256
    );
    let field = if executable {
        "found_executable_sha256"
    } else {
        "found_seat_file_sha256"
    };
    assert_eq!(marked[field], found);
    drop(admission);
    Ok(())
}

#[test]
fn seat_program_refusal_does_not_quote_the_agent_pass_input() -> Result {
    let mut fixture = Fixture::new(PROGRAM, 0o600)?;
    let input = format!("invalid-proof-input-{}", fixture.dir.path().display());
    fixture
        .launch
        .environment
        .insert("LYS_AGENT_PASS".to_owned(), input.clone());
    let error = crate::harness_control::seat_program::prepare(&mut fixture.launch)
        .err()
        .ok_or("an invalid run proof file was admitted")?;
    assert_eq!(error.name(), "seat_pass_not_private");
    assert!(
        !error.to_string().contains(&input),
        "the refusal quoted the agent pass input"
    );
    Ok(())
}
