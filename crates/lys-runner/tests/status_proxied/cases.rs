#![cfg(test)]

//! A run counted through the proxy: what its status line reports is held,
//! then written before the next calls read from its usage file.

use std::collections::BTreeMap;
use std::error::Error;

use crate::peer::Collected;
use crate::tracking::{Measure, UsageRecord};
use crate::tracking_proxy::{PROXY_ADAPTER, ProxyTracking};
use crate::tracking_store::{Body, Commit, SourceState};
use crate::{Launch, Sessions};

const RUN: &str = "0123456789abcdef0123456789abcdef";

fn usage(sessions: &Sessions) -> Result<Vec<UsageRecord>, Box<dyn Error>> {
    Ok(sessions
        .lock()?
        .feed
        .page(None)?
        .entries
        .into_iter()
        .filter_map(|entry| match entry.body {
            Body::Usage(usage) => Some(usage),
            _ => None,
        })
        .collect())
}

fn tick(
    sessions: &std::sync::Arc<Sessions>,
    dollars: f64,
    running_ms: u64,
) -> Result<String, crate::RunnerError> {
    sessions.collect(
        "session",
        &Collected::StatusLine {
            input: serde_json::json!({
                "session_id": "the harness's own id, which the proxy never learns",
                "cost": {"total_cost_usd": dollars, "total_duration_ms": running_ms}
            }),
        },
    )
}

/// A session whose calls go through the proxy, its usage file bound and
/// empty, and that file's path.
fn proxied(
    directory: &std::path::Path,
) -> Result<(std::sync::Arc<Sessions>, std::path::PathBuf), Box<dyn Error>> {
    let file = directory.join(format!("{RUN}.jsonl"));
    std::fs::write(&file, b"")?;
    let sessions = Sessions::open(&directory.join("state"), 4096)?;
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
    {
        let mut table = sessions.lock()?;
        table
            .sessions
            .get_mut("session")
            .ok_or("session missing")?
            .guard
            .proxy = Some(ProxyTracking {
            run: RUN.to_owned(),
            context_window: 0,
            profile_version: 1,
            account: Some("account".to_owned()),
        });
        table.feed.append(
            "session",
            1,
            Vec::new(),
            Commit {
                source: Some(SourceState {
                    path: file.display().to_string(),
                    bound: RUN.to_owned(),
                    ..SourceState::default()
                }),
                attempt: None,
            },
        )?;
    }
    Ok((sessions, file))
}

#[test]
fn a_proxied_runs_status_reports_are_held_and_written_before_its_next_calls()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let (sessions, file) = proxied(directory.path())?;
    // Two reports: both are held, neither is on the record yet, and the
    // session the status line names is not asked to match the run key.
    tick(&sessions, 0.25, 4000)?;
    tick(&sessions, 0.40, 9000)?;
    assert!(usage(&sessions)?.is_empty());
    // A report that repeats the last one held changes nothing.
    assert!(tick(&sessions, 0.40, 9000)?.contains("repeat"));
    // The run's next call lands in its usage file and is read.
    let call = serde_json::json!({
        "call_id": "c", "run": RUN, "session": "harness", "api": "anthropic-messages",
        "model": "m", "windows": [], "started_at": "2000-01-01T00:00:00Z",
        "ended_at": "2000-01-01T00:00:02Z", "status": "complete", "record": "harness",
        "entry": "e", "usage": {"input": 10, "output": 5, "cache_creation": 0, "cache_read": 0}
    });
    std::fs::write(&file, format!("{call}\n"))?;
    sessions.read_source("session", None);
    let records = usage(&sessions)?;
    // The held report is one snapshot, written before the call read after
    // it: the dollars both reports added up to, the latest running time.
    let [status, spend] = records.as_slice() else {
        return Err(format!("one snapshot then one call expected: {records:?}").into());
    };
    assert_eq!(status.measure, Measure::Snapshot);
    assert_eq!(status.figures.dollars_micros, Some(400_000));
    assert_eq!(status.figures.running_ms, Some(9000));
    assert_eq!(status.figures.input_tokens, None);
    assert_eq!(status.adapter, PROXY_ADAPTER);
    assert_eq!(status.run.as_deref(), Some(RUN));
    assert_eq!(status.account.as_deref(), Some("account"));
    assert_eq!(spend.measure, Measure::Spend);
    assert_eq!(spend.figures.input_tokens, Some(10));
    assert_eq!(spend.figures.dollars_micros, None);
    // A later report adds only what was added since the one written.
    tick(&sessions, 0.55, 12_000)?;
    crate::collector::status::flush_status(&mut sessions.lock()?, "session")?;
    let records = usage(&sessions)?;
    let last = records.last().ok_or("a third record")?;
    assert_eq!(last.figures.dollars_micros, Some(150_000));
    assert_eq!(last.figures.running_ms, Some(12_000));
    Ok(())
}

#[test]
fn a_session_tracked_neither_way_keeps_nothing_of_a_status_line() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let (sessions, _) = proxied(directory.path())?;
    sessions
        .lock()?
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .guard
        .proxy = None;
    assert!(tick(&sessions, 0.25, 4000)?.contains("not tracked"));
    assert!(usage(&sessions)?.is_empty());
    Ok(())
}
