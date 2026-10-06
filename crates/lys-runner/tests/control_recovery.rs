#![cfg(test)]
//! A prepared request's certainty survives without restoring a dead pipe.

use std::error::Error;
use lys_runner::{Act, Sessions};
use lys_runner::harness_control::{Kind, Pending};
use lys_runner::operations::{OperationState, TextDigest};
use lys_core::Ed25519Identity;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn prepared(certainty: &str) -> Result<tempfile::TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let pending = Pending::new("delivery".to_owned(), Kind::Reminder, "private goal words".to_owned());
    let at = u64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis())?;
    let outcome = json!({"operation":"delivery","session":"session","request":"goal_reminder",
        "state":if certainty == "safely_unsent" { "accepted" } else { "delivering" },
        "at":at,"words":"prepared","text":TextDigest::of("private goal words"),"ended":null});
    let control = json!({"prepared": {
        "binding":{"session":"session","generation":1,"leader":{"pid":42,"start":"proved-start"},
            "conversation":"conversation","entry":{"path":"entry","sha256":"entry-digest"},
            "harness":{"path":"harness","sha256":"harness-digest"},"harness_version":"9.8.7","adapter":"fixture-adapter/1"},
        "uuid":pending.uuid.clone(),"frame":{"type":"user","uuid":pending.uuid,"session_id":"conversation",
            "message":{"role":"user","content":"Lys reminder\nprivate goal words"}},
        "reference":{"goal":"goal","occurrence":"occurrence","version":"original","prior":null}},
        "certainty":certainty,"admitted":null,"decision":null});
    let record = json!({"outcome":outcome,"control":control});
    let mut bytes = serde_json::to_vec(&record)?;
    bytes.push(b'\n');
    std::fs::write(dir.path().join("operations.v2.journal"), bytes)?;
    Ok(dir)
}

fn receipt(sessions: &std::sync::Arc<Sessions>, directory: &std::path::Path) -> Result<Value, Box<dyn Error>> {
    let act: Act = serde_json::from_value(json!({"act":"control_receipt","operation":"delivery"}))?;
    let key = Ed25519Identity::load_or_generate(&directory.join("key"))?;
    let greeting = lys_runner::protocol::Greeting::fresh("21");
    let request = lys_runner::protocol::sign_request(&key, &greeting, &act)?;
    let answer = lys_runner::socket::dispatch(sessions, &key.public_key_bytes(), &greeting,
        &request, &std::sync::atomic::AtomicBool::new(false));
    let wire = serde_json::to_value(answer)?;
    assert_eq!(wire["kind"], "control_receipt", "{wire}");
    Ok(wire["receipt"].clone())
}

#[test]
fn a_crash_before_a_write_attempt_preserves_the_safely_unsent_preparation() -> TestResult {
    let dir = prepared("safely_unsent")?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let outcome = sessions.outcome("delivery")?;
    assert_eq!(outcome.state, OperationState::Refused);
    let receipt = receipt(&sessions, dir.path())?;
    assert_eq!(receipt["certainty"], "safely_unsent");
    assert_eq!(receipt["prepared"], true);
    assert!(sessions.status(Some("session"))?.sessions.is_empty());
    Ok(())
}

#[test]
fn a_crash_after_a_possible_write_keeps_the_original_operation_uncertain() -> TestResult {
    let dir = prepared("possibly_sent")?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    assert_eq!(sessions.outcome("delivery")?.state, OperationState::Uncertain);
    let receipt = receipt(&sessions, dir.path())?;
    assert_eq!(receipt["certainty"], "possibly_sent");
    assert_eq!(receipt["operation"], "delivery");
    Ok(())
}

#[test]
fn outbound_control_receipts_contain_identifiers_and_states_without_goal_words() -> TestResult {
    let dir = prepared("possibly_sent")?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let receipt = receipt(&sessions, dir.path())?;
    assert_eq!(receipt["reference"]["goal"], "goal");
    assert!(!serde_json::to_string(&receipt)?.contains("private goal words"));
    assert!(receipt.get("frame").is_none());
    Ok(())
}
