#![cfg(test)]
//! Native conversations survive old feeds and remain separate from proxy accounting.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::peer::Collected;
use crate::tracking_proxy::ProxyTracking;
use crate::tracking_store::{Commit, FEED_FORMAT, Feed, SourceState};
use crate::{Launch, Sessions};

const NATIVE: &str = "019bcd60-1a27-7000-8000-123456789abc";
const OTHER: &str = "019bcd60-1a27-7000-8000-abcdef123456";
const RUN: &str = "0123456789abcdef0123456789abcdef";

fn old_source(bound: &str) -> Value {
    json!({
        "path": "/work/stream.jsonl", "generation": 3, "offset": 91,
        "identity": "1:2", "bound": bound, "turn": "turn",
        "pending": null, "totals": {"input": 21, "cached": 4, "output": 8},
        "snapshot": null
    })
}

fn old_index(sources: Value) -> Value {
    let mut index = json!({
        "format": FEED_FORMAT, "feed": RUN, "committed": 0,
        "next_seq": 0, "attempts": {}
    });
    index["sources"] = sources;
    index
}

fn assert_old_source(source: &SourceState, bound: &str) {
    assert_eq!(source.native_conversation, None);
    assert_eq!(source.path, "/work/stream.jsonl");
    assert_eq!(source.generation, 3);
    assert_eq!(source.offset, 91);
    assert_eq!(source.identity.as_deref(), Some("1:2"));
    assert_eq!(source.bound, bound);
    assert_eq!(source.turn.as_deref(), Some("turn"));
    assert_eq!(source.totals.as_ref().map(|totals| totals.input), Some(21));
}

#[test]
fn an_old_index_keeps_native_identity_and_never_resumes_a_proxy_key_after_proxy_metadata_is_lost()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let index = old_index(json!({"native":old_source(NATIVE), "proxied":old_source(RUN)}));
    std::fs::write(
        directory.path().join("feed.index.json"),
        serde_json::to_vec(&index)?,
    )?;
    std::fs::write(directory.path().join("feed.jsonl"), b"")?;
    let feed = Feed::open(directory.path())?;
    let native = feed.source("native").ok_or("native source absent")?;
    assert_old_source(native, NATIVE);
    assert_eq!(
        crate::liveness::recorded_conversation(native, None).as_deref(),
        Some(NATIVE)
    );
    let proxied = feed.source("proxied").ok_or("proxy source absent")?;
    assert_old_source(proxied, RUN);
    let proxy = ProxyTracking {
        run: RUN.to_owned(),
        context_window: 0,
        profile_version: 1,
        account: None,
    };
    assert_eq!(
        crate::liveness::recorded_conversation(proxied, Some(&proxy)),
        None
    );
    assert!(feed.page(None)?.entries.is_empty());
    drop(feed);
    let reopened = Feed::open(directory.path())?;
    let proxied = reopened
        .source("proxied")
        .ok_or("reopened proxy source absent")?;
    assert_old_source(proxied, RUN);
    assert_eq!(crate::liveness::recorded_conversation(proxied, None), None);
    let native = reopened
        .source("native")
        .ok_or("reopened native source absent")?;
    assert_eq!(
        crate::liveness::recorded_conversation(native, None).as_deref(),
        Some(NATIVE)
    );
    Ok(())
}

#[test]
fn an_old_feed_commit_replays_without_a_native_field_and_keeps_its_cursor()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    std::fs::write(
        directory.path().join("feed.index.json"),
        serde_json::to_vec(&old_index(json!({})))?,
    )?;
    let commit = json!({
        "seq": 0, "at": 1, "session": "session",
        "body": {"kind":"commit", "entry":{"source":old_source(NATIVE)}}
    });
    std::fs::write(directory.path().join("feed.jsonl"), format!("{commit}\n"))?;
    let feed = Feed::open(directory.path())?;
    let kept = feed.source("session").ok_or("old source absent")?;
    assert_old_source(kept, NATIVE);
    assert_eq!(
        crate::liveness::recorded_conversation(kept, None).as_deref(),
        Some(NATIVE)
    );
    let end = feed.end();
    drop(feed);
    let reopened = Feed::open(directory.path())?;
    assert_old_source(
        reopened.source("session").ok_or("reopened source absent")?,
        NATIVE,
    );
    assert_eq!(reopened.end(), end);
    Ok(())
}

#[test]
fn a_codex_notification_refuses_a_non_uuid_before_binding_or_accounting()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let sessions = Sessions::open(directory.path(), 4096)?;
    let before = sessions.lock()?.feed.end();
    for thread in ["native", "", "../thread", RUN] {
        let error = sessions
            .collect(
                "session",
                &Collected::Notify {
                    notification: json!({"type":"agent-turn-complete", "thread-id":thread}),
                },
            )
            .err()
            .ok_or("non-UUID notification accepted")?;
        assert_eq!(error.name(), "notify_malformed");
        assert_eq!(sessions.lock()?.feed.end(), before);
        assert!(sessions.lock()?.feed.source("session").is_none());
    }
    Ok(())
}

struct Held {
    sessions: Arc<Sessions>,
    directory: tempfile::TempDir,
}

impl Held {
    fn proxied() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let sessions = Sessions::open(&directory.path().join("state"), 4096)?;
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
        let path = directory.path().join(format!("{RUN}.jsonl"));
        std::fs::write(&path, b"")?;
        let mut table = sessions.lock()?;
        table
            .sessions
            .get_mut("session")
            .ok_or("session absent")?
            .guard
            .proxy = Some(ProxyTracking {
            run: RUN.to_owned(),
            context_window: 0,
            profile_version: 1,
            account: None,
        });
        table.feed.append(
            "session",
            1,
            Vec::new(),
            Commit {
                source: Some(SourceState {
                    path: path.display().to_string(),
                    bound: RUN.to_owned(),
                    ..SourceState::default()
                }),
                attempt: None,
            },
        )?;
        drop(table);
        Ok(Self {
            sessions,
            directory,
        })
    }

    fn notify(&self, thread: &str) -> Result<String, crate::RunnerError> {
        self.sessions.collect(
            "session",
            &Collected::Notify {
                notification: json!({"type":"agent-turn-complete", "thread-id":thread}),
            },
        )
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = self.sessions.stop_all() {
            crate::error::said(&format!("native_collector_cleanup_failed: {error}"));
        }
    }
}

#[test]
fn a_changed_native_uuid_is_refused_without_rebinding_the_proxy_or_committing_a_turn()
-> Result<(), Box<dyn Error>> {
    let held = Held::proxied()?;
    held.notify(NATIVE)?;
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
        .notify(OTHER)
        .err()
        .ok_or("changed native UUID accepted")?;
    assert_eq!(error.name(), "native_conversation_changed");
    let table = held.sessions.lock()?;
    assert_eq!(table.feed.end(), before);
    assert_eq!(table.feed.source("session"), Some(&source));
    assert_eq!(source.native_conversation.as_deref(), Some(NATIVE));
    assert_eq!(source.bound, RUN);
    assert_eq!(
        source.path,
        held.directory
            .path()
            .join(format!("{RUN}.jsonl"))
            .display()
            .to_string()
    );
    Ok(())
}
