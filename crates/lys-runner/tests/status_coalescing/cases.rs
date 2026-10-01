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
    let transcript = dir.path().join("transcript.jsonl");
    std::fs::write(&transcript, b"")?;
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
            config_home: dir.path().display().to_string(),
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
