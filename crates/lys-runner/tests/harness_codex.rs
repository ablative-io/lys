//! Codex wire fixtures preserve boundaries and permission settings.

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
    let mut control = Controller::new(binding(), Transport::Codex)?;
    control.ingest(
        &binding(),
        &json!({"id":"lys-initialize","result":{"userAgent":"lys-runner/9.8.7 (fixture)"}}),
    )?;
    control.ingest(&binding(), &json!({"id":"lys-thread","result":{"thread":{"id":"conversation","status":{"type":"idle"}}}}))?;
    Ok(control)
}
fn started(turn: &str) -> Value {
    json!({"method":"turn/started","params":{"threadId":"conversation","turn":{"id":turn}}})
}
fn completed(turn: &str) -> Value {
    json!({"method":"turn/completed","params":{"threadId":"conversation","turn":{"id":turn,"status":"completed"}}})
}
#[test]
fn the_codex_fixture_emits_thread_compact_start_for_the_bound_thread() -> Result {
    let mut control = ready()?;
    let update = control.enqueue(pending("compact", Kind::Compact))?;
    assert_eq!(
        update.dispatches[0].frame,
        json!({"id":"compact","method":"thread/compact/start","params":{"threadId":"conversation"}})
    );
    Ok(())
}
#[test]
fn the_empty_codex_compact_response_leaves_the_operation_accepted() -> Result {
    let mut control = ready()?;
    control.enqueue(pending("compact", Kind::Compact))?;
    let update = control.ingest(&binding(), &json!({"id":"compact","result":{}}))?;
    assert!(update.receipts.is_empty());
    assert!(!control.idle());
    Ok(())
}
#[test]
fn a_completed_context_compaction_item_with_matching_thread_and_turn_confirms_compaction() -> Result
{
    let mut control = ready()?;
    control.enqueue(pending("compact", Kind::Compact))?;
    control.ingest(&binding(), &json!({"id":"compact","result":{}}))?;
    control.ingest(&binding(), &started("compact-turn"))?;
    let wrong = json!({"method":"item/completed","params":{"threadId":"other","turnId":"compact-turn","item":{"type":"contextCompaction","id":"item"}}});
    assert!(control.ingest(&binding(), &wrong)?.receipts.is_empty());
    let matching = json!({"method":"item/completed","params":{"threadId":"conversation","turnId":"compact-turn","item":{"type":"contextCompaction","id":"item"}}});
    assert!(control.ingest(&binding(), &matching)?.receipts.is_empty());
    assert!(
        control
            .ingest(&binding(), &completed("other-turn"))?
            .receipts
            .is_empty()
    );
    let update = control.ingest(&binding(), &completed("compact-turn"))?;
    assert_eq!(update.receipts[0].state, OperationState::Confirmed);
    assert!(control.idle());
    Ok(())
}
#[test]
fn a_codex_reminder_uses_turn_start_only_after_turn_completed() -> Result {
    let mut control = ready()?;
    control.ingest(&binding(), &started("active-turn"))?;
    assert!(
        control
            .enqueue(pending("reminder", Kind::Reminder))?
            .dispatches
            .is_empty()
    );
    let update = control.ingest(&binding(), &completed("active-turn"))?;
    assert_eq!(update.dispatches[0].frame["method"], "turn/start");
    assert_eq!(
        update.dispatches[0].frame["params"]["threadId"],
        "conversation"
    );
    assert_eq!(
        update.dispatches[0].frame["params"]["input"][0]["type"],
        "text"
    );
    Ok(())
}
#[test]
fn a_reminder_request_contains_no_sandbox_override() -> Result {
    let mut control = ready()?;
    let update = control.enqueue(pending("reminder", Kind::Reminder))?;
    let params = &update.dispatches[0].frame["params"];
    assert!(params.get("sandboxPolicy").is_none());
    assert!(params.get("sandbox").is_none());
    assert!(params.get("permissions").is_none());
    Ok(())
}
#[test]
fn a_reminder_request_contains_no_approval_override() -> Result {
    let mut control = ready()?;
    let update = control.enqueue(pending("reminder", Kind::Reminder))?;
    let params = &update.dispatches[0].frame["params"];
    assert!(params.get("approvalPolicy").is_none());
    assert!(params.get("approvalsReviewer").is_none());
    Ok(())
}
#[test]
fn a_harness_approval_request_receives_no_fabricated_approval() -> Result {
    let mut control = ready()?;
    control.enqueue(pending("reminder", Kind::Reminder))?;
    let error = control.ingest(&binding(), &json!({"id":"approval","method":"item/commandExecution/requestApproval",
        "params":{"threadId":"conversation","turnId":"active-turn","itemId":"tool","command":"do-something"}}))
        .err().ok_or("approval request was accepted without an authority")?;
    assert_eq!(error.name(), "control_approval_unsupported");
    assert!(!control.idle());
    Ok(())
}
#[test]
fn managed_codex_arguments_preserve_the_explicit_model_and_sandbox() -> Result {
    let arguments = lys_runner::harness_control::codex::arguments(&[
        "--model".to_owned(),
        "chosen-model".to_owned(),
        "--sandbox".to_owned(),
        "workspace-write".to_owned(),
        "-c".to_owned(),
        "sandbox_workspace_write.network_access=false".to_owned(),
        "-c".to_owned(),
        "sandbox_workspace_write.exclude_slash_tmp=true".to_owned(),
    ])?;
    for expected in [
        "model=\"chosen-model\"",
        "sandbox_mode=\"workspace-write\"",
        "sandbox_workspace_write.network_access=false",
        "sandbox_workspace_write.exclude_slash_tmp=true",
    ] {
        assert!(
            arguments.iter().any(|argument| argument == expected),
            "missing {expected}"
        );
    }
    assert_eq!(
        &arguments[arguments.len() - 3..],
        ["app-server", "--listen", "stdio://"]
    );
    Ok(())
}
#[test]
fn additional_directories_are_added_to_the_explicit_roots_without_loss() -> Result {
    let arguments = lys_runner::harness_control::codex::arguments(&[
        "-c".to_owned(),
        "sandbox_workspace_write.writable_roots=[\"/original\"]".to_owned(),
        "--add-dir".to_owned(),
        "/additional".to_owned(),
    ])?;
    assert!(arguments.iter().any(|argument| argument
        == "sandbox_workspace_write.writable_roots=[\"/original\",\"/additional\"]"));
    Ok(())
}
#[test]
fn additional_directories_without_proved_base_roots_are_refused() -> Result {
    let error = lys_runner::harness_control::codex::arguments(&[
        "--add-dir".to_owned(),
        "/additional".to_owned(),
    ])
    .err()
    .ok_or("unproved base writable roots were replaced")?;
    assert_eq!(error.name(), "control_launch_unsupported");
    Ok(())
}

#[test]
fn another_session_or_generation_cannot_release_the_owned_turn() -> Result {
    let mut control = ready()?;
    control.ingest(&binding(), &started("active"))?;
    control.enqueue(pending("waiting", Kind::Reminder))?;
    let other_thread = json!({"method":"turn/completed","params":{"threadId":"other","turn":{"id":"active","status":"completed"}}});
    assert!(
        control
            .ingest(&binding(), &other_thread)?
            .dispatches
            .is_empty()
    );
    for different in [true, false] {
        let mut source = binding();
        if different {
            source.session = "other-session".to_owned();
        } else {
            source.generation += 1;
        }
        let error = control
            .ingest(&source, &completed("active"))
            .err()
            .ok_or("foreign completion was accepted")?;
        assert_eq!(error.name(), "control_source_mismatch");
    }
    assert!(!control.idle());
    assert_eq!(
        control
            .ingest(&binding(), &completed("active"))?
            .dispatches
            .len(),
        1
    );
    Ok(())
}
#[test]
fn a_duplicate_completion_cannot_release_the_next_reserved_turn() -> Result {
    let mut control = ready()?;
    control.ingest(&binding(), &started("active"))?;
    control.enqueue(pending("first", Kind::Reminder))?;
    control.enqueue(pending("second", Kind::Reminder))?;
    assert_eq!(
        control
            .ingest(&binding(), &completed("active"))?
            .dispatches
            .len(),
        1
    );
    assert!(
        control
            .ingest(&binding(), &completed("active"))?
            .dispatches
            .is_empty()
    );
    control.ingest(
        &binding(),
        &json!({"id":"first","result":{"turn":{"id":"next"}}}),
    )?;
    assert!(
        control
            .ingest(&binding(), &completed("active"))?
            .dispatches
            .is_empty()
    );
    assert_eq!(
        control
            .ingest(&binding(), &completed("next"))?
            .dispatches
            .len(),
        1
    );
    Ok(())
}
#[test]
fn a_lost_connection_keeps_the_boundary_unknown_and_does_not_replay_input() -> Result {
    let mut control = ready()?;
    control.enqueue(pending("possibly-written", Kind::Reminder))?;
    let lost = control.disconnected();
    assert_eq!(lost.receipts[0].state, OperationState::Uncertain);
    assert!(lost.dispatches.is_empty());
    assert!(!control.ready());
    for kind in [Kind::Reminder, Kind::Human] {
        let error = control
            .enqueue(pending("waiting", kind))
            .err()
            .ok_or("lost channel accepted new input")?;
        assert_eq!(error.name(), "control_source_unbound");
    }
    let error = control
        .ingest(&binding(), &completed("stale"))
        .err()
        .ok_or("lost generation accepted more frames")?;
    assert_eq!(error.name(), "control_transport_lost");
    assert!(!control.idle());
    Ok(())
}

#[test]
fn a_correlated_request_error_refuses_the_whole_queue_and_closes_readiness() -> Result {
    for kind in [Kind::Reminder, Kind::Compact] {
        let mut control = ready()?;
        control.enqueue(pending("refused", kind))?;
        control.enqueue(pending("queued-human", Kind::Human))?;
        control.enqueue(pending("queued-reminder", Kind::Reminder))?;
        let update = control.ingest(
            &binding(),
            &json!({"id":"refused","error":{"code":-1,"message":"refused"}}),
        )?;
        assert_eq!(update.receipts.len(), 3);
        assert!(
            update
                .receipts
                .iter()
                .all(|receipt| receipt.state == OperationState::Refused)
        );
        assert!(update.dispatches.is_empty());
        assert!(!control.ready());
        assert_eq!(
            control
                .enqueue(pending("later", Kind::Human))
                .err()
                .ok_or("refused generation accepted new input")?
                .name(),
            "control_source_unbound"
        );
    }
    Ok(())
}

#[test]
fn a_non_idle_resumed_thread_refuses_before_binding_ready() -> Result {
    for status in [
        json!({"type":"active"}),
        json!({"type":"notLoaded"}),
        Value::Null,
    ] {
        let mut control = Controller::new(binding(), Transport::Codex)?;
        control.ingest(
            &binding(),
            &json!({"id":"lys-initialize","result":{"userAgent":"lys-runner/9.8.7 (fixture)"}}),
        )?;
        let error = control.ingest(&binding(), &json!({"id":"lys-thread","result":{"thread":{"id":"conversation","status":status}}}))
            .err().ok_or("non-idle thread bound ready")?;
        assert_eq!(error.name(), "control_source_unbound");
        assert!(!control.ready());
        assert!(!control.idle());
    }
    Ok(())
}

#[test]
fn unsupported_protocol_variants_are_named_instead_of_becoming_idle() -> Result {
    for value in [
        json!({"method":"future/method","params":{"threadId":"conversation"}}),
        json!({"method":"item/completed","params":{"threadId":"conversation","turnId":"active","item":{"type":"futureItem"}}}),
        json!({"method":"turn/completed","params":{"threadId":"conversation","turn":{"id":"active","status":"futureStatus"}}}),
    ] {
        let mut control = ready()?;
        control.ingest(&binding(), &started("active"))?;
        let error = control
            .ingest(&binding(), &value)
            .err()
            .ok_or("unsupported variant was silently accepted")?;
        assert_eq!(error.name(), "control_protocol_unsupported");
        assert!(!control.idle());
    }
    Ok(())
}
#[test]
fn a_large_encoded_message_is_carried_whole() -> Result {
    let mut control = ready()?;
    let text = "\"".repeat(600_000);
    let update = control.enqueue(Pending::new("whole".to_owned(), Kind::Human, text.clone()))?;
    assert_eq!(
        update.dispatches[0].frame["params"]["input"][0]["text"],
        text
    );
    assert!(!control.idle());
    Ok(())
}

#[test]
fn the_codex_initialize_version_must_match_the_probed_version() -> Result {
    for user_agent in ["lys-runner/9.8.6 (fixture)", "lys-runner (fixture)"] {
        let mut control = Controller::new(binding(), Transport::Codex)?;
        let error = control
            .ingest(
                &binding(),
                &json!({"id":"lys-initialize","result":{"userAgent":user_agent}}),
            )
            .err()
            .ok_or("initialize bypassed the probed version")?;
        assert_eq!(error.name(), "control_adapter_unqualified");
        assert!(!control.ready());
    }
    Ok(())
}
