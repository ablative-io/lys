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
fn a_proxied_codex_notification_names_a_native_uuid_and_keeps_the_usage_source_and_turn_end()
-> Result<(), Box<dyn Error>> {
    let conversation = "019bcd60-1a27-7000-8000-123456789abc";
    for native in [None, Some(Harness::Codex)] {
        let held = Held::new(native)?;
        held.calls("openai-responses")?;
        let mut expected = held
            .sessions
            .lock()?
            .feed
            .source("session")
            .cloned()
            .ok_or("source absent")?;
        expected.native_conversation = Some(conversation.to_owned());
        held.sessions.collect("session", &Collected::Notify {
            notification: json!({"type":"agent-turn-complete", "thread-id":conversation, "turn-id":"turn"})
        })?;
        let table = held.sessions.lock()?;
        assert_eq!(table.feed.source("session"), Some(&expected));
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
        assert_eq!(
            held.sessions.liveness(Some("session"))?[0]
                .harness_session
                .as_deref(),
            Some(conversation)
        );
        held.sessions.stop_all()?;
        let reopened = crate::tracking_store::Feed::open(&held.directory.path().join("state"))?;
        assert_eq!(reopened.source("session"), Some(&expected));
    }
    Ok(())
}

#[test]
fn a_claude_proxy_session_start_keeps_native_identity_with_one_boundary_and_preserves_accounting()
-> Result<(), Box<dyn Error>> {
    use std::io::Write;
    let conversation = "019bcd60-1a27-7000-8000-123456789abc";
    for native in [None, Some(Harness::ClaudeCode)] {
        let held = Held::new(native)?;
        held.calls("anthropic-messages")?;
        let (before, mut expected) = {
            let table = held.sessions.lock()?;
            (
                table.feed.end(),
                table
                    .feed
                    .source("session")
                    .cloned()
                    .ok_or("source absent")?,
            )
        };
        expected.native_conversation = Some(conversation.to_owned());
        held.sessions.collect("session", &Collected::Hook {
            event: "SessionStart".to_owned(),
            input: json!({"session_id":conversation, "transcript_path":"/no-native-transcript-is-read.jsonl"}),
        })?;
        {
            let table = held.sessions.lock()?;
            assert_eq!(table.feed.source("session"), Some(&expected));
            let captured = table.feed.page(Some(&before))?;
            assert_eq!(captured.entries.len(), 1);
            assert!(matches!(&captured.entries[0].body,
                Body::Boundary(boundary) if boundary.boundary == "session_start"));
        }
        assert_eq!(
            held.sessions.liveness(Some("session"))?[0]
                .harness_session
                .as_deref(),
            Some(conversation)
        );
        let call = json!({
            "call_id":"after-native", "run":super::RUN, "session":conversation,
            "api":"anthropic-messages", "model":"model", "windows":[],
            "status":"complete", "http":200, "started_at":"2026-10-12T20:17:00Z",
            "ended_at":"2026-10-12T20:17:01Z", "record":conversation,
            "entry":"after-native-entry", "usage":{"input":7,"output":2,"cache_creation":0,"cache_read":0}
        });
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(held.directory.path().join(format!("{}.jsonl", super::RUN)))?;
        writeln!(file, "{call}")?;
        drop(file);
        held.sessions.read_source("session", None);
        held.sessions.read_source("session", None);
        let records = super::usage(&held.sessions)?;
        assert_eq!(records.len(), 3);
        assert_eq!(records[2].id, "after-native");
        assert_eq!(records[2].adapter, PROXY_ADAPTER);
        assert_eq!(records[2].figures.input_tokens, Some(7));
        assert_eq!(records[2].figures.output_tokens, Some(2));
        let committed = held
            .sessions
            .lock()?
            .feed
            .source("session")
            .cloned()
            .ok_or("source absent")?;
        assert_eq!(committed.native_conversation.as_deref(), Some(conversation));
        held.sessions.stop_all()?;
        let reopened = crate::tracking_store::Feed::open(&held.directory.path().join("state"))?;
        assert_eq!(reopened.source("session"), Some(&committed));
    }
    Ok(())
}

#[test]
fn invalid_missing_or_changed_claude_proxy_ids_refuse_before_any_feed_append()
-> Result<(), Box<dyn Error>> {
    let conversation = "019bcd60-1a27-7000-8000-123456789abc";
    let other = "019bcd60-1a27-7000-8000-abcdef123456";
    for native in [None, Some(Harness::ClaudeCode)] {
        let held = Held::new(native)?;
        held.sessions.collect(
            "session",
            &Collected::Hook {
                event: "SessionStart".to_owned(),
                input: json!({"session_id":conversation}),
            },
        )?;
        let (before, source) = {
            let table = held.sessions.lock()?;
            (
                table.feed.end(),
                table
                    .feed
                    .source("session")
                    .cloned()
                    .ok_or("source absent")?,
            )
        };
        for (input, refusal) in [
            (json!({}), "hook_malformed"),
            (json!({"session_id":null}), "hook_malformed"),
            (json!({"session_id":""}), "hook_malformed"),
            (json!({"session_id":super::RUN}), "hook_malformed"),
            (json!({"session_id":"../thread"}), "hook_malformed"),
            (json!({"session_id":other}), "native_conversation_changed"),
        ] {
            let error = held
                .sessions
                .collect(
                    "session",
                    &Collected::Hook {
                        event: "SessionStart".to_owned(),
                        input,
                    },
                )
                .err()
                .ok_or("invalid native identity accepted")?;
            assert_eq!(error.name(), refusal, "{error}");
            let table = held.sessions.lock()?;
            assert_eq!(table.feed.end(), before);
            assert_eq!(table.feed.source("session"), Some(&source));
        }
    }
    Ok(())
}

#[test]
fn a_claude_proxy_hook_must_match_the_native_uuid_declared_by_fresh_or_resumed_launch_arguments()
-> Result<(), Box<dyn Error>> {
    let conversation = "019bcd60-1a27-7000-8000-123456789abc";
    let other = "019bcd60-1a27-7000-8000-abcdef123456";
    for flag in ["--session-id", "--resume", "-r"] {
        let held = Held::new(None)?;
        held.sessions
            .lock()?
            .sessions
            .get_mut("session")
            .ok_or("session absent")?
            .launch
            .as_mut()
            .ok_or("launch absent")?
            .arguments = vec![flag.to_owned(), conversation.to_owned()];
        let (before, source) = {
            let table = held.sessions.lock()?;
            (
                table.feed.end(),
                table
                    .feed
                    .source("session")
                    .cloned()
                    .ok_or("source absent")?,
            )
        };
        let error = held
            .sessions
            .collect(
                "session",
                &Collected::Hook {
                    event: "SessionStart".to_owned(),
                    input: json!({"session_id":other}),
                },
            )
            .err()
            .ok_or("a different declared UUID was accepted")?;
        assert_eq!(error.name(), "native_conversation_changed", "{error}");
        assert_eq!(held.sessions.lock()?.feed.end(), before);
        assert_eq!(held.sessions.lock()?.feed.source("session"), Some(&source));
        held.sessions.collect(
            "session",
            &Collected::Hook {
                event: "SessionStart".to_owned(),
                input: json!({"session_id":conversation}),
            },
        )?;
    }
    Ok(())
}

#[test]
fn a_managed_claude_proxy_hook_keeps_its_reviewed_binding_and_does_not_release_its_turn()
-> Result<(), Box<dyn Error>> {
    let conversation = "019bcd60-1a27-7000-8000-123456789abc";
    let held = Held::new(Some(Harness::ClaudeCode))?;
    {
        let mut table = held.sessions.lock()?;
        let session = table.sessions.get_mut("session").ok_or("session absent")?;
        session.managed = Some(crate::harness_control::Settings {
            transport: crate::harness_control::Transport::Claude,
            conversation: conversation.to_owned(),
            resume: false,
        });
        session.guard.idle = false;
    }
    let before = held.sessions.lock()?.feed.end();
    let error = held
        .sessions
        .collect(
            "session",
            &Collected::Hook {
                event: "SessionStart".to_owned(),
                input: json!({"session_id":"019bcd60-1a27-7000-8000-abcdef123456"}),
            },
        )
        .err()
        .ok_or("another managed conversation was accepted")?;
    assert_eq!(error.name(), "native_conversation_changed", "{error}");
    assert_eq!(held.sessions.lock()?.feed.end(), before);
    held.sessions.collect(
        "session",
        &Collected::Hook {
            event: "SessionStart".to_owned(),
            input: json!({"session_id":conversation}),
        },
    )?;
    let table = held.sessions.lock()?;
    let session = table.sessions.get("session").ok_or("session absent")?;
    assert!(!session.guard.idle);
    let managed = session.managed.as_ref().ok_or("managed binding absent")?;
    assert_eq!(managed.conversation, conversation);
    assert_eq!(managed.transport, crate::harness_control::Transport::Claude);
    assert!(!managed.resume);
    Ok(())
}
