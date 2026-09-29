//! The sessions file: what is kept reads back as the same sessions, keyed
//! by the hash of the secret; an ended session is dropped as the file is
//! read; a file in another format is refused by name; no file is none.

use std::collections::HashMap;
use std::error::Error;

use lys_identity::{Actor, AgentId, AuthMethod, LoginBinding, Provenance};
use lys_identity_server::error::ServerError;
use lys_identity_server::session::SessionEntry;
use lys_identity_server::session_store::{FORMAT, load, save};

type TestResult = Result<(), Box<dyn Error>>;

fn entry(id: &str, method: AuthMethod, ends_at: u64) -> Result<SessionEntry, Box<dyn Error>> {
    Ok(SessionEntry {
        id: id.to_owned(),
        actor: Actor::new(
            LoginBinding::new("https://issuer.example.test", "ada-subject")?,
            Provenance::new(method, 90),
        ),
        started_at: 100,
        ends_at,
    })
}

#[test]
fn kept_sessions_read_back_and_an_ended_one_is_dropped() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("sessions.json");
    assert!(load(&file, 150)?.is_empty(), "no file is no sessions");
    let agent: AgentId = "agent-00000000000000000000000000000002".parse()?;
    let mut live = HashMap::new();
    live.insert("a".repeat(64), entry("one", AuthMethod::Oidc, 200)?);
    live.insert(
        "b".repeat(64),
        entry("two", AuthMethod::AgentSignature(agent), 200)?,
    );
    live.insert("c".repeat(64), entry("three", AuthMethod::Oidc, 150)?);
    save(&file, &live)?;
    let read = load(&file, 150)?;
    live.remove(&"c".repeat(64));
    assert_eq!(read, live);
    Ok(())
}

#[test]
fn a_file_in_another_format_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("sessions.json");
    std::fs::write(
        &file,
        r#"{"format":"lys-directory-sessions/v0","sessions":[]}"#,
    )?;
    match load(&file, 0) {
        Err(ServerError::SessionsUnavailable { reason }) => {
            assert!(reason.contains(FORMAT), "{reason}");
        }
        other => return Err(format!("another format was read: {other:?}").into()),
    }
    Ok(())
}
