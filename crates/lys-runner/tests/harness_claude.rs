//! Claude wire fixtures distinguish admission from completed compaction.

#![cfg(test)]

use lys_runner::harness_control::{Binding, Controller, Executable, Kind, Pending, Transport};
use lys_runner::operations::OperationState;
use lys_runner::peer::{Leader, StartIdentity};
use serde_json::{Value, json};
use std::error::Error;
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
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
fn pending(id: &str, kind: Kind) -> Pending {
    Pending::new(
        id.to_owned(),
        kind,
        "/compact\nSaved words: \"quoted\"".to_owned(),
    )
}
fn ready() -> Result<Controller> {
    let mut control = Controller::new(binding(), Transport::Claude)?;
    control.ingest(&binding(), &json!({"type":"system","subtype":"init","session_id":"conversation","claude_code_version":"9.8.7","slash_commands":["compact"]}))?;
    Ok(control)
}
fn replay(uuid: &str) -> Value {
    json!({"type":"user","session_id":"conversation","uuid":uuid,"parent_tool_use_id":null,
        "message":{"role":"user","content":"the saved words"}})
}
fn result(uuid: &str) -> Value {
    json!({"type":"result","session_id":"conversation","uuid":uuid,"is_error":false})
}
#[test]
fn the_claude_fixture_sends_compact_through_the_managed_json_pipe() -> Result {
    let mut control = ready()?;
    let input = pending("compact", Kind::Compact);
    let uuid = input.uuid.clone();
    let update = control.enqueue(input)?;
    assert_eq!(update.dispatches.len(), 1);
    assert_eq!(
        update.dispatches[0].frame,
        json!({"type":"user", "uuid":uuid,
        "session_id":"conversation","parent_tool_use_id":null,"message":{"role":"user","content":"/compact"}})
    );
    assert!(!control.idle());
    Ok(())
}
#[test]
fn a_claude_success_result_without_compact_boundary_is_recorded_not_compacted() -> Result {
    let mut control = ready()?;
    let input = pending("compact", Kind::Compact);
    let uuid = input.uuid.clone();
    control.enqueue(input)?;
    control.ingest(&binding(), &replay(&uuid))?;
    let update = control.ingest(&binding(), &result("result-one"))?;
    assert_eq!(update.receipts.len(), 1);
    assert_eq!(update.receipts[0].state, OperationState::Refused);
    assert_eq!(update.receipts[0].reason, "not_compacted");
    Ok(())
}
#[test]
fn a_matching_claude_replay_uuid_records_admission() -> Result {
    let mut control = ready()?;
    let input = pending("reminder", Kind::Reminder);
    let uuid = input.uuid.clone();
    control.enqueue(input)?;
    let update = control.ingest(&binding(), &replay(&uuid))?;
    assert_eq!(update.receipts.len(), 1);
    assert_eq!(update.receipts[0].state, OperationState::Confirmed);
    assert_eq!(update.receipts[0].reason, "harness_admitted");
    assert!(!control.idle());
    assert!(
        control
            .ingest(&binding(), &replay(&uuid))?
            .receipts
            .is_empty()
    );
    Ok(())
}
#[test]
fn a_mismatched_replay_uuid_cannot_confirm_delivery() -> Result {
    let mut control = ready()?;
    control.enqueue(pending("reminder", Kind::Reminder))?;
    assert!(
        control
            .ingest(&binding(), &replay("unrelated-input"))?
            .receipts
            .is_empty()
    );
    assert!(!control.idle());
    Ok(())
}
#[test]
fn partial_assistant_text_cannot_release_a_queued_message() -> Result {
    let mut control = ready()?;
    control.enqueue(pending("first", Kind::Human))?;
    control.enqueue(pending("second", Kind::Reminder))?;
    let update = control.ingest(
        &binding(),
        &json!({"type":"assistant","session_id":"conversation",
        "message":{"content":[{"type":"text","text":"partial answer"}]}}),
    )?;
    assert!(update.dispatches.is_empty());
    assert!(!control.idle());
    Ok(())
}
#[test]
fn a_duplicate_claude_result_cannot_release_the_next_operation() -> Result {
    let mut control = ready()?;
    let first = pending("first", Kind::Human);
    let first_uuid = first.uuid.clone();
    control.enqueue(first)?;
    control.enqueue(pending("second", Kind::Reminder))?;
    control.enqueue(pending("third", Kind::Reminder))?;
    control.ingest(&binding(), &replay(&first_uuid))?;
    let ended = result("result-one");
    assert_eq!(
        control.ingest(&binding(), &ended)?.dispatches[0].operation,
        "second"
    );
    assert!(control.ingest(&binding(), &ended)?.dispatches.is_empty());
    assert!(!control.idle());
    Ok(())
}

#[test]
fn given_arguments_cannot_replace_the_managed_wire_or_conversation() -> Result {
    for argument in [
        "--input-format=text",
        "--output-format",
        "--resume",
        "--session-id=another",
    ] {
        let error =
            lys_runner::harness_control::claude::arguments(&[argument.to_owned()], "conversation")
                .err()
                .ok_or("managed protocol was overridden")?;
        assert_eq!(error.name(), "control_launch_invalid");
    }
    let arguments = lys_runner::harness_control::claude::arguments(
        &["--settings".to_owned(), "isolated.json".to_owned()],
        "conversation",
    )?;
    assert_eq!(&arguments[..2], ["--settings", "isolated.json"]);
    assert!(
        arguments
            .iter()
            .any(|argument| argument == "--replay-user-messages")
    );
    Ok(())
}

#[test]
fn subagent_and_hook_frames_cannot_end_or_release_the_owned_turn() -> Result {
    let mut control = ready()?;
    let input = pending("first", Kind::Human);
    let uuid = input.uuid.clone();
    control.enqueue(input)?;
    control.enqueue(pending("second", Kind::Reminder))?;
    for frame in [
        json!({"type":"user","session_id":"conversation","uuid":"subagent-input",
            "parent_tool_use_id":"parent-tool","message":{"role":"user","content":"tool result"}}),
        json!({"type":"system","subtype":"hook_started","session_id":"conversation"}),
        json!({"type":"system","subtype":"hook_response","session_id":"conversation"}),
        json!({"type":"tool_progress","session_id":"conversation"}),
    ] {
        let observation = control.ingest(&binding(), &frame)?;
        assert!(observation.dispatches.is_empty());
        assert!(observation.receipts.is_empty());
        assert!(!control.idle());
    }
    assert_eq!(
        control.ingest(&binding(), &replay(&uuid))?.receipts.len(),
        1
    );
    assert_eq!(
        control.ingest(&binding(), &result("final"))?.dispatches[0].operation,
        "second"
    );
    Ok(())
}

#[test]
fn a_replay_claiming_the_owned_uuid_must_prove_its_sdk_envelope() -> Result {
    for field in ["parent_tool_use_id", "message"] {
        let mut control = ready()?;
        let input = pending("owned", Kind::Reminder);
        let mut frame = replay(&input.uuid);
        frame[field] = json!("invalid");
        control.enqueue(input)?;
        let error = control
            .ingest(&binding(), &frame)
            .err()
            .ok_or("claimed replay bypassed correlation")?;
        assert_eq!(error.name(), "control_correlation_unsupported");
        assert!(!control.idle());
    }
    Ok(())
}

#[test]
fn the_claude_init_version_must_match_the_probed_version() -> Result {
    for version in [json!("9.8.6"), Value::Null] {
        let mut control = Controller::new(binding(), Transport::Claude)?;
        let error = control.ingest(&binding(), &json!({"type":"system","subtype":"init",
            "session_id":"conversation","claude_code_version":version,"slash_commands":["compact"]}))
            .err().ok_or("init bypassed the probed version")?;
        assert_eq!(error.name(), "control_adapter_unqualified");
        assert!(!control.ready());
    }
    Ok(())
}

#[test]
fn all_admitted_inputs_and_their_whole_encoded_words_remain_queued() -> Result {
    let mut control = ready()?;
    let first = pending("active", Kind::Human);
    control.enqueue(first)?;
    for at in 0..1024 {
        assert!(
            control
                .enqueue(pending(&format!("waiting-{at}"), Kind::Reminder))?
                .dispatches
                .is_empty()
        );
    }
    let whole = "quoted\n".repeat(200_000);
    assert!(
        control
            .enqueue(Pending::new("whole".to_owned(), Kind::Human, whole))?
            .dispatches
            .is_empty()
    );
    Ok(())
}
