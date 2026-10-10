#![cfg(test)]
//! AGENTS-002 R4: a managed session's frames are kept as rendered lines, a
//! person's turn among them, and a read that follows answers once a line
//! is rendered after its cursor; never on a clock.

use super::{AttachLine, Ring, USER};
use crate::harness_control::{Settings, Transport};
use crate::protocol::{Answer, Launch};
use crate::session::Sessions;
use serde_json::json;
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const CONVERSATION: &str = "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee";

/// A session whose process is a shell, held as the managed channel's
/// reader holds one, so frames are rendered into it as they would be read.
fn managed(sessions: &Arc<Sessions>, id: &str) -> TestResult<u64> {
    sessions.start(Launch {
        session: id.to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "cat".to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    let mut table = sessions.lock()?;
    let session = table
        .sessions
        .get_mut(id)
        .ok_or("the session is not held")?;
    session.managed = Some(Settings {
        transport: Transport::Claude,
        conversation: CONVERSATION.to_owned(),
    });
    Ok(session.generation)
}

fn lines(answer: Answer) -> TestResult<(Vec<AttachLine>, u64, bool)> {
    match answer {
        Answer::AttachLines {
            lines,
            cursor,
            ended,
        } => Ok((lines, cursor, ended)),
        other => Err(format!("attach_read answered {other:?}").into()),
    }
}

#[test]
fn a_managed_sessions_user_turn_is_an_attach_line_of_kind_user() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sessions = Sessions::open(directory.path(), 4096)?;
    let generation = managed(&sessions, "seat")?;
    let turn = json!({"type":"user","session_id":CONVERSATION,"uuid":"u-1",
        "parent_tool_use_id":null,"message":{"role":"user","content":"hello from the person"}});
    sessions.attach_frame("seat", generation, Transport::Claude, &turn)?;
    let (read, cursor, ended) =
        lines(sessions.attach_read("seat", None, false, &AtomicBool::new(false))?)?;
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].kind, USER);
    assert_eq!(read[0].text, "hello from the person");
    assert_eq!(cursor, 1);
    assert!(!ended);
    let held = sessions.liveness(Some("seat"))?;
    assert!(held[0].last_signal_at.is_some());
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn a_following_read_answers_with_the_next_rendered_line() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sessions = Sessions::open(directory.path(), 4096)?;
    let generation = managed(&sessions, "seat")?;
    let reader = {
        let sessions = Arc::clone(&sessions);
        std::thread::spawn(move || {
            sessions
                .attach_read("seat", Some(0), true, &AtomicBool::new(false))
                .map_err(|error| error.to_string())
        })
    };
    let said = json!({"type":"assistant","session_id":CONVERSATION,
        "message":{"content":[{"type":"text","text":"an answer"},
        {"type":"tool_use","name":"Read","input":{"file_path":"/a"}}]}});
    sessions.attach_frame("seat", generation, Transport::Claude, &said)?;
    let answer = reader.join().map_err(|panic| format!("{panic:?}"))??;
    let (read, cursor, ended) = lines(answer)?;
    assert!(!ended);
    assert_eq!(read[0].kind, "assistant");
    assert_eq!(read[0].text, "an answer");
    assert_eq!(read[1].kind, "tool_call");
    assert!(read[1].text.starts_with("Read "), "{}", read[1].text);
    assert_eq!(cursor, 2);
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn the_ring_keeps_its_bound_and_names_the_oldest_cursor_it_holds() -> TestResult {
    let mut ring = Ring::new(8);
    for text in ["one", "two", "three"] {
        ring.push(AttachLine {
            at: 1,
            kind: USER.to_owned(),
            text: text.to_owned(),
        });
    }
    assert_eq!((ring.oldest(), ring.end()), (1, 3));
    let refused = ring.from(Some(0)).err().ok_or("an expired cursor read")?;
    assert_eq!(refused.name(), "cursor_expired");
    assert!(matches!(
        refused,
        crate::RunnerError::Refused {
            oldest: Some(1),
            ..
        }
    ));
    let kept = ring.from(None)?;
    assert_eq!(kept.lines.len(), 2);
    assert_eq!(kept.cursor, 3);
    let ahead = ring
        .from(Some(4))
        .err()
        .ok_or("a cursor past the end read")?;
    assert_eq!(ahead.name(), "cursor_ahead");
    Ok(())
}
