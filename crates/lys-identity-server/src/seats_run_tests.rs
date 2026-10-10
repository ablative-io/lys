//! A seat's start answers `seat_owner_held` by name when the runner already
//! holds the seat's session: the refusal is the runner's own, produced by
//! its public start, read back as the start path reads a runner's reply,
//! and rendered as the HTTP answer unchanged.

use std::error::Error;
use std::path::PathBuf;

use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use lys_runner::harness_control::ManagedLaunch;
use lys_runner::protocol::{Launch, read_reply, reply_line};
use lys_runner::{Answer, RunnerError, Sessions};
use serde_json::{Value, json};

use crate::error::ServerError;

type TestResult = Result<(), Box<dyn Error>>;

/// The session the runner holds and the seat's start names.
const SESSION: &str = "held";

/// The seat's start for `session` as the runner is asked it: a managed
/// launch carrying the typed owner binding a seat start sends.
fn supervised(session: &str) -> Result<ManagedLaunch, serde_json::Error> {
    serde_json::from_value(json!({
        "launch": {
            "session": session,
            "program": "claude",
            "arguments": [],
            "directory": "/",
            "environment": {},
            "columns": 120,
            "rows": 40
        },
        "transport": {"mode": "claude"},
        "conversation": "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f",
        "requires_controls": false,
        "owner": {
            "seat": format!("seat-{session}"),
            "session": session,
            "conversation": "0f3c9a1e-5b7d-4c2a-8e6f-1b3d5a7c9e2f",
            "generation": 1
        }
    }))
}

#[tokio::test]
async fn a_seat_start_answers_the_runners_held_owner_by_name() -> TestResult {
    // The runner already holds the session the seat's start names.
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(&dir.path().join("state"), 1 << 16)?;
    sessions.set_server_key([7; 32]);
    // Were an owner ever started here, it would start nothing.
    sessions.set_owner_program(PathBuf::from("/usr/bin/false"));
    let held: Launch = serde_json::from_value(json!({
        "session": SESSION,
        "program": "/bin/sh",
        "arguments": ["-c", "exit 0"],
        "directory": "/",
        "environment": {},
        "columns": 80,
        "rows": 24
    }))?;
    sessions.start(held)?;

    // The runner refuses a second owner for it, by name, through the start
    // a seat's managed start is answered by.
    let refused = sessions
        .start_managed(supervised(SESSION)?, None, None)
        .err()
        .ok_or("the runner started an owner for a session it holds")?;
    assert!(
        matches!(&refused, RunnerError::Refused { refusal, .. } if refusal == "seat_owner_held"),
        "{refused}"
    );
    assert!(
        sessions.owned_seats()?.is_empty(),
        "a refused start holds no owner"
    );

    // The refusal crosses the wire as the runner writes it and is read as
    // the server's runner client reads it.
    let line = reply_line(Answer::refusal(&refused));
    let read = read_reply(&line)
        .err()
        .ok_or("the runner's refusal read as an answer")?;
    let error = ServerError::from(read);
    assert!(
        matches!(&error, ServerError::Runner { refusal, .. } if refusal == "seat_owner_held"),
        "{error}"
    );

    // The seat's start answers it by name.
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body: Value =
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), usize::MAX).await?)?;
    assert_eq!(body["refusal"], "seat_owner_held");
    assert_eq!(
        body["reason"],
        format!("seat_owner_held: session {SESSION} is a session of this runner, not a seat")
    );
    Ok(())
}
