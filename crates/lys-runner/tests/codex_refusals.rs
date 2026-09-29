#![cfg(test)]
//! Native rejection fixtures distinguish unknown cause, failure and forged notify.

use lys_runner::codex_refusals::rejection;
use serde_json::{Value, json};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn event(kind: &str, status: &str) -> Value {
    json!({"method":"item/completed", "params":{
        "threadId":"bound-thread", "turnId":"turn-a", "completedAtMs":1,
        "item":{"type":kind, "id":"item-a", "status":status,
            "command":"private-command", "aggregatedOutput":"permission denied SECRET",
            "changes":[{"diff":"private-file-contents"}]}
    }})
}

#[test]
fn native_command_and_file_rejections_have_unknown_cause() -> TestResult {
    for kind in ["commandExecution", "fileChange"] {
        let record = rejection("bound-thread", &event(kind, "declined"))?.ok_or("no rejection")?;
        let encoded = serde_json::to_value(&record)?;
        assert_eq!(encoded["source"], "codex_reported");
        assert_eq!(encoded["cause"], "unavailable");
        assert!(record.summary().contains("cause is unavailable"));
        let text = encoded.to_string();
        for secret in ["private-command", "SECRET", "private-file-contents"] {
            assert!(!text.contains(secret));
        }
    }
    Ok(())
}

#[test]
fn failed_and_completed_commands_are_not_policy_refusals() -> TestResult {
    for status in ["failed", "completed"] {
        assert!(rejection("bound-thread", &event("commandExecution", status))?.is_none());
    }
    Ok(())
}

#[test]
fn turn_complete_and_forged_notify_denial_are_not_evidence() -> TestResult {
    for notice in [
        json!({"type":"agent-turn-complete", "denied":true}),
        json!({"method":"notify", "params":{"kind":"policy_denied"}}),
    ] {
        assert!(rejection("bound-thread", &notice)?.is_none());
    }
    Ok(())
}

#[test]
fn replay_identity_is_stable_and_other_turns_remain_distinct() -> TestResult {
    let first = event("commandExecution", "declined");
    let record = rejection("bound-thread", &first)?.ok_or("no rejection")?;
    let repeated = rejection("bound-thread", &first)?.ok_or("no repeated rejection")?;
    assert_eq!(record.source_id(), repeated.source_id());
    let mut second = first;
    second["params"]["turnId"] = json!("turn-b");
    let another = rejection("bound-thread", &second)?.ok_or("no second rejection")?;
    assert_ne!(record.source_id(), another.source_id());
    Ok(())
}

#[test]
fn another_sessions_event_is_refused_by_name() {
    let result = rejection("another-thread", &event("commandExecution", "declined"));
    assert!(result.is_err());
    if let Err(error) = result {
        assert!(
            error
                .to_string()
                .starts_with("codex_event_session_mismatch:")
        );
    }
}

#[test]
fn new_status_cannot_disappear_as_a_non_refusal() {
    for status in ["unknown-rejection", "inProgress"] {
        let result = rejection("bound-thread", &event("commandExecution", status));
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(
                error
                    .to_string()
                    .starts_with("codex_policy_contract_unsupported:")
            );
        }
    }
}
