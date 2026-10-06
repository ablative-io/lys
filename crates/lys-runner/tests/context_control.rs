#![cfg(test)]
//! Context actions reserve the boundary before queued normal input.

use lys_runner::harness_control::{
    Binding, BoundaryReply, ContextDecision, Controller, Executable, Kind, Pending, Transport,
};
use lys_runner::peer::{Leader, StartIdentity};
use serde_json::json;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_context_crossing_carries_its_identity_without_sending_the_threshold() -> TestResult {
    let wire = json!({"operation":"crossing", "session":"session",
        "request":{"request":"context_compact", "text":"/compact", "crossing":"crossing"}});
    let operation: lys_runner::operations::Operation = serde_json::from_value(wire.clone())?;
    assert_eq!(serde_json::to_value(operation)?, wire);
    let mut invalid = wire;
    invalid["request"]["threshold"] = json!(80);
    assert!(serde_json::from_value::<lys_runner::operations::Operation>(invalid).is_err());
    Ok(())
}

fn binding() -> Binding {
    Binding {
        session: "session".to_owned(),
        generation: 1,
        leader: Leader {
            pid: 42,
            start: StartIdentity("proved-start".to_owned()),
        },
        conversation: "conversation".to_owned(),
        entry: Executable {
            path: "lys".to_owned(),
            sha256: "entry-digest".to_owned(),
        },
        harness: Executable {
            path: "harness".to_owned(),
            sha256: "harness-digest".to_owned(),
        },
        harness_version: "9.8.7".to_owned(),
        adapter: "fixture-adapter/1".to_owned(),
    }
}

#[test]
fn a_compact_crossing_sends_one_request_after_the_active_turn_before_queued_normal_input()
-> TestResult {
    let source = binding();
    let mut control = Controller::new(source.clone(), Transport::Claude)?;
    control.ingest(
        &source,
        &json!({"type":"system","subtype":"init",
        "session_id":"conversation","claude_code_version":"9.8.7","slash_commands":["compact"]}),
    )?;
    let active = Pending::new("active".to_owned(), Kind::Human, "first message".to_owned());
    let uuid = active.uuid.clone();
    control.enqueue(active)?;
    control.ingest(
        &source,
        &json!({"type":"user","session_id":"conversation",
        "uuid":uuid,"parent_tool_use_id":null,"message":{"role":"user","content":"first message"}}),
    )?;
    assert!(
        control
            .enqueue(Pending::new(
                "normal".to_owned(),
                Kind::Human,
                "queued normal message".to_owned()
            ))?
            .dispatches
            .is_empty()
    );
    assert!(
        control
            .enqueue(Pending::new(
                "crossing".to_owned(),
                Kind::Compact,
                "/compact".to_owned()
            ))?
            .dispatches
            .is_empty()
    );
    let completed = control.ingest(
        &source,
        &json!({"type":"result",
        "session_id":"conversation","uuid":"completed-active","is_error":false}),
    )?;
    assert_eq!(completed.dispatches.len(), 1);
    assert_eq!(completed.dispatches[0].operation, "crossing");
    assert_eq!(
        completed.dispatches[0].frame.pointer("/message/content"),
        Some(&json!("/compact"))
    );
    Ok(())
}

fn controlled() -> Result<(Binding, Controller), Box<dyn Error>> {
    let source = binding();
    let mut controller = Controller::new(source.clone(), Transport::Claude)?;
    controller.require_boundary_authority()?;
    controller.ingest(
        &source,
        &json!({"type":"system","subtype":"init",
        "session_id":"conversation","claude_code_version":"9.8.7","slash_commands":["compact"]}),
    )?;
    Ok((source, controller))
}

fn reply(
    controller: &mut Controller,
    context: ContextDecision,
) -> Result<lys_runner::harness_control::Update, Box<dyn Error>> {
    let status = controller.control_status();
    let boundary = status.boundary.ok_or("no owned boundary")?;
    Ok(controller.boundary_reply(&BoundaryReply {
        generation: status.generation,
        boundary: Some(boundary),
        context,
        reminders: Vec::new(),
    })?)
}

#[test]
fn a_threshold_crossing_holds_a_queued_normal_message() -> TestResult {
    let (_, mut controller) = controlled()?;
    assert!(
        controller
            .enqueue(Pending::new(
                "normal".to_owned(),
                Kind::Human,
                "words".to_owned()
            ))?
            .dispatches
            .is_empty()
    );
    controller.context_compact(Pending::new(
        "crossing".to_owned(),
        Kind::Compact,
        "/compact".to_owned(),
    ))?;
    let update = reply(
        &mut controller,
        ContextDecision::Held {
            crossing: Some("crossing".to_owned()),
            reason: "context_above_limit".to_owned(),
        },
    )?;
    assert!(update.dispatches.is_empty());
    assert!(!controller.idle());
    Ok(())
}

#[test]
fn a_valid_current_service_reply_releases_the_held_next_message() -> TestResult {
    let (_, mut controller) = controlled()?;
    controller.enqueue(Pending::new(
        "normal".to_owned(),
        Kind::Human,
        "words".to_owned(),
    ))?;
    let update = reply(&mut controller, ContextDecision::Released)?;
    assert_eq!(update.dispatches.len(), 1);
    assert_eq!(update.dispatches[0].operation, "normal");
    assert!(controller.control_status().boundary.is_none());
    Ok(())
}

#[test]
fn an_above_threshold_service_decision_keeps_the_next_message_held() -> TestResult {
    let (_, mut controller) = controlled()?;
    controller.enqueue(Pending::new(
        "normal".to_owned(),
        Kind::Human,
        "words".to_owned(),
    ))?;
    let update = reply(
        &mut controller,
        ContextDecision::Held {
            crossing: None,
            reason: "context_above_limit".to_owned(),
        },
    )?;
    assert!(update.dispatches.is_empty());
    assert!(controller.control_status().boundary.is_some());
    Ok(())
}

#[test]
fn a_missing_measurement_service_decision_keeps_the_next_message_held() -> TestResult {
    let (_, mut controller) = controlled()?;
    controller.enqueue(Pending::new(
        "normal".to_owned(),
        Kind::Human,
        "words".to_owned(),
    ))?;
    let update = reply(
        &mut controller,
        ContextDecision::Held {
            crossing: None,
            reason: "context_measurement_unavailable".to_owned(),
        },
    )?;
    assert!(update.dispatches.is_empty());
    assert!(!controller.idle());
    Ok(())
}

#[test]
fn a_reply_for_another_generation_or_boundary_cannot_release_input() -> TestResult {
    let (_, mut controller) = controlled()?;
    controller.enqueue(Pending::new(
        "normal".to_owned(),
        Kind::Human,
        "words".to_owned(),
    ))?;
    let status = controller.control_status();
    let boundary = status.boundary.ok_or("no owned boundary")?;
    for (generation, named) in [
        (status.generation + 1, boundary),
        (status.generation, "another-boundary".to_owned()),
    ] {
        let result = controller.boundary_reply(&BoundaryReply {
            generation,
            boundary: Some(named),
            context: ContextDecision::Released,
            reminders: Vec::new(),
        });
        let error = result.err().ok_or("unowned boundary released input")?;
        assert_eq!(error.name(), "control_boundary_changed");
    }
    assert!(!controller.idle());
    Ok(())
}

#[test]
fn the_judged_agent_cannot_remove_its_own_context_hold() -> TestResult {
    use lys_core::Ed25519Identity;
    use lys_runner::protocol::{Greeting, sign_request};
    use lys_runner::{Act, Answer, Sessions};
    use std::sync::atomic::AtomicBool;

    let directory = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&directory.path().join("service.key"))?;
    let sessions = Sessions::open(&directory.path().join("state"), 4096)?;
    let greeting = Greeting::fresh("21");
    for caller in ["agent.judged", "person.responsible"] {
        let act = Act::AsCaller {
            caller: caller.to_owned(),
            done: Box::new(Act::Operate {
                operation: lys_runner::operations::Operation {
                    operation: "release".to_owned(),
                    session: "session".to_owned(),
                    request: lys_runner::operations::OperationRequest::BoundaryReply {
                        reply: BoundaryReply {
                            generation: 1,
                            boundary: Some("boundary".to_owned()),
                            context: ContextDecision::Released,
                            reminders: Vec::new(),
                        },
                    },
                },
            }),
        };
        let answer = lys_runner::socket::dispatch(
            &sessions,
            &key.public_key_bytes(),
            &greeting,
            &sign_request(&key, &greeting, &act)?,
            &AtomicBool::new(false),
        );
        assert!(
            matches!(&answer, Answer::Refused { refusal, .. }
            if refusal == "control_boundary_service_required"),
            "{answer:?}"
        );
    }
    Ok(())
}
