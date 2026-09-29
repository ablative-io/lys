//! Real operation-file persistence; no process or harness is dispatched here.
use super::*;
use crate::containment_policy::Binding;
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
