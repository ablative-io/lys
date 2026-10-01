#![cfg(test)]
//! A failed transcript stat never binds earlier records to a new session.

use std::collections::BTreeMap;
use std::error::Error;

use crate::peer::Collected;
use crate::tracking::{CLAUDE_ADAPTER, Harness, Tracking};
use crate::tracking_store::Body;
use crate::{Launch, Sessions};

#[test]
fn an_unreadable_transcript_binding_is_refused_and_audited() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let home = dir.path().join("home");
    let sessions = Sessions::open(&dir.path().join("state"), 4096)?;
    sessions.start(Launch {
        session: "session".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    sessions
        .lock()
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .guard
        .tracking = Some(Tracking {
        harness: Harness::ClaudeCode,
        adapter: CLAUDE_ADAPTER.to_owned(),
        version: "2.1.285".to_owned(),
        config_home: home.display().to_string(),
        context_window: 200_000,
        profile_version: 1,
        account: None,
        requires_pre_tool: false,
    });
    let path = home.join("projects/-/native.jsonl");
    let result = sessions.collect(
        "session",
        &Collected::Hook {
            event: "SessionStart".to_owned(),
            input: serde_json::json!({"session_id":"native", "transcript_path":path}),
        },
    );
    sessions.stop_all();
    let error = result
        .err()
        .ok_or("missing transcript was bound from zero")?;
    assert!(
        error.to_string().contains("transcript_unreadable"),
        "{error}"
    );
    let table = sessions.lock();
    assert!(table.feed.source("session").is_none());
    assert!(table.feed.page(None)?.entries.iter().any(|entry|
        matches!(&entry.body, Body::Coverage(coverage) if coverage.state == "source_refused")));
    Ok(())
}
