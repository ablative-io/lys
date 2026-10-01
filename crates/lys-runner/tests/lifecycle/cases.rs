#![cfg(test)]
//! Stream reads release the table, structured limits drive rotation, and ordinary stops flush spend.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::{Sessions, trip_on_words};
use crate::Launch;
use crate::peer::Collected;
use crate::rotation::{Limit, Rotation, RotationState};
use crate::tracking::{CLAUDE_ADAPTER, Harness, Tracking};
use crate::tracking_store::{Body, Commit, SourceState};

type TestResult = Result<(), Box<dyn Error>>;

fn tracked(
    dir: &std::path::Path,
    script: &str,
    transcript: &std::path::Path,
) -> Result<Arc<Sessions>, Box<dyn Error>> {
    let sessions = Sessions::open(&dir.join("state"), 4096)?;
    sessions.start(Launch {
        session: "session".to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), script.to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::from([("TRANSCRIPT".to_owned(), transcript.display().to_string())]),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    let mut table = sessions.lock();
    table
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .guard
        .tracking = Some(Tracking {
        harness: Harness::ClaudeCode,
        adapter: CLAUDE_ADAPTER.to_owned(),
        version: "2.1.285".to_owned(),
        config_home: dir.display().to_string(),
        context_window: 200_000,
        profile_version: 1,
        account: Some("account".to_owned()),
        requires_pre_tool: false,
    });
    table.feed.append(
        "session",
        1,
        Vec::new(),
        Commit {
            source: Some(SourceState {
                path: transcript.display().to_string(),
                bound: "native".to_owned(),
                ..SourceState::default()
            }),
            attempt: None,
        },
    )?;
    drop(table);
    sessions.until("session", &AtomicBool::new(false), |session, _| {
        session
            .scrollback()
            .from(0)
            .is_ok_and(|bytes| bytes.windows(5).any(|word| word == b"ready"))
            .then_some(Ok(()))
    })?;
    Ok(sessions)
}

#[test]
fn a_stream_wait_holds_no_session_table_lock() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream");
    nix::unistd::mkfifo(
        &path,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )?;
    let sessions = tracked(dir.path(), "printf 'ready\\n'; exec cat", &path)?;
    let reader = Arc::clone(&sessions);
    let joined = std::thread::spawn(move || reader.read_source("session", None));
    // Opening the writer signals that the reader reached the stream; it cannot finish until EOF.
    let writer = std::fs::OpenOptions::new().write(true).open(&path)?;
    let available = sessions.table.try_lock().is_ok();
    drop(writer);
    joined
        .join()
        .map_err(|panic| format!("stream reader panicked: {panic:?}"))?;
    std::fs::remove_file(&path)?;
    sessions.stop_all();
    assert!(available, "transcript I/O held the global session table");
    Ok(())
}

#[test]
fn quoted_limit_words_with_a_forty_percent_status_do_not_rotate_claude() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream.jsonl");
    std::fs::write(&path, b"")?;
    let sessions = tracked(dir.path(), "printf 'ready\\n'; exec cat", &path)?;
    sessions
        .lock()
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .rotation = Some(RotationState::new(Rotation {
        accounts: vec!["one".to_owned(), "two".to_owned()],
        variable: "ACCOUNT".to_owned(),
        limit: Limit::Words {
            words: vec!["usage limit reached".to_owned()],
        },
        resume_arguments: Vec::new(),
    })?);
    sessions.collect(
        "session",
        &Collected::StatusLine {
            input: serde_json::json!({
                "session_id": "native", "rate_limits": {"five_hour": {
                    "used_percentage": 40, "resets_at": crate::session::now_ms() / 1000 + 3600
                }}
            }),
        },
    )?;
    let tripped = {
        let mut table = sessions.lock();
        let session = table.sessions.get_mut("session").ok_or("session missing")?;
        session.scrollback.push(b"review says: usage limit reached");
        trip_on_words(session, 31);
        session
            .rotation
            .as_ref()
            .ok_or("rotation missing")?
            .tripped()
    };
    sessions.stop_all();
    assert!(
        !tripped,
        "quoted terminal words rotated a structured Claude session"
    );
    Ok(())
}

#[test]
fn an_ordinary_stop_counts_the_usage_written_on_term() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream.jsonl");
    std::fs::write(&path, b"")?;
    let record = serde_json::json!({"type":"assistant", "timestamp":"2026-10-01T01:00:00Z", "message": {
        "id":"final", "model":"model", "usage":{"input_tokens":10,"output_tokens":20,
            "cache_creation_input_tokens":0,"cache_read_input_tokens":0}
    }});
    let script = format!(
        "trap 'printf '\\''%s\\n'\\'' '\\''{record}'\\'' >> \"$TRANSCRIPT\"; exit 0' TERM; printf 'ready\\n'; while IFS= read -r line; do :; done"
    );
    let sessions = tracked(dir.path(), &script, &path)?;
    let ended = sessions.end("session", &AtomicBool::new(false))?;
    let entries = sessions.lock().feed.page(None)?.entries;
    sessions.stop_all();
    assert_eq!(ended.status, Some(0), "{ended:?}");
    assert!(
        entries
            .iter()
            .any(|entry| matches!(&entry.body, Body::Usage(usage)
        if usage.figures.output_tokens == Some(20))),
        "{entries:?}"
    );
    Ok(())
}

#[test]
fn a_full_live_status_window_trips_the_declared_rotation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream.jsonl");
    std::fs::write(&path, b"")?;
    let sessions = tracked(dir.path(), "printf 'ready\\n'; exec cat", &path)?;
    sessions
        .lock()
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .rotation = Some(RotationState::new(Rotation {
        accounts: vec!["one".to_owned(), "two".to_owned()],
        variable: "ACCOUNT".to_owned(),
        limit: Limit::PlanWindow,
        resume_arguments: Vec::new(),
    })?);
    sessions.collect(
        "session",
        &Collected::StatusLine {
            input: serde_json::json!({
                "session_id": "native", "rate_limits": {"five_hour": {
                    "used_percentage": 100, "resets_at": crate::session::now_ms() / 1000 + 3600
                }}
            }),
        },
    )?;
    let tripped = sessions
        .lock()
        .sessions
        .get("session")
        .ok_or("session missing")?
        .rotation
        .as_ref()
        .ok_or("rotation missing")?
        .tripped();
    sessions.stop_all();
    assert!(
        tripped,
        "the collector did not apply the declared account-window limit"
    );
    Ok(())
}
