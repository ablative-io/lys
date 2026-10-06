#![cfg(test)]
//! A saved occurrence remains data and needs current service authority.

use std::error::Error;

use lys_runner::harness_control::{
    Binding, BoundaryReply, ContextDecision, Controller, Executable, Kind, Pending,
    ReminderDecision, ReminderReference, Transport, Update,
};
use lys_runner::operations::OperationState;
use lys_runner::peer::{Leader, StartIdentity};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn controlled() -> Result<(Binding, Controller), Box<dyn Error>> {
    let binding = Binding {
        session: "session".to_owned(),
        generation: 1,
        leader: Leader {
            pid: 42,
            start: StartIdentity("proved-start".to_owned()),
        },
        conversation: "conversation".to_owned(),
        entry: Executable {
            path: "entry".to_owned(),
            sha256: "entry-digest".to_owned(),
        },
        harness: Executable {
            path: "harness".to_owned(),
            sha256: "harness-digest".to_owned(),
        },
        harness_version: "9.8.7".to_owned(),
        adapter: "fixture-adapter/1".to_owned(),
    };
    let mut controller = Controller::new(binding.clone(), Transport::Claude)?;
    controller.require_boundary_authority()?;
    controller.ingest(
        &binding,
        &json!({"type":"system","subtype":"init",
        "session_id":"conversation","claude_code_version":"9.8.7","slash_commands":["compact"]}),
    )?;
    Ok((binding, controller))
}

fn reference(version: &str) -> ReminderReference {
    ReminderReference {
        goal: "goal".to_owned(),
        occurrence: "occurrence".to_owned(),
        version: version.to_owned(),
        prior: None,
    }
}

fn deliver(text: &str, version: &str) -> ReminderDecision {
    ReminderDecision::Deliver {
        operation: "delivery".to_owned(),
        reference: reference(version),
        text: text.to_owned(),
    }
}

fn answer(
    controller: &mut Controller,
    context: ContextDecision,
    reminders: Vec<ReminderDecision>,
) -> Result<Update, Box<dyn Error>> {
    let status = controller.control_status();
    Ok(controller.boundary_reply(&BoundaryReply {
        generation: status.generation,
        boundary: status.boundary,
        context,
        reminders,
    })?)
}

fn queue(controller: &mut Controller, text: &str) -> TestResult {
    assert!(
        controller
            .enqueue(Pending::for_goal(
                "delivery".to_owned(),
                text.to_owned(),
                reference("original")
            ))?
            .dispatches
            .is_empty()
    );
    Ok(())
}

#[test]
fn the_admitted_reminder_preserves_the_saved_goal_words_as_json_data() -> TestResult {
    let (_, mut controller) = controlled()?;
    let text = "A \"quoted\" goal\nwith a backslash \\ and Unicode 雨";
    queue(&mut controller, text)?;
    let update = answer(
        &mut controller,
        ContextDecision::Released,
        vec![deliver(text, "original")],
    )?;
    assert_eq!(update.dispatches.len(), 1);
    assert_eq!(
        update.dispatches[0].frame.pointer("/message/content"),
        Some(&json!(format!("Lys reminder\n{text}")))
    );
    assert!(
        update
            .receipts
            .iter()
            .all(|receipt| receipt.state != OperationState::Confirmed)
    );
    Ok(())
}

#[test]
fn saved_text_beginning_compact_is_delivered_as_reminder_data() -> TestResult {
    let (_, mut controller) = controlled()?;
    queue(&mut controller, "/compact\nkeep these words")?;
    let update = answer(
        &mut controller,
        ContextDecision::Released,
        vec![deliver("/compact\nkeep these words", "original")],
    )?;
    assert_eq!(
        update.dispatches[0].frame.pointer("/message/content"),
        Some(&json!("Lys reminder\n/compact\nkeep these words"))
    );
    Ok(())
}

#[test]
fn a_queued_old_revision_dispatches_current_words_under_the_original_occurrence() -> TestResult {
    let (_, mut controller) = controlled()?;
    queue(&mut controller, "old words")?;
    let update = answer(
        &mut controller,
        ContextDecision::Released,
        vec![deliver("new words", "edited")],
    )?;
    assert_eq!(update.dispatches[0].operation, "delivery");
    assert_eq!(
        update.dispatches[0].frame.pointer("/message/content"),
        Some(&json!("Lys reminder\nnew words"))
    );
    Ok(())
}

#[test]
fn a_revoked_goal_refuses_its_pending_words() -> TestResult {
    let (_, mut controller) = controlled()?;
    queue(&mut controller, "private words")?;
    let update = answer(
        &mut controller,
        ContextDecision::Released,
        vec![ReminderDecision::Refuse {
            operation: "delivery".to_owned(),
            reference: reference("original"),
            reason: "goal_authority_revoked".to_owned(),
        }],
    )?;
    assert!(update.dispatches.is_empty());
    assert!(
        update
            .receipts
            .iter()
            .any(|receipt| receipt.operation == "delivery"
                && receipt.state == OperationState::Refused)
    );
    assert!(controller.control_status().queued.is_empty());
    Ok(())
}

#[test]
fn another_occurrence_cannot_replace_the_queued_payload() -> TestResult {
    let (_, mut controller) = controlled()?;
    queue(&mut controller, "private words")?;
    let mut another = reference("edited");
    another.occurrence = "another-occurrence".to_owned();
    let result = answer(
        &mut controller,
        ContextDecision::Released,
        vec![ReminderDecision::Deliver {
            operation: "delivery".to_owned(),
            reference: another,
            text: "unrelated words".to_owned(),
        }],
    );
    assert!(result.is_err());
    assert_eq!(
        controller.control_status().queued[0].reference,
        reference("original")
    );
    Ok(())
}

#[test]
fn a_reminder_due_during_an_active_turn_waits_for_completion() -> TestResult {
    let (binding, mut controller) = controlled()?;
    let active = Pending::new("human".to_owned(), Kind::Human, "active words".to_owned());
    let uuid = active.uuid.clone();
    controller.enqueue(active)?;
    assert_eq!(
        answer(&mut controller, ContextDecision::Released, Vec::new())?
            .dispatches
            .len(),
        1
    );
    controller.ingest(
        &binding,
        &json!({"type":"user","session_id":"conversation","uuid":uuid,
        "parent_tool_use_id":null,"message":{"role":"user","content":"active words"}}),
    )?;
    queue(&mut controller, "saved words")?;
    assert!(controller.control_status().boundary.is_none());
    let completed = controller.ingest(&binding, &json!({"type":"result","session_id":"conversation","uuid":"result-human","is_error":false}))?;
    assert!(completed.dispatches.is_empty());
    let update = answer(
        &mut controller,
        ContextDecision::Released,
        vec![deliver("saved words", "original")],
    )?;
    assert_eq!(update.dispatches.len(), 1);
    assert_eq!(update.dispatches[0].operation, "delivery");
    Ok(())
}

#[test]
fn a_compaction_triggered_reminder_waits_for_completion_and_context_release() -> TestResult {
    let (binding, mut controller) = controlled()?;
    controller.context_compact(Pending::new(
        "crossing".to_owned(),
        Kind::Compact,
        "/compact".to_owned(),
    ))?;
    let compact = answer(
        &mut controller,
        ContextDecision::Compact {
            crossing: "crossing".to_owned(),
        },
        Vec::new(),
    )?;
    let uuid = compact.dispatches[0]
        .frame
        .get("uuid")
        .ok_or("compact uuid missing")?
        .clone();
    controller.ingest(
        &binding,
        &json!({"type":"user","session_id":"conversation","uuid":uuid,
        "parent_tool_use_id":null,"message":{"role":"user","content":"/compact"}}),
    )?;
    queue(&mut controller, "saved words")?;
    assert!(
        controller
            .ingest(
                &binding,
                &json!({"type":"system","subtype":"compact_boundary","session_id":"conversation"})
            )?
            .dispatches
            .is_empty()
    );
    let completed = controller.ingest(&binding, &json!({"type":"result","session_id":"conversation","uuid":"result-compact","is_error":false}))?;
    assert!(completed.dispatches.is_empty());
    assert!(completed.receipts.iter().any(
        |receipt| receipt.operation == "crossing" && receipt.state == OperationState::Confirmed
    ));
    assert!(
        answer(
            &mut controller,
            ContextDecision::Held {
                crossing: Some("crossing".to_owned()),
                reason: "context_post_measurement_missing".to_owned()
            },
            vec![deliver("saved words", "original")]
        )?
        .dispatches
        .is_empty()
    );
    let released = answer(
        &mut controller,
        ContextDecision::Released,
        vec![deliver("saved words", "original")],
    )?;
    assert_eq!(released.dispatches[0].operation, "delivery");
    Ok(())
}

#[test]
fn lost_boundary_authority_refuses_queued_words_without_releasing_input() -> TestResult {
    let (_, mut controller) = controlled()?;
    queue(&mut controller, "private words")?;
    let update = answer(
        &mut controller,
        ContextDecision::Unavailable {
            reason: "runner_feed_ended".to_owned(),
        },
        Vec::new(),
    )?;
    assert!(update.dispatches.is_empty());
    assert!(
        update
            .receipts
            .iter()
            .any(|receipt| receipt.state == OperationState::Refused
                && receipt.reason == "runner_feed_ended")
    );
    assert!(!controller.idle());
    Ok(())
}
