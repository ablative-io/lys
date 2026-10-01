use std::collections::BTreeMap;
use std::error::Error;

use crate::peer::Collected;
use crate::tracking::{CLAUDE_ADAPTER, Harness, Tracking};
use crate::tracking_store::{Body, Commit, SourceState};
use crate::{Launch, Sessions};

#[test]
fn changed_status_ticks_keep_one_latest_snapshot_at_the_turn_boundary() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::tempdir()?;
    let sessions = tracked(dir.path())?;
    let contract = sessions
        .lock()
        .sessions
        .get("session")
        .ok_or("session missing")?
        .guard
        .tracking
        .clone()
        .ok_or("tracking missing")?;
    let reading = crate::tracking::Reading {
        runner: sessions.runner(),
        session: "session",
        tracking: &contract,
        accounts: crate::tracking::Accounts {
            current: None,
            moves: &[],
            declared: Some("account"),
        },
        now: crate::session::now_ms(),
    };
    let mut uncoalesced = SourceState::default();
    let mut expected_dollars = 0_u64;
    for tokens in 1..=100 {
        let input = serde_json::json!({
            "session_id":"native", "cost":{"total_cost_usd": if tokens > 50 { tokens - 50 } else { tokens }}, "context_window":{"current_usage":{"input_tokens":tokens,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}
        });
        let body = reading
            .status(&mut uncoalesced, &input, format!("tick:{tokens}"))
            .ok_or("status repeated")?;
        let Body::Usage(usage) = body else {
            return Err("status was not usage".into());
        };
        expected_dollars += usage.figures.dollars_micros.ok_or("cost absent")?;
        sessions.collect("session", &Collected::StatusLine { input })?;
    }
    let ticks = snapshots(&sessions)?;
    sessions.collect(
        "session",
        &Collected::Hook {
            event: "Stop".to_owned(),
            input: serde_json::json!({}),
        },
    )?;
    let boundary = snapshots(&sessions)?;
    sessions.stop_all();
    assert!(
        ticks.is_empty(),
        "status ticks appended {} audit records",
        ticks.len()
    );
    assert_eq!(boundary.len(), 1, "{boundary:?}");
    assert_eq!(boundary[0].figures.context_tokens, Some(100));
    assert_eq!(boundary[0].figures.dollars_micros, Some(expected_dollars));
    assert!(
        boundary[0]
            .unavailable
            .iter()
            .any(|note| note.reason == "reported_session_cost_reset")
    );
    Ok(())
}

fn snapshots(sessions: &Sessions) -> Result<Vec<crate::tracking::UsageRecord>, Box<dyn Error>> {
    Ok(sessions
        .lock()
        .feed
        .page(None)?
        .entries
        .into_iter()
        .filter_map(|entry| match entry.body {
            Body::Usage(usage) if usage.measure == crate::tracking::Measure::Snapshot => {
                Some(usage)
            }
            _ => None,
        })
        .collect())
}

fn tick(
    sessions: &std::sync::Arc<Sessions>,
    native: &str,
    dollars: u64,
) -> Result<String, crate::RunnerError> {
    sessions.collect(
        "session",
        &Collected::StatusLine {
            input: serde_json::json!({
                "session_id": native, "cost": {"total_cost_usd": dollars}
            }),
        },
    )
}

#[test]
fn overflow_is_named_and_leaves_the_prior_snapshot_available() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = tracked(dir.path())?;
    tick(&sessions, "native", 18_446_744_073_709)?;
    let result = tick(&sessions, "native", 1);
    sessions.stop_all();
    assert_eq!(
        result.err().ok_or("overflow accepted")?.name(),
        "status_cost_overflow"
    );
    let kept = snapshots(&sessions)?;
    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0].figures.dollars_micros,
        Some(18_446_744_073_709_000_000)
    );
    Ok(())
}

#[test]
fn account_and_source_changes_flush_before_new_attribution() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = tracked(dir.path())?;
    tick(&sessions, "native", 1)?;
    sessions
        .lock()
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .guard
        .tracking
        .as_mut()
        .ok_or("tracking missing")?
        .account = Some("next".to_owned());
    tick(&sessions, "native", 2)?;
    assert_eq!(snapshots(&sessions)?.len(), 1);
    let next = dir.path().join("next.jsonl");
    std::fs::write(&next, b"")?;
    sessions.bind(
        &mut sessions.lock(),
        "session",
        (&next.display().to_string(), "second"),
        true,
    )?;
    let before = snapshots(&sessions)?;
    tick(&sessions, "second", 3)?;
    sessions.stop_all();
    let kept = snapshots(&sessions)?;
    assert_eq!(before.len(), 2);
    assert_eq!(kept.len(), 3);
    assert_eq!(kept[0].account.as_deref(), Some("account"));
    assert_eq!(kept[1].account.as_deref(), Some("next"));
    assert_eq!(kept[2].account.as_deref(), Some("next"));
    assert_eq!(
        kept.iter()
            .map(|record| record.figures.dollars_micros.unwrap_or(0))
            .collect::<Vec<_>>(),
        vec![1_000_000, 1_000_000, 3_000_000]
    );
    assert_eq!(kept[2].generation, 1);
    Ok(())
}

#[test]
fn a_replaced_stream_flushes_the_old_generation_without_losing_its_cost_baseline()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = tracked(dir.path())?;
    sessions.read_source("session", None);
    tick(&sessions, "native", 1)?;
    let replacement = dir.path().join("replacement.jsonl");
    std::fs::write(&replacement, b"")?;
    std::fs::rename(&replacement, dir.path().join("transcript.jsonl"))?;
    sessions.read_source("session", None);
    let before = snapshots(&sessions)?;
    tick(&sessions, "native", 2)?;
    sessions.stop_all();
    let kept = snapshots(&sessions)?;
    assert_eq!(before.len(), 1);
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].generation, 0);
    assert_eq!(kept[1].generation, 1);
    assert_eq!(kept[0].figures.dollars_micros, Some(1_000_000));
    assert_eq!(kept[1].figures.dollars_micros, Some(1_000_000));
    Ok(())
}

fn tracked(directory: &std::path::Path) -> Result<std::sync::Arc<Sessions>, Box<dyn Error>> {
    let transcript = directory.join("transcript.jsonl");
    std::fs::write(&transcript, b"")?;
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
            config_home: directory.display().to_string(),
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
    }
    Ok(sessions)
}
