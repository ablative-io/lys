//! DIRECTORY-064 boundary projection: no absence, old generation or partial
//! result is a proved idle boundary. These are fixtures, not a live harness proof.

use lys_runner::containment_policy::Binding;
use lys_runner::harness_control::events::{Boundary, Event, Kind, Projection, Source};
use lys_runner::peer::{Leader, StartIdentity};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn source() -> Source {
    Source {
        binding: Binding {
            runner: "runner".into(),
            session: "session".into(),
            incarnation: "lifetime".into(),
            agent: "agent".into(),
            revision: 1,
            digest: "policy-digest".into(),
        },
        leader: Leader {
            pid: 17,
            start: StartIdentity("process-start".into()),
        },
        generation: 1,
        conversation: "thread".into(),
    }
}

fn event(kind: Kind) -> Event {
    Event {
        source: source(),
        source_id: match &kind {
            Kind::Rejected { rejection } => rejection.source_id().to_owned(),
            _ => "native-id".into(),
        },
        kind,
    }
}

#[test]
fn a_new_or_reconnected_projection_starts_unknown() -> TestResult {
    let mut projection = Projection::new(source())?;
    assert_eq!(projection.boundary(), &Boundary::Unknown);
    projection.apply(&event(Kind::IdleReconciled))?;
    assert_eq!(projection.boundary(), &Boundary::Idle);
    projection.apply(&event(Kind::Lost {
        reason: "pipe_closed".into(),
    }))?;
    assert_eq!(projection.boundary(), &Boundary::Unknown);
    Ok(())
}

#[test]
fn another_generation_cannot_open_a_boundary() -> TestResult {
    let mut projection = Projection::new(source())?;
    let mut wrong = event(Kind::IdleReconciled);
    wrong.source.generation = 2;
    assert_eq!(
        projection
            .apply(&wrong)
            .expect_err("foreign generation")
            .name(),
        "control_source_mismatch"
    );
    assert_eq!(projection.boundary(), &Boundary::Unknown);
    Ok(())
}

#[test]
fn only_the_matching_terminal_turn_opens_the_next_boundary() -> TestResult {
    let mut projection = Projection::new(source())?;
    projection.apply(&event(Kind::IdleReconciled))?;
    projection.apply(&event(Kind::TurnStarted { turn: "one".into() }))?;
    for kind in [
        Kind::Admitted {
            operation: "op".into(),
            turn: Some("one".into()),
        },
        Kind::Compacted {
            operation: "op".into(),
            turn: "one".into(),
            item: "item".into(),
        },
        Kind::Rejected {
            rejection: lys_runner::codex_refusals::rejection("thread", &serde_json::json!({"method":"item/completed","params":{"threadId":"thread","turnId":"one","item":{"id":"item","type":"commandExecution","status":"declined"}}}))?.ok_or("missing rejection")?,
        },
    ] {
        projection.apply(&event(kind))?;
        assert_eq!(projection.boundary(), &Boundary::Active("one".into()));
    }
    assert!(
        projection
            .apply(&event(Kind::TurnCompleted { turn: "old".into() }))
            .is_err()
    );
    assert!(projection.apply(&event(Kind::IdleReconciled)).is_err());
    assert_eq!(projection.boundary(), &Boundary::Active("one".into()));
    projection.apply(&event(Kind::TurnCompleted { turn: "one".into() }))?;
    assert_eq!(projection.boundary(), &Boundary::Idle);
    Ok(())
}

#[test]
fn a_feed_gap_requires_actual_reconciliation() -> TestResult {
    let mut projection = Projection::new(source())?;
    projection.apply(&event(Kind::IdleReconciled))?;
    assert_eq!(projection.gap().name(), "control_event_gap");
    assert_eq!(projection.boundary(), &Boundary::Unknown);
    assert!(
        projection
            .apply(&event(Kind::TurnCompleted { turn: "old".into() }))
            .is_err()
    );
    Ok(())
}

#[test]
fn an_exited_process_cannot_be_reopened_by_a_late_event() -> TestResult {
    let mut projection = Projection::new(source())?;
    projection.apply(&event(Kind::Exited))?;
    for kind in [
        Kind::IdleReconciled,
        Kind::Lost {
            reason: "late close".into(),
        },
        Kind::TurnStarted { turn: "new".into() },
    ] {
        assert_eq!(
            projection
                .apply(&event(kind))
                .expect_err("ended is final")
                .name(),
            "control_session_ended"
        );
        assert_eq!(projection.boundary(), &Boundary::Ended);
    }
    Ok(())
}

#[test]
fn durable_native_rejection_stays_bound_to_its_actual_thread() -> TestResult {
    let rejection = lys_runner::codex_refusals::rejection("thread", &serde_json::json!({"method":"item/completed","params":{"threadId":"thread","turnId":"one","item":{"id":"item","type":"commandExecution","status":"declined"}}}))?.ok_or("missing rejection")?;
    let original = event(Kind::Rejected { rejection });
    let restored: Event = serde_json::from_value(serde_json::to_value(&original)?)?;
    assert_eq!(original, restored);
    let mut projection = Projection::new(source())?;
    projection.apply(&restored)?;
    let mut wrong = restored;
    wrong.source_id = "another-event".into();
    assert_eq!(
        projection.apply(&wrong).expect_err("source changed").name(),
        "control_source_mismatch"
    );
    Ok(())
}
