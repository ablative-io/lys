#![cfg(test)]
//! A missing containing directory never becomes a root watch.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use crate::session::transcript_parent;
use crate::tracking_store::Body;
use crate::{Launch, Sessions};

#[test]
fn parentless_and_bare_relative_sources_are_refused() -> Result<(), Box<dyn Error>> {
    for path in ["", "/", "source.jsonl"] {
        assert_eq!(
            transcript_parent(Path::new(path))
                .err()
                .ok_or("parentless source accepted")?
                .name(),
            "transcript_parent_missing"
        );
    }
    assert_eq!(
        transcript_parent(Path::new("/logs/source.jsonl"))?,
        Path::new("/logs")
    );
    Ok(())
}

#[test]
fn parentless_binding_is_refused_and_audited_before_installing_a_follower()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
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
    let result = sessions.bind(
        &mut *sessions.lock()?,
        "session",
        ("source-without-parent.jsonl", "native"),
        true,
    );
    sessions.stop_all()?;
    assert_eq!(
        result.err().ok_or("parentless binding accepted")?.name(),
        "transcript_parent_missing"
    );
    let table = sessions.lock()?;
    assert!(table.feed.source("session").is_none());
    assert!(
        table
            .sessions
            .get("session")
            .ok_or("session missing")?
            .follower
            .is_none()
    );
    assert!(table.feed.page(None)?.entries.iter().any(|entry| matches!(&entry.body, Body::Coverage(coverage) if coverage.state == "source_refused")));
    Ok(())
}
