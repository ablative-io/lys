#![cfg(test)]
//! Pinned native wire fixtures. No model or app-server is invoked; these prove
//! correlation and refusal semantics, not live capability or enforcement.
use lys_runner::containment_policy::Binding;
use lys_runner::harness_control::codex::{Compaction, Request, lifecycle};
use lys_runner::harness_control::events::{Boundary, Kind, Source};
use lys_runner::peer::{Leader, StartIdentity};
use serde_json::json;

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

#[test]
fn reminders_are_data_without_permission_overrides_and_only_at_idle() -> TestResult {
    let words = "/compact\nquoted \"words\" and $(echo secret)";
    let request = Request::reminder("occurrence", "thread", &Boundary::Idle, words)?;
    assert_eq!(
        request.frame(),
        &json!({"id":"occurrence","method":"turn/start","params":{
            "threadId":"thread", "clientUserMessageId":"occurrence",
            "input":[{"type":"text","text":words,"text_elements":[]}]
        }})
    );
    for boundary in [
        Boundary::Unknown,
        Boundary::Active("tool-turn".into()),
        Boundary::Ended,
    ] {
        assert!(Request::reminder("occurrence", "thread", &boundary, words).is_err());
        assert!(Request::compact("operation", "thread", &boundary).is_err());
    }
    Ok(())
}

#[test]
fn empty_compact_reply_is_admission_only_and_requires_exact_correlation() -> TestResult {
    let request = Request::compact("compact", "thread", &Boundary::Idle)?;
    assert_eq!(
        request.frame(),
        &json!({"id":"compact","method":"thread/compact/start","params":{"threadId":"thread"}})
    );
    let event = request.admission(&source(), &json!({"id":"compact","result":{}}))?;
    assert_eq!(
        event.kind,
        Kind::Admitted {
            operation: "compact".into(),
            turn: None
        }
    );
    assert!(
        request
            .admission(&source(), &json!({"id":"other","result":{}}))
            .is_err()
    );
    assert!(
        request
            .admission(
                &source(),
                &json!({"id":"compact","error":{"message":"sensitive native text"}})
            )
            .is_err()
    );
    let mut foreign = source();
    foreign.conversation = "another".into();
    assert!(
        request
            .admission(&foreign, &json!({"id":"compact","result":{}}))
            .is_err()
    );
    Ok(())
}

fn compact_item(turn: &str) -> serde_json::Value {
    json!({"method":"item/completed","params":{"threadId":"thread","turnId":turn,
        "item":{"type":"contextCompaction","id":"compact-item"},"completedAtMs":10}})
}
fn terminal(status: &str) -> serde_json::Value {
    json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"turn","status":status,"items":[]}}})
}

#[test]
fn compaction_needs_both_native_item_and_successful_matching_terminal_turn() -> TestResult {
    for item_first in [true, false] {
        let mut proof = Compaction::new(source(), "op".into(), "turn".into())?;
        let (first, second) = if item_first {
            (compact_item("turn"), terminal("completed"))
        } else {
            (terminal("completed"), compact_item("turn"))
        };
        assert!(proof.observe(&first)?.is_none());
        let event = proof.observe(&second)?.ok_or("missing complete evidence")?;
        assert_eq!(
            event.kind,
            Kind::Compacted {
                operation: "op".into(),
                turn: "turn".into(),
                item: "compact-item".into()
            }
        );
        assert_eq!(
            proof.observe(&second)?,
            Some(event),
            "replay retains the same source identity"
        );
    }
    Ok(())
}

#[test]
fn failed_turn_or_wrong_thread_or_wrong_item_cannot_confirm_compaction() -> TestResult {
    let mut proof = Compaction::new(source(), "op".into(), "turn".into())?;
    assert!(proof.observe(&compact_item("foreign-turn")).is_err());
    let mut foreign = compact_item("turn");
    foreign["params"]["threadId"] = json!("foreign-thread");
    assert!(proof.observe(&foreign).is_err());
    let mut ordinary = compact_item("turn");
    ordinary["params"]["item"]["type"] = json!("agentMessage");
    assert!(proof.observe(&ordinary)?.is_none());
    assert!(proof.observe(&terminal("failed"))?.is_none());
    assert!(proof.observe(&compact_item("turn"))?.is_none());
    assert!(proof.observe(&terminal("completed")).is_err());
    Ok(())
}

#[test]
fn tool_completion_is_not_turn_completion_and_approval_is_never_fabricated() -> TestResult {
    let item = json!({"method":"item/completed","params":{"threadId":"thread","turnId":"turn",
        "item":{"type":"commandExecution","id":"tool","status":"completed"}}});
    assert!(lifecycle(&source(), &item)?.is_none());
    let terminal_event = lifecycle(&source(), &terminal("completed"))?.ok_or("missing terminal")?;
    assert_eq!(
        terminal_event.kind,
        Kind::TurnCompleted {
            turn: "turn".into()
        }
    );
    assert!(lifecycle(&source(), &terminal("inProgress")).is_err());
    let request = json!({"id":42,"method":"item/commandExecution/requestApproval","params":{"threadId":"thread"}});
    assert_eq!(
        lifecycle(&source(), &request)
            .expect_err("policy owner required")
            .name(),
        "control_policy_answer_required"
    );
    Ok(())
}

#[test]
fn admission_names_a_turn_without_claiming_completion_or_obedience() -> TestResult {
    let request = Request::reminder("op", "thread", &Boundary::Idle, "saved goal")?;
    let event = request.admission(
        &source(),
        &json!({"id":"op","result":{"turn":{"id":"turn","status":"inProgress"}}}),
    )?;
    assert_eq!(
        event.kind,
        Kind::Admitted {
            operation: "op".into(),
            turn: Some("turn".into())
        }
    );
    assert!(!serde_json::to_string(&event)?.contains("saved goal"));
    Ok(())
}
