//! Real operation-file persistence; no process or harness is dispatched here.
use super::*;
use crate::containment_policy::Binding;
use crate::harness_control::events::Kind;
use crate::operations::OperationOutcome;
use crate::peer::{Leader, StartIdentity};

type Outcome = Result<(), Box<dyn std::error::Error>>;
fn source(session: &str) -> Source {
    Source {
        binding: Binding {
            runner: "runner".into(),
            session: session.into(),
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
fn accepted(dir: &std::path::Path) -> Result<Operations, RunnerError> {
    let mut operations = Operations::open(dir)?;
    operations.held.push(OperationOutcome {
        operation: "reminder-1".into(),
        session: "session".into(),
        request: "reminder".into(),
        state: OperationState::Accepted,
        at: 1,
        words: "accepted".into(),
        text: Some(TextDigest::of("private saved goal words")),
        ended: None,
        control: None,
    });
    operations.persist()?;
    Ok(operations)
}

#[test]
fn possible_write_is_durable_and_reopen_never_authorizes_resend() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    let source = source("session");
    let frame = b"private saved goal words";
    (&mut operations, &source).before_write("reminder-1", frame)?;
    let kept = operations.get("reminder-1").ok_or("missing operation")?;
    assert_eq!(kept.state, OperationState::Delivering);
    assert_eq!(kept.control.as_ref().ok_or("no metadata")?.source, source);
    assert_eq!(
        kept.control.as_ref().ok_or("no metadata")?.frame.length,
        frame.len()
    );
    let bytes = std::fs::read(dir.path().join("operations.json"))?;
    assert!(!String::from_utf8_lossy(&bytes).contains("private saved goal words"));
    drop(operations);
    let mut operations = Operations::open(dir.path())?;
    assert_eq!(
        operations.get("reminder-1").ok_or("missing")?.state,
        OperationState::Uncertain
    );
    assert_eq!(
        (&mut operations, &source)
            .before_write("reminder-1", frame)
            .expect_err("resend")
            .name(),
        "control_delivery_uncertain"
    );
    assert_eq!(operations.held.len(), 1);
    Ok(())
}

#[test]
fn failed_persistence_fences_the_operation_before_any_write() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    let source = source("session");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir(&blocked)?;
    operations.path = blocked;
    assert!(
        (&mut operations, &source)
            .before_write("reminder-1", b"frame")
            .is_err()
    );
    assert_eq!(
        operations.get("reminder-1").ok_or("missing")?.state,
        OperationState::Delivering
    );
    assert_eq!(
        (&mut operations, &source)
            .before_write("reminder-1", b"frame")
            .expect_err("retry after persistence failure")
            .name(),
        "control_delivery_uncertain"
    );
    // Nothing was sent; the prior durable Accepted record becomes Refused on restart.
    let reopened = Operations::open(dir.path())?;
    assert_eq!(
        reopened.get("reminder-1").ok_or("missing")?.state,
        OperationState::Refused
    );
    Ok(())
}

#[test]
fn another_session_cannot_journal_this_operation() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    assert_eq!(
        (&mut operations, &source("other"))
            .before_write("reminder-1", b"frame")
            .expect_err("foreign session")
            .name(),
        "control_source_mismatch"
    );
    let kept = operations.get("reminder-1").ok_or("missing")?;
    assert_eq!(kept.state, OperationState::Accepted);
    assert!(kept.control.is_none());
    Ok(())
}

fn event(kind: Kind) -> Event {
    Event {
        source: source("session"),
        source_id: format!("{kind:?}"),
        kind,
    }
}

fn admission() -> Event {
    event(Kind::Admitted {
        operation: "reminder-1".into(),
        turn: Some("turn-1".into()),
    })
}

#[test]
fn compaction_requires_admission_terminal_and_actual_compaction_across_restart() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    operations.held[0].request = "compact".into();
    (&mut operations, &source("session")).before_write("reminder-1", b"compact request")?;
    let mut feed = Feed::open(dir.path())?;
    let admitted = operations.keep_control(&mut feed, "reminder-1", &admission())?;
    assert_eq!(admitted.state, OperationState::Delivered);
    let terminal = event(Kind::TurnCompleted {
        turn: "turn-1".into(),
    });
    let finished = operations.keep_control(&mut feed, "reminder-1", &terminal)?;
    assert_eq!(
        finished.state,
        OperationState::Delivered,
        "ordinary completion is not compaction"
    );
    drop(operations);
    drop(feed);
    let mut operations = Operations::open(dir.path())?;
    let mut feed = Feed::open(dir.path())?;
    let compacted = event(Kind::Compacted {
        operation: "reminder-1".into(),
        turn: "turn-1".into(),
        item: "native-compact-item".into(),
    });
    let confirmed = operations.keep_control(&mut feed, "reminder-1", &compacted)?;
    assert_eq!(confirmed.state, OperationState::Confirmed);
    let cursor = feed.end();
    assert_eq!(
        operations.keep_control(&mut feed, "reminder-1", &compacted)?,
        confirmed
    );
    assert_eq!(feed.end(), cursor);
    assert_eq!(feed.page(None)?.entries.len(), 3);
    assert!(
        (&mut operations, &source("session"))
            .before_write("reminder-1", b"retry")
            .is_err()
    );
    assert_eq!(
        Operations::open(dir.path())?.get("reminder-1"),
        Some(&confirmed)
    );
    Ok(())
}

#[test]
fn reminder_admission_is_never_a_claim_of_goal_satisfaction() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    (&mut operations, &source("session")).before_write("reminder-1", b"reminder request")?;
    let mut feed = Feed::open(dir.path())?;
    operations.keep_control(&mut feed, "reminder-1", &admission())?;
    let terminal = event(Kind::TurnCompleted {
        turn: "turn-1".into(),
    });
    let receipt = operations.keep_control(&mut feed, "reminder-1", &terminal)?;
    assert_eq!(receipt.state, OperationState::Delivered);
    assert!(receipt.words.contains("goal satisfaction are not asserted"));
    let cursor = feed.end();
    let compacted = event(Kind::Compacted {
        operation: "reminder-1".into(),
        turn: "turn-1".into(),
        item: "unrelated-compact".into(),
    });
    assert!(
        operations
            .keep_control(&mut feed, "reminder-1", &compacted)
            .is_err()
    );
    assert_eq!(feed.end(), cursor);
    assert_eq!(operations.get("reminder-1"), Some(&receipt));
    Ok(())
}

#[test]
fn durable_event_reconciles_a_failed_receipt_write_without_repeating_delivery() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    (&mut operations, &source("session")).before_write("reminder-1", b"request")?;
    let before = operations.get("reminder-1").ok_or("missing")?.clone();
    let blocked = dir.path().join("blocked");
    std::fs::create_dir(&blocked)?;
    operations.path = blocked;
    let mut feed = Feed::open(dir.path())?;
    assert!(
        operations
            .keep_control(&mut feed, "reminder-1", &admission())
            .is_err()
    );
    assert_eq!(operations.get("reminder-1"), Some(&before));
    let cursor = feed.end();
    drop(operations);
    drop(feed);
    let mut operations = Operations::open(dir.path())?;
    assert_eq!(
        operations.get("reminder-1").ok_or("missing")?.state,
        OperationState::Uncertain
    );
    let mut feed = Feed::open(dir.path())?;
    let kept = feed.control_event(&source("session"), &admission().source_id)?;
    let receipt = operations.keep_control(&mut feed, "reminder-1", &kept)?;
    assert_eq!(receipt.state, OperationState::Delivered);
    assert_eq!(feed.end(), cursor);
    assert_eq!(feed.page(None)?.entries.len(), 1);
    assert!(
        (&mut operations, &source("session"))
            .before_write("reminder-1", b"retry")
            .is_err()
    );
    Ok(())
}

#[test]
fn unretained_foreign_or_conflicting_evidence_cannot_change_the_receipt() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    (&mut operations, &source("session")).before_write("reminder-1", b"request")?;
    let mut feed = Feed::open(dir.path())?;
    assert_eq!(
        feed.control_event(&source("session"), "absent")
            .expect_err("gap")
            .name(),
        "control_event_gap"
    );
    let before = operations.get("reminder-1").cloned();
    let cursor = feed.end();
    let mut foreign = admission();
    foreign.source.generation += 1;
    let other_operation = event(Kind::Admitted {
        operation: "other".into(),
        turn: Some("turn-1".into()),
    });
    let unbound_terminal = event(Kind::TurnCompleted {
        turn: "turn-1".into(),
    });
    let mut rejected = 0;
    for event in [foreign, other_operation, unbound_terminal] {
        assert!(
            operations
                .keep_control(&mut feed, "reminder-1", &event)
                .is_err()
        );
        rejected += 1;
    }
    assert_eq!(rejected, 3);
    assert_eq!(feed.end(), cursor);
    assert_eq!(operations.get("reminder-1"), before.as_ref());
    operations.keep_control(&mut feed, "reminder-1", &admission())?;
    let before = operations.get("reminder-1").cloned();
    let mut conflict = event(Kind::TurnCompleted {
        turn: "turn-1".into(),
    });
    conflict.source_id = admission().source_id;
    assert_eq!(
        operations
            .keep_control(&mut feed, "reminder-1", &conflict)
            .expect_err("reused id")
            .name(),
        "control_event_reused"
    );
    assert_eq!(operations.get("reminder-1"), before.as_ref());
    Ok(())
}

#[test]
fn legacy_hook_cannot_confirm_a_managed_compaction() -> Outcome {
    let dir = tempfile::tempdir()?;
    let mut operations = accepted(dir.path())?;
    operations.held[0].request = "compact".into();
    (&mut operations, &source("session")).before_write("reminder-1", b"compact request")?;
    let mut feed = Feed::open(dir.path())?;
    let before = operations.keep_control(&mut feed, "reminder-1", &admission())?;
    drop(feed);
    drop(operations);
    let sessions = crate::session::Sessions::open(dir.path(), 1024)?;
    super::super::compacting(&mut sessions.lock(), "session");
    assert_eq!(sessions.outcome("reminder-1")?, before);
    assert_eq!(
        sessions
            .reconcile_control("reminder-1", "not-retained")
            .expect_err("gap")
            .name(),
        "control_event_gap"
    );
    assert_eq!(
        sessions.reconcile_control("reminder-1", &admission().source_id)?,
        before
    );
    assert_eq!(
        sessions.outcome("reminder-1")?.state,
        OperationState::Delivered
    );
    Ok(())
}
