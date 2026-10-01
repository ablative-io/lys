//! Missing evidence is a coverage gap, and peer proof reads only its leader.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::error::Error;

use lys_runner::RunnerError;
use lys_runner::peer::{Leader, Processes, StartIdentity, prove_with};
use lys_runner::tracking::{Accounts, CLAUDE_ADAPTER, Harness, Reading, Tracking};
use lys_runner::tracking_store::{Body, SourceState};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn contract() -> Tracking {
    Tracking {
        harness: Harness::ClaudeCode,
        adapter: CLAUDE_ADAPTER.to_owned(),
        version: "2.1.285".to_owned(),
        config_home: "/isolated-home".to_owned(),
        context_window: 200_000,
        profile_version: 1,
        account: Some("current-account".to_owned()),
        requires_pre_tool: false,
    }
}

fn reading(tracking: &Tracking) -> Reading<'_> {
    Reading {
        runner: "runner",
        session: "session",
        tracking,
        accounts: Accounts {
            current: None,
            moves: &[],
            declared: Some("current-account"),
        },
        now: 1_800_000_000_000,
    }
}

fn assistant() -> Value {
    json!({"type": "assistant", "timestamp": "2026-10-01T01:00:00Z",
    "message": {"id": "response", "model": "model", "usage": {
        "input_tokens": 10, "output_tokens": 20,
        "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0
    }}})
}

fn gap(bodies: &[Body], name: &str, offset: u64) -> bool {
    bodies.iter().any(|body| {
        matches!(body, Body::Coverage(coverage)
        if coverage.state == name && coverage.offset == Some(offset))
    })
}

#[test]
fn anonymous_responses_are_named_gaps_instead_of_overwriting_spend() {
    let tracking = contract();
    let reader = reading(&tracking);
    let mut source = SourceState::default();
    let mut record = assistant();
    record["message"].as_object_mut().unwrap().remove("id");
    for offset in [12, 34] {
        let bodies = reader.claude(&mut source, offset, &record);
        assert!(gap(&bodies, "usage_unattributed", offset), "{bodies:?}");
        assert!(source.pending.is_none());
    }
}

#[test]
fn a_response_without_an_instant_never_uses_the_current_account() -> TestResult {
    let tracking = contract();
    let reader = reading(&tracking);
    let mut source = SourceState::default();
    let mut record = assistant();
    record
        .as_object_mut()
        .ok_or("record is not an object")?
        .remove("timestamp");
    let bodies = reader.claude(&mut source, 42, &record);
    assert!(gap(&bodies, "usage_instant_missing", 42), "{bodies:?}");
    let body = reader.flush(&mut source).ok_or("response was lost")?;
    let Body::Usage(usage) = body else {
        return Err("not usage".into());
    };
    assert_eq!(usage.account, None);
    assert_eq!(
        usage.account_unknown.as_deref(),
        Some("usage_instant_missing")
    );
    assert_eq!(usage.figures.output_tokens, Some(20));
    Ok(())
}

#[test]
fn an_unreadable_legacy_pending_figure_is_named_with_its_offset() -> TestResult {
    let tracking = contract();
    let reader = reading(&tracking);
    let mut source: SourceState = serde_json::from_value(json!({
        "path": "source", "generation": 0, "offset": 50, "identity": null,
        "bound": "thread", "turn": null, "totals": null, "snapshot": null,
        "pending": {"key": "message|request", "usage": "{broken", "model": null,
            "observed_at": 1_800_000_000_000_u64, "offset": 42, "turn": null}
    }))?;
    let bodies = reader.flush(&mut source).into_iter().collect::<Vec<_>>();
    assert!(gap(&bodies, "usage_unreadable", 42), "{bodies:?}");
    Ok(())
}

struct ProcessReads(Cell<usize>);

impl Processes for ProcessReads {
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        if pid == 999 {
            Ok(500)
        } else {
            Err(RunnerError::refused(
                "peer_unproved",
                "unexpected parent read",
            ))
        }
    }

    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        self.0.set(self.0.get() + 1);
        Ok(StartIdentity(format!("start-{pid}")))
    }
}

#[test]
fn peer_proof_reads_two_start_identities_regardless_of_session_count() -> TestResult {
    let leaders = (100..=500)
        .map(|pid| {
            (
                format!("session-{pid}"),
                Leader {
                    pid,
                    start: StartIdentity(format!("start-{pid}")),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let reads = ProcessReads(Cell::new(0));
    assert_eq!(prove_with(&reads, (999, 12, 12), &leaders)?, "session-500");
    assert_eq!(reads.0.get(), 2);
    Ok(())
}

#[test]
fn malformed_rollout_metadata_names_the_json_error() -> TestResult {
    let home = tempfile::tempdir()?;
    let date = home.path().join("sessions/2026/10/01");
    std::fs::create_dir_all(&date)?;
    std::fs::write(date.join("rollout-2026-10-01-thread.jsonl"), b"{broken\n")?;
    let error = lys_runner::collector::rollout(home.path(), "thread")
        .err()
        .ok_or("malformed rollout accepted")?;
    assert!(error.to_string().contains("rollout_unreadable"), "{error}");
    assert!(
        error.to_string().contains("first line is not JSON"),
        "{error}"
    );
    Ok(())
}

#[test]
fn unreadable_rollout_directories_are_not_reported_as_missing_threads() -> TestResult {
    let home = tempfile::tempdir()?;
    std::fs::write(home.path().join("sessions"), b"not a directory")?;
    let error = lys_runner::collector::rollout(home.path(), "thread")
        .err()
        .ok_or("unreadable directory accepted")?;
    assert!(error.to_string().contains("rollout_unreadable"), "{error}");
    Ok(())
}
