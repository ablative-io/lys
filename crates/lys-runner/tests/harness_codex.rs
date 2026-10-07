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

const CONNECTION_METHODS: [&str; 21] = [
    "skills/changed",
    "project/changed",
    "command/exec/outputDelta",
    "process/outputDelta",
    "process/exited",
    "mcpServer/event/stream/notification",
    "account/updated",
    "account/gatewayOAuth/changed",
    "account/rateLimits/updated",
    "app/list/updated",
    "remoteControl/status/changed",
    "externalAgentConfig/import/progress",
    "externalAgentConfig/import/completed",
    "fs/changed",
    "deprecationNotice",
    "configWarning",
    "fuzzyFileSearch/sessionUpdated",
    "fuzzyFileSearch/sessionCompleted",
    "windows/worldWritableWarning",
    "windowsSandbox/setupCompleted",
    "account/login/completed",
];
const OPTIONAL_THREAD_METHODS: [&str; 3] = [
    "warning",
    "mcpServer/oauthLogin/completed",
    "mcpServer/startupStatus/updated",
];

fn unchanged(control: &mut Controller, frame: &Value) -> Result {
    let before = control.control_status();
    let identity = control.binding.clone();
    let ready = control.ready();
    let idle = control.idle();
    let update = control.ingest(&binding(), frame)?;
    assert!(update.events.is_empty(), "{frame}");
    assert!(update.receipts.is_empty(), "{frame}");
    assert!(update.dispatches.is_empty(), "{frame}");
    assert_eq!(control.control_status(), before);
    assert_eq!(control.binding, identity);
    assert_eq!(control.ready(), ready);
    assert_eq!(control.idle(), idle);
    Ok(())
}

#[test]
fn connection_notifications_leave_unbound_and_active_control_unchanged() -> Result {
    let mut unbound = Controller::new(binding(), Transport::Codex)?;
    let mut active = ready()?;
    active.ingest(&binding(), &started("owned-turn"))?;
    active.enqueue(pending("waiting", Kind::Reminder))?;
    for method in CONNECTION_METHODS {
        let frame = json!({"method":method,"params":{"payload":"private fixture value"}});
        unchanged(&mut unbound, &frame)?;
        unchanged(&mut active, &frame)?;
    }
    assert_eq!(
        active
            .ingest(&binding(), &completed("owned-turn"))?
            .dispatches
            .len(),
        1
    );
    Ok(())
}

#[test]
fn optional_thread_notifications_preserve_control_for_each_identity_shape() -> Result {
    let mut control = ready()?;
    control.ingest(&binding(), &started("owned-turn"))?;
    control.enqueue(pending("waiting", Kind::Reminder))?;
    let mut unbound = Controller::new(binding(), Transport::Codex)?;
    for method in OPTIONAL_THREAD_METHODS {
        for params in [json!({}), json!({"threadId":null})] {
            unchanged(&mut unbound, &json!({"method":method,"params":params}))?;
        }
        for params in [
            json!({"threadId":"conversation"}),
            json!({"threadId":"other"}),
            json!({}),
            json!({"threadId":null}),
        ] {
            unchanged(&mut control, &json!({"method":method,"params":params}))?;
        }
        for thread in [json!(1), json!({"id":"conversation"}), json!("")] {
            let error = control
                .ingest(
                    &binding(),
                    &json!({"method":method,"params":{"threadId":thread}}),
                )
                .err()
                .ok_or("malformed optional thread was accepted")?;
            assert_eq!(error.name(), "control_correlation_unsupported");
            assert!(error.to_string().contains(method));
        }
    }
    assert_eq!(
        control
            .ingest(&binding(), &completed("owned-turn"))?
            .dispatches
            .len(),
        1
    );
    Ok(())
}

#[test]
fn thread_started_requires_its_nested_identity_without_binding_control() -> Result {
    let mut control = Controller::new(binding(), Transport::Codex)?;
    for id in ["conversation", "other"] {
        unchanged(
            &mut control,
            &json!({"method":"thread/started","params":{"thread":{"id":id}}}),
        )?;
    }
    for params in [
        json!({}),
        json!({"thread":{}}),
        json!({"thread":{"id":1}}),
        json!({"thread":{"id":""}}),
    ] {
        let error = control
            .ingest(
                &binding(),
                &json!({"method":"thread/started","params":params}),
            )
            .err()
            .ok_or("thread/started without typed identity was accepted")?;
        assert_eq!(error.name(), "control_correlation_unsupported");
        assert!(error.to_string().contains("thread/started"));
        assert!(error.to_string().contains("params.thread.id"));
    }
    assert!(!control.ready());
    Ok(())
}

#[test]
fn unknown_notifications_name_method_and_parameter_keys_without_values() -> Result {
    let mut control = ready()?;
    for method in [
        "unrecognised/event",
        "account/unrecognised",
        "warning/unrecognised",
    ] {
        let error = control
            .ingest(
                &binding(),
                &json!({"method":method,"params":{
                    "opaque":"private fixture phrase", "nested":{"secret":"private fixture phrase"}
                }}),
            )
            .err()
            .ok_or("unknown notification was accepted")?;
        assert_eq!(error.name(), "control_correlation_unsupported");
        let reason = error.to_string();
        assert!(reason.contains(method), "{reason}");
        assert!(reason.contains("opaque"), "{reason}");
        assert!(reason.contains("nested"), "{reason}");
        assert!(!reason.contains("private fixture phrase"));
        assert!(!reason.contains("secret"));
    }
    Ok(())
}

#[test]
fn connection_notifications_cannot_approve_a_server_request() -> Result {
    let mut control = ready()?;
    for method in CONNECTION_METHODS
        .into_iter()
        .chain(OPTIONAL_THREAD_METHODS)
    {
        let before = control.control_status();
        let error = control
            .ingest(
                &binding(),
                &json!({"id":"approval","method":method,"params":{}}),
            )
            .err()
            .ok_or("server request was treated as a notification")?;
        assert_eq!(error.name(), "control_approval_unsupported");
        assert_eq!(control.control_status(), before);
    }
    Ok(())
}

#[test]
fn threaded_notification_refusals_keep_method_and_keys_without_payload_values() -> Result {
    let mut control = ready()?;
    for frame in [
        json!({"method":"unknown/threaded","params":{"threadId":"conversation","payload":"private fixture phrase"}}),
        json!({"method":"turn/started","params":{"threadId":"conversation","payload":"private fixture phrase"}}),
        json!({"method":"turn/completed","params":{"threadId":"conversation","turn":{"id":"owned","status":"private fixture phrase"},"payload":"private fixture phrase"}}),
    ] {
        let error = control
            .ingest(&binding(), &frame)
            .err()
            .ok_or("malformed notification accepted")?;
        let reason = error.to_string();
        let method = frame["method"].as_str().ok_or("fixture method missing")?;
        assert!(reason.contains(method), "{reason}");
        assert!(reason.contains("threadId"), "{reason}");
        assert!(reason.contains("payload"), "{reason}");
        assert!(!reason.contains("private fixture phrase"));
    }
    Ok(())
}

#[test]
fn malformed_notification_params_remain_protocol_faults_with_named_methods() -> Result {
    let mut control = ready()?;
    for frame in [
        json!({"method":"warning"}),
        json!({"method":"turn/started","params":null}),
        json!({"method":"thread/started","params":["private fixture phrase"]}),
    ] {
        let error = control
            .ingest(&binding(), &frame)
            .err()
            .ok_or("malformed params were accepted")?;
        assert_eq!(error.name(), "control_protocol_unsupported");
        let reason = error.to_string();
        assert!(reason.contains(frame["method"].as_str().ok_or("fixture method missing")?));
        assert!(reason.contains("params_keys=[]"));
        assert!(!reason.contains("private fixture phrase"));
    }
    Ok(())
}

const NOTIFICATIONS: [(&str, &str, u32, bool); 85] = [
    ("error", "v2-notification.rs", 62, true),
    ("thread/started", "v2-thread.rs", 1972, false),
    ("thread/status/changed", "v2-thread.rs", 1979, true),
    ("thread/archived", "v2-thread.rs", 1987, true),
    ("thread/deleted", "v2-thread.rs", 1994, true),
    ("thread/unarchived", "v2-thread.rs", 2001, true),
    ("thread/closed", "v2-thread.rs", 2008, true),
    ("thread/reverted", "v2-thread.rs", 2015, true),
    ("skills/changed", "v2-plugin.rs", 1057, false),
    ("thread/name/updated", "v2-thread.rs", 2021, true),
    (
        "thread/attachment/updated",
        "v2-thread_attachment.rs",
        101,
        true,
    ),
    ("thread/goal/updated", "v2-thread.rs", 2031, true),
    ("thread/goal/cleared", "v2-thread.rs", 2040, true),
    ("thread/queue/changed", "v2-thread.rs", 2047, true),
    ("project/changed", "v2-project.rs", 180, false),
    ("thread/project/updated", "v2-project.rs", 188, true),
    (
        "thread/environment/connected",
        "v2-environment.rs",
        60,
        true,
    ),
    (
        "thread/environment/disconnected",
        "v2-environment.rs",
        60,
        true,
    ),
    ("thread/settings/updated", "v2-thread.rs", 330, true),
    ("thread/tokenUsage/updated", "v2-thread.rs", 1878, true),
    ("turn/started", "v2-turn.rs", 532, true),
    ("hook/started", "v2-hook.rs", 145, true),
    ("turn/completed", "v2-turn.rs", 549, true),
    ("hook/completed", "v2-hook.rs", 154, true),
    ("turn/diff/updated", "v2-turn.rs", 559, true),
    ("turn/plan/updated", "v2-turn.rs", 568, true),
    ("item/started", "v2-item.rs", 1337, true),
    ("item/autoApprovalReview/started", "v2-item.rs", 1351, true),
    (
        "item/autoApprovalReview/completed",
        "v2-item.rs",
        1382,
        true,
    ),
    (
        "autoApprovalReview/strictReviewRequired",
        "v2-notification.rs",
        51,
        true,
    ),
    ("item/completed", "v2-item.rs", 1415, true),
    ("rawResponseItem/completed", "v2-item.rs", 1427, true),
    ("rawResponse/completed", "v2-thread.rs", 1889, true),
    ("item/agentMessage/delta", "v2-item.rs", 1437, true),
    ("item/plan/delta", "v2-item.rs", 1449, true),
    ("command/exec/outputDelta", "v2-command_exec.rs", 202, false),
    ("process/outputDelta", "v2-process.rs", 165, false),
    ("process/exited", "v2-process.rs", 181, false),
    (
        "item/commandExecution/outputDelta",
        "v2-item.rs",
        1506,
        true,
    ),
    (
        "item/commandExecution/terminalInteraction",
        "v2-item.rs",
        1494,
        true,
    ),
    ("item/fileChange/outputDelta", "v2-item.rs", 1518, true),
    ("item/fileChange/patchUpdated", "v2-item.rs", 1528, true),
    ("serverRequest/resolved", "v2-notification.rs", 74, true),
    ("item/mcpToolCall/progress", "v2-mcp.rs", 328, true),
    ("mcpServer/oauthLogin/completed", "v2-mcp.rs", 338, false),
    ("mcpServer/startupStatus/updated", "v2-mcp.rs", 360, false),
    (
        "mcpServer/event/stream/notification",
        "v2-mcp.rs",
        193,
        false,
    ),
    ("account/updated", "v2-account.rs", 588, false),
    ("account/gatewayOAuth/changed", "v2-account.rs", 606, false),
    ("account/rateLimits/updated", "v2-account.rs", 657, false),
    ("app/list/updated", "v2-apps.rs", 269, false),
    (
        "remoteControl/status/changed",
        "v2-remote_control.rs",
        30,
        false,
    ),
    (
        "externalAgentConfig/import/progress",
        "v2-config.rs",
        1056,
        false,
    ),
    (
        "externalAgentConfig/import/completed",
        "v2-config.rs",
        1064,
        false,
    ),
    ("fs/changed", "v2-fs.rs", 199, false),
    ("item/reasoning/summaryTextDelta", "v2-item.rs", 1459, true),
    ("item/reasoning/summaryPartAdded", "v2-item.rs", 1471, true),
    ("item/reasoning/textDelta", "v2-item.rs", 1482, true),
    ("thread/compacted", "v2-thread.rs", 2055, true),
    ("model/rerouted", "v2-model.rs", 188, true),
    ("model/verification", "v2-model.rs", 199, true),
    (
        "modelProvider/authRecoveryStarted",
        "v2-notification.rs",
        11,
        true,
    ),
    (
        "modelProvider/authRecoveryCompleted",
        "v2-notification.rs",
        11,
        true,
    ),
    ("turn/moderationMetadata", "v2-model.rs", 208, true),
    ("model/safetyBuffering/updated", "v2-model.rs", 217, true),
    ("warning", "v2-notification.rs", 31, false),
    ("guardianWarning", "v2-notification.rs", 41, true),
    ("deprecationNotice", "v2-notification.rs", 21, false),
    ("configWarning", "v2-config.rs", 1130, false),
    ("fuzzyFileSearch/sessionUpdated", "common.rs", 1903, false),
    ("fuzzyFileSearch/sessionCompleted", "common.rs", 1912, false),
    ("thread/realtime/started", "v2-realtime.rs", 382, true),
    ("thread/realtime/itemAdded", "v2-realtime.rs", 392, true),
    ("thread/realtime/item/started", "v2-realtime.rs", 401, true),
    (
        "thread/realtime/item/transcript/delta",
        "v2-realtime.rs",
        410,
        true,
    ),
    (
        "thread/realtime/item/completed",
        "v2-realtime.rs",
        420,
        true,
    ),
    (
        "thread/realtime/transcript/delta",
        "v2-realtime.rs",
        430,
        true,
    ),
    (
        "thread/realtime/transcript/done",
        "v2-realtime.rs",
        442,
        true,
    ),
    (
        "thread/realtime/outputAudio/delta",
        "v2-realtime.rs",
        453,
        true,
    ),
    ("thread/realtime/sdp", "v2-realtime.rs", 462, true),
    ("thread/realtime/error", "v2-realtime.rs", 471, true),
    ("thread/realtime/closed", "v2-realtime.rs", 480, true),
    (
        "windows/worldWritableWarning",
        "v2-windows_sandbox.rs",
        10,
        false,
    ),
    (
        "windowsSandbox/setupCompleted",
        "v2-windows_sandbox.rs",
        68,
        false,
    ),
    ("account/login/completed", "v2-account.rs", 814, false),
];

fn notification_case(method: &str, thread: &str) -> Result {
    let mut control = ready()?;
    control.ingest(&binding(), &started("active"))?;
    control.enqueue(pending("waiting", Kind::Reminder))?;
    let frame = json!({"method":method,"params":{
        "threadId":thread,"thread":{"id":thread},"turnId":"active",
        "turn":{"id":"active","status":"completed"},"item":{"type":"agentMessage"},
        "run":{},"private":"never retained"}});
    if thread == "conversation"
        && matches!(
            method,
            "thread/closed" | "thread/archived" | "thread/deleted" | "thread/reverted"
        )
    {
        let error = control
            .ingest(&binding(), &frame)
            .err()
            .ok_or("ended thread was accepted")?;
        if error.name() != "control_thread_ended" || !error.to_string().contains(method) {
            return Err(format!("ended thread has the wrong refusal: {error}").into());
        }
        if error.to_string().contains("never retained") {
            return Err("ended-thread refusal retains payload".into());
        }
    } else if thread == "conversation" && method == "turn/completed" {
        let update = control.ingest(&binding(), &frame)?;
        if update.dispatches.len() != 1 || update.dispatches[0].operation != "waiting" {
            return Err("matching turn did not release the queued request".into());
        }
    } else {
        unchanged(&mut control, &frame)?;
    }
    Ok(())
}

#[test]
fn every_documented_notification_has_an_exact_bound_and_foreign_thread_answer() {
    let mut failures = Vec::new();
    for (method, file, line, _) in NOTIFICATIONS {
        for thread in ["conversation", "other"] {
            if let Err(error) = notification_case(method, thread) {
                failures.push(format!("{method} {file}:{line} thread={thread}: {error}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_required_thread_notification_refuses_a_malformed_correlation() -> Result {
    let mut failures = Vec::new();
    for (method, file, line, required) in NOTIFICATIONS {
        if !required {
            continue;
        }
        for params in [
            json!({}),
            json!({"threadId":null}),
            json!({"threadId":1}),
            json!({"threadId":""}),
        ] {
            let mut control = ready()?;
            let answer = control.ingest(&binding(), &json!({"method":method,"params":params}));
            if !answer.is_err_and(|error| error.name() == "control_correlation_unsupported") {
                failures.push(format!(
                    "{method} {file}:{line} accepted malformed threadId"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn an_unknown_foreign_notification_is_refused_without_retaining_its_payload() -> Result {
    let mut control = ready()?;
    let error = control
        .ingest(
            &binding(),
            &json!({"method":"hook/future","params":{
        "threadId":"other","private":"never retained"}}),
        )
        .err()
        .ok_or("unknown foreign method was admitted")?;
    assert_eq!(error.name(), "control_protocol_unsupported");
    let words = error.to_string();
    assert!(words.contains("hook/future") && words.contains("private"));
    assert!(!words.contains("never retained"));
    Ok(())
}
