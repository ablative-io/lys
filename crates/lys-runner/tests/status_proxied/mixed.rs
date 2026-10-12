#![cfg(test)]

//! Proxy sources retain their adapter when a terminal also has native hooks.

use std::error::Error;
use std::sync::Arc;

use crate::Sessions;
use crate::peer::Collected;
use crate::tracking::{Harness, Tracking};
use crate::tracking_proxy::PROXY_ADAPTER;
use crate::tracking_store::Body;
use serde_json::json;

struct Held {
    sessions: Arc<Sessions>,
    directory: tempfile::TempDir,
}

impl Held {
    fn new(harness: Option<Harness>) -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let (sessions, file) = super::proxied(directory.path())?;
        assert_eq!(file, directory.path().join(format!("{}.jsonl", super::RUN)));
        if let Some(harness) = harness {
            sessions
                .lock()?
                .sessions
                .get_mut("session")
                .ok_or("session absent")?
                .guard
                .tracking = Some(Tracking {
                harness,
                adapter: harness.adapter().to_owned(),
                version: String::new(),
                config_home: directory.path().display().to_string(),
                context_window: 0,
                profile_version: 0,
                account: None,
                requires_pre_tool: false,
            });
        }
        Ok(Self {
            sessions,
            directory,
        })
    }

    fn calls(&self, api: &str) -> Result<(), Box<dyn Error>> {
        let probe = json!({
            "call_id":"probe", "run":super::RUN, "session":"native",
            "api":api, "model":"model", "windows":[], "status":"unrecorded",
            "http":429, "started_at":"2026-10-12T20:15:00Z",
            "ended_at":"2026-10-12T20:15:01Z", "record":"native", "entry":"probe-entry"
        });
        let answered = json!({
            "call_id":"answer", "run":super::RUN, "session":"native",
            "api":api, "model":"model", "windows":[], "status":"complete", "http":200,
            "started_at":"2026-10-12T20:16:00Z", "ended_at":"2026-10-12T20:16:01Z",
            "record":"native", "entry":"answer-entry",
            "usage":{"input":10,"output":5,"cache_creation":0,"cache_read":0}
        });
        std::fs::write(
            self.directory.path().join(format!("{}.jsonl", super::RUN)),
            format!("{probe}\n{answered}\n"),
        )?;
        self.sessions.read_source("session", None);
        Ok(())
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = self.sessions.stop_all() {
            crate::error::said(&format!("proxy_test_cleanup_failed: {error}"));
        }
    }
}

#[test]
fn both_terminal_shapes_record_anthropic_and_codex_calls_once() -> Result<(), Box<dyn Error>> {
    for (api, harness) in [
        ("anthropic-messages", Harness::ClaudeCode),
        ("openai-responses", Harness::Codex),
    ] {
        for native in [None, Some(harness)] {
            let held = Held::new(native)?;
            held.calls(api)?;
            held.sessions.read_source("session", None);
            let records = super::usage(&held.sessions)?;
            assert_eq!(records.len(), 2, "{api} native={native:?}: {records:?}");
            assert_eq!(records[0].id, "probe");
            assert_eq!(records[0].figures.input_tokens, None);
            assert_eq!(
                records[0]
                    .record
                    .as_ref()
                    .ok_or("probe reference absent")?
                    .status,
                "unrecorded"
            );
            assert_eq!(records[1].id, "answer");
            assert_eq!(records[1].adapter, PROXY_ADAPTER);
            assert_eq!(records[1].figures.input_tokens, Some(10));
            assert_eq!(records[1].figures.output_tokens, Some(5));
            assert_eq!(
                records[1]
                    .record
                    .as_ref()
                    .ok_or("answer reference absent")?
                    .entry,
                "answer-entry"
            );
            assert!(
                held.sessions
                    .lock()?
                    .feed
                    .page(None)?
                    .entries
                    .iter()
                    .any(|entry| matches!(&entry.body, Body::Coverage(coverage)
                    if coverage.state == "usage_unreported" && coverage.words.contains("429")))
            );
        }
    }
    Ok(())
}

#[test]
fn native_hooks_do_not_compare_a_claude_status_id_to_a_proxy_run_key() -> Result<(), Box<dyn Error>>
{
    let held = Held::new(Some(Harness::ClaudeCode))?;
    super::tick(&held.sessions, 0.25, 4000)?;
    held.calls("anthropic-messages")?;
    let records = super::usage(&held.sessions)?;
    assert_eq!(records.len(), 3, "{records:?}");
    assert_eq!(records[0].figures.dollars_micros, Some(250_000));
    assert_eq!(records[0].figures.running_ms, Some(4000));
    assert_eq!(records[0].adapter, PROXY_ADAPTER);
    Ok(())
}

#[test]
fn a_proxied_codex_notification_keeps_the_usage_source_and_turn_end() -> Result<(), Box<dyn Error>>
{
    for native in [None, Some(Harness::Codex)] {
        let held = Held::new(native)?;
        held.calls("openai-responses")?;
        held.sessions.collect("session", &Collected::Notify {
            notification: json!({"type":"agent-turn-complete", "thread-id":"native", "turn-id":"turn"})
        })?;
        let table = held.sessions.lock()?;
        assert_eq!(
            table.feed.source("session").ok_or("source absent")?.path,
            held.directory
                .path()
                .join(format!("{}.jsonl", super::RUN))
                .display()
                .to_string()
        );
        assert!(
            table
                .feed
                .page(None)?
                .entries
                .iter()
                .any(|entry| matches!(&entry.body, Body::Boundary(boundary)
                if boundary.boundary == "turn_end" && boundary.turn.as_deref() == Some("turn")))
        );
        drop(table);
        assert_eq!(super::usage(&held.sessions)?.len(), 2);
    }
    Ok(())
}
