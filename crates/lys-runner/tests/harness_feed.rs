//! Managed observations use the existing durable feed across reconnect/restart.
use std::error::Error;

use lys_runner::containment_policy::Binding;
use lys_runner::harness_control::events::{Event, Kind, Source};
use lys_runner::peer::{Leader, StartIdentity};
use lys_runner::tracking_store::{Body, Feed};

type Outcome = Result<(), Box<dyn Error>>;
fn source() -> Source {
    Source {
        binding: Binding {
            runner: "runner".into(),
            session: "session".into(),
            incarnation: "incarnation".into(),
            agent: "agent".into(),
            revision: 3,
            digest: "policy".into(),
        },
        leader: Leader {
            pid: 123,
            start: StartIdentity("start".into()),
        },
        generation: 1,
        conversation: "thread".into(),
    }
}
fn completed(source: &Source) -> Event {
    Event {
        source: source.clone(),
        source_id: "native-completion-1".into(),
        kind: Kind::TurnCompleted {
            turn: "turn-1".into(),
        },
    }
}

#[test]
fn replay_after_restart_reuses_sequence_and_cursor() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut feed = Feed::open(dir.path())?;
    let source = source();
    let event = completed(&source);
    let seq = feed.append_control(&source, 10, &event)?;
    let cursor = feed.end();
    assert_eq!(feed.append_control(&source, 11, &event)?, seq);
    assert_eq!(feed.end(), cursor);
    drop(feed);
    let mut feed = Feed::open(dir.path())?;
    assert_eq!(feed.append_control(&source, 12, &event)?, seq);
    assert_eq!(feed.end(), cursor);
    let page = feed.page(None)?;
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].body, Body::Control(event));
    assert!(feed.page(Some(&cursor))?.entries.is_empty());
    Ok(())
}

#[test]
fn stale_index_recovers_control_identity_from_committed_log() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut feed = Feed::open(dir.path())?;
    feed.append("session", 1, Vec::new(), Default::default())?;
    let index_path = dir.path().join("feed.index.json");
    let before = std::fs::read(&index_path)?;
    let source = source();
    let event = completed(&source);
    let seq = feed.append_control(&source, 10, &event)?;
    let cursor = feed.end();
    drop(feed);
    // The durable log unit survived but its index write did not.
    std::fs::write(index_path, before)?;
    let mut feed = Feed::open(dir.path())?;
    assert_eq!(feed.append_control(&source, 20, &event)?, seq);
    assert_eq!(feed.end(), cursor);
    assert_eq!(feed.page(None)?.entries.len(), 1);
    Ok(())
}

#[test]
fn foreign_generations_and_conflicting_replay_cannot_append() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut feed = Feed::open(dir.path())?;
    let source = source();
    let mut event = completed(&source);
    event.source.generation += 1;
    assert_eq!(
        feed.append_control(&source, 10, &event)
            .expect_err("foreign generation")
            .name(),
        "control_source_mismatch"
    );
    assert!(feed.page(None)?.entries.is_empty());
    event = completed(&source);
    feed.append_control(&source, 11, &event)?;
    let cursor = feed.end();
    event.kind = Kind::TurnCompleted {
        turn: "another-turn".into(),
    };
    assert_eq!(
        feed.append_control(&source, 12, &event)
            .expect_err("identity reused")
            .name(),
        "control_event_reused"
    );
    assert_eq!(feed.end(), cursor);
    let mut next_generation = source.clone();
    next_generation.generation += 1;
    next_generation.leader.start = StartIdentity("next-start".into());
    feed.append_control(&next_generation, 13, &completed(&next_generation))?;
    assert_eq!(feed.page(None)?.entries.len(), 2);
    Ok(())
}

#[test]
fn failed_append_does_not_claim_a_retained_observation() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut feed = Feed::open(dir.path())?;
    let source = source();
    let event = completed(&source);
    let log_path = dir.path().join("feed.jsonl");
    std::fs::create_dir(&log_path)?;
    assert!(feed.append_control(&source, 10, &event).is_err());
    std::fs::remove_dir(&log_path)?;
    let seq = feed.append_control(&source, 11, &event)?;
    assert_eq!(seq, 0);
    assert_eq!(feed.page(None)?.entries.len(), 1);
    Ok(())
}
