//! Passive hooks preserve terminal boundaries without claiming transcript coverage.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::operations::{Operation, OperationRequest, OperationState};
use crate::peer::Collected;
use crate::protocol::{Act, Answer, Greeting, sign_request};
use crate::tracking_store::Body;
use crate::{Launch, Sessions};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

struct Held {
    sessions: Arc<Sessions>,
    directory: tempfile::TempDir,
}

impl Held {
    fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let sessions = Sessions::open(&directory.path().join("state"), 4096)?;
        let launch: Launch = serde_json::from_value(json!({
            "session":"session", "program":"/bin/cat", "arguments":[],
            "directory":"/", "environment":{}, "columns":80, "rows":24
        }))?;
        sessions.start(launch)?;
        Ok(Self {
            sessions,
            directory,
        })
    }

    fn hook(&self, event: &str, input: Value) -> Result<String, crate::RunnerError> {
        self.sessions.collect(
            "session",
            &Collected::Hook {
                event: event.to_owned(),
                input,
            },
        )
    }

    fn idle(&self) -> Result<bool, Box<dyn Error>> {
        Ok(self
            .sessions
            .lock()?
            .sessions
            .get("session")
            .ok_or("session absent")?
            .guard
            .idle)
    }

    fn bodies(&self) -> Result<Vec<Body>, Box<dyn Error>> {
        Ok(self
            .sessions
            .lock()?
            .feed
            .page(None)?
            .entries
            .into_iter()
            .map(|entry| entry.body)
            .collect())
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = self.sessions.stop_all() {
            crate::error::said(&format!("collector_test_cleanup_failed: {error}"));
        }
    }
}

#[test]
fn an_old_start_line_without_tracking_still_starts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sessions = Sessions::open(&directory.path().join("state"), 4096)?;
    let key = lys_core::Ed25519Identity::load_or_generate(&directory.path().join("server.key"))?;
    let act: Act = serde_json::from_str(
        r#"{"act":"start","launch":{"session":"old","program":"/bin/cat","arguments":[],"directory":"/","environment":{},"columns":80,"rows":24}}"#,
    )?;
    let greeting = Greeting::fresh(sessions.runner());
    let line = sign_request(&key, &greeting, &act)?;
    let answer = crate::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &line,
        &AtomicBool::new(false),
    );
    sessions.stop_all()?;
    assert!(
        matches!(answer, Answer::Started { ref session, .. } if session == "old"),
        "{answer:?}"
    );
    Ok(())
}

#[test]
fn an_untracked_session_start_is_recorded_without_binding_a_transcript() -> TestResult {
    let held = Held::new()?;
    let receipt = held.hook("SessionStart", json!({"session_id":"native", "transcript_path":held.directory.path().join("unbound.jsonl")}))?;
    assert!(receipt.contains("hook_untracked"), "{receipt}");
    assert!(held.sessions.lock()?.feed.source("session").is_none());
    assert!(held.bodies()?.iter().any(|body| matches!(body, Body::Coverage(coverage) if coverage.state == "hook_untracked" && coverage.words.contains("SessionStart"))));
    Ok(())
}

#[test]
fn an_untracked_stop_sets_idle_and_names_the_limited_receipt() -> TestResult {
    let held = Held::new()?;
    assert!(
        held.hook("UserPromptSubmit", json!({"prompt":"private prompt"}))?
            .contains("hook_untracked")
    );
    assert!(!held.idle()?);
    let receipt = held.hook("Stop", json!({}))?;
    assert!(held.idle()?);
    assert!(receipt.contains("hook_untracked"), "{receipt}");
    let bodies = held.bodies()?;
    assert!(
        bodies.iter().any(
            |body| matches!(body, Body::Boundary(boundary) if boundary.boundary == "turn_end")
        )
    );
    assert!(!serde_json::to_string(&bodies)?.contains("private prompt"));
    Ok(())
}

#[test]
fn an_active_stop_hook_does_not_release_an_untracked_turn() -> TestResult {
    let held = Held::new()?;
    held.hook("UserPromptSubmit", json!({}))?;
    let before = held.bodies()?;
    held.hook("Stop", json!({"stop_hook_active":true}))?;
    assert!(!held.idle()?);
    assert_eq!(held.bodies()?, before);
    Ok(())
}

fn compact() -> Operation {
    Operation {
        operation: "compact".to_owned(),
        session: "session".to_owned(),
        request: OperationRequest::Compact {
            text: "/compact".to_owned(),
        },
    }
}

#[test]
fn post_compact_confirms_a_delivered_compaction_and_keeps_its_input() -> TestResult {
    let held = Held::new()?;
    assert_eq!(
        held.sessions.operate_admitted(compact())?.state,
        OperationState::Delivered
    );
    assert!(!held.idle()?);
    held.hook("PostCompact", json!({"trigger":"manual", "custom_instructions":"preserve the task", "prompt":"private prompt"}))?;
    assert_eq!(
        held.sessions.outcome("compact")?.state,
        OperationState::Confirmed
    );
    assert!(held.idle()?);
    let bodies = held.bodies()?;
    assert!(
        bodies.iter().any(
            |body| matches!(body, Body::Boundary(boundary) if boundary.boundary == "compacted")
        )
    );
    let input = bodies
        .iter()
        .find_map(|body| match body {
            Body::Coverage(coverage) if coverage.state == "compacted_input" => {
                Some(coverage.words.as_str())
            }
            _ => None,
        })
        .ok_or("compaction input absent")?;
    let input: Value = serde_json::from_str(input)?;
    assert_eq!(
        input,
        json!({"trigger":"manual", "custom_instructions":"preserve the task"})
    );
    assert!(!serde_json::to_string(&bodies)?.contains("private prompt"));
    Ok(())
}

#[test]
fn post_compact_without_optional_instructions_keeps_the_trigger() -> TestResult {
    let held = Held::new()?;
    held.hook("UserPromptSubmit", json!({}))?;
    held.hook("PostCompact", json!({"trigger":"auto"}))?;
    assert!(held.idle()?);
    let bodies = held.bodies()?;
    let input = bodies
        .iter()
        .find_map(|body| match body {
            Body::Coverage(coverage) if coverage.state == "compacted_input" => {
                Some(coverage.words.as_str())
            }
            _ => None,
        })
        .ok_or("compaction input absent")?;
    assert_eq!(
        serde_json::from_str::<Value>(input)?,
        json!({"trigger":"auto"})
    );
    Ok(())
}

#[test]
fn malformed_post_compact_input_is_refused_before_confirming() -> TestResult {
    for input in [
        json!({"trigger":42}),
        json!({"custom_instructions":false}),
        json!(null),
        json!([]),
    ] {
        let held = Held::new()?;
        held.sessions.operate_admitted(compact())?;
        let error = held
            .hook("PostCompact", input)
            .err()
            .ok_or("malformed hook accepted")?;
        assert_eq!(error.name(), "hook_malformed");
        assert_eq!(
            held.sessions.outcome("compact")?.state,
            OperationState::Delivered
        );
        assert!(!held.idle()?);
    }
    Ok(())
}

#[test]
fn post_compact_observes_a_managed_session_without_releasing_its_turn() -> TestResult {
    let held = Held::new()?;
    held.sessions.operate_admitted(compact())?;
    held.sessions
        .lock()?
        .sessions
        .get_mut("session")
        .ok_or("session absent")?
        .managed = Some(crate::harness_control::Settings {
        transport: crate::harness_control::Transport::Claude,
        conversation: "native".to_owned(),
        resume: false,
    });
    held.hook("PostCompact", json!({"trigger":"auto"}))?;
    assert!(!held.idle()?);
    assert_eq!(
        held.sessions.outcome("compact")?.state,
        OperationState::Delivered
    );
    assert!(
        held.bodies()?.iter().any(
            |body| matches!(body, Body::Boundary(boundary) if boundary.boundary == "compacted")
        )
    );
    Ok(())
}

fn bind_fixture(held: &Held) -> TestResult {
    let path = held.directory.path().join("transcript.jsonl");
    std::fs::write(&path, b"")?;
    let mut table = held.sessions.lock()?;
    table
        .sessions
        .get_mut("session")
        .ok_or("session absent")?
        .guard
        .tracking = Some(crate::tracking::Tracking {
        harness: crate::tracking::Harness::ClaudeCode,
        adapter: crate::tracking::CLAUDE_ADAPTER.to_owned(),
        version: "2.1.285".to_owned(),
        config_home: held.directory.path().display().to_string(),
        context_window: 200_000,
        profile_version: 1,
        account: None,
        requires_pre_tool: false,
    });
    table.feed.append(
        "session",
        1,
        Vec::new(),
        crate::tracking_store::Commit {
            source: Some(crate::tracking_store::SourceState {
                path: path.display().to_string(),
                bound: "native".to_owned(),
                ..crate::tracking_store::SourceState::default()
            }),
            attempt: None,
        },
    )?;
    Ok(())
}

#[test]
fn post_compact_reads_and_flushes_usage_before_the_compacted_boundary() -> TestResult {
    let held = Held::new()?;
    bind_fixture(&held)?;
    let path = held.directory.path().join("transcript.jsonl");
    std::fs::write(
        path,
        format!(
            "{}\n",
            json!({"type":"assistant", "timestamp":"2026-10-12T10:00:00Z", "message":{"id":"message", "model":"claude", "usage":{"input_tokens":7, "output_tokens":2}}})
        ),
    )?;
    held.hook("PostCompact", json!({"trigger":"auto"}))?;
    let bodies = held.bodies()?;
    let usage = bodies
        .iter()
        .position(
            |body| matches!(body, Body::Usage(usage) if usage.figures.input_tokens == Some(7)),
        )
        .ok_or("usage absent")?;
    let boundary = bodies
        .iter()
        .position(
            |body| matches!(body, Body::Boundary(boundary) if boundary.boundary == "compacted"),
        )
        .ok_or("boundary absent")?;
    assert!(usage < boundary);
    Ok(())
}

#[test]
fn post_compact_confirms_before_delivering_the_next_waiting_operation() -> TestResult {
    let held = Held::new()?;
    bind_fixture(&held)?;
    held.sessions.operate_admitted(compact())?;
    let notice = Operation {
        operation: "wake".to_owned(),
        session: "session".to_owned(),
        request: OperationRequest::Notice {
            text: "continue".to_owned(),
        },
    };
    assert_eq!(
        held.sessions.operate_admitted(notice.clone())?.state,
        OperationState::Accepted
    );
    held.hook("PostCompact", json!({"trigger":"auto"}))?;
    assert_eq!(
        held.sessions.operate_admitted(notice)?.state,
        OperationState::Delivered
    );
    assert!(!held.idle()?);
    let bodies = held.bodies()?;
    let confirmed = bodies.iter().position(|body| matches!(body, Body::Operation(outcome) if outcome.operation == "compact" && outcome.state == OperationState::Confirmed)).ok_or("confirmation absent")?;
    let next = bodies.iter().position(|body| matches!(body, Body::Operation(outcome) if outcome.operation == "wake" && outcome.state == OperationState::Delivering)).ok_or("next delivery absent")?;
    assert!(confirmed < next);
    Ok(())
}

#[test]
fn a_tracked_claude_hook_binds_only_the_named_config_home_transcript() -> TestResult {
    let held = Held::new()?;
    bind_fixture(&held)?;
    let transcript = held
        .directory
        .path()
        .join("projects")
        .join(super::slug("/"))
        .join("native.jsonl");
    std::fs::create_dir_all(transcript.parent().ok_or("transcript parent absent")?)?;
    std::fs::write(&transcript, b"")?;
    held.hook(
        "SessionStart",
        json!({"session_id":"native", "transcript_path":transcript}),
    )?;
    assert_eq!(
        held.sessions
            .lock()?
            .feed
            .source("session")
            .ok_or("source absent")?
            .path,
        transcript.display().to_string()
    );
    assert!(
        held.bodies()?.iter().any(
            |body| matches!(body, Body::Coverage(coverage) if coverage.state == "source_bound")
        )
    );
    let error = held.hook("SessionStart", json!({"session_id":"native", "transcript_path":held.directory.path().join("wrong.jsonl")})).err().ok_or("wrong transcript bound")?;
    assert_eq!(error.name(), "transcript_unbound");
    assert_eq!(
        held.sessions
            .lock()?
            .feed
            .source("session")
            .ok_or("source absent")?
            .path,
        transcript.display().to_string()
    );
    Ok(())
}
