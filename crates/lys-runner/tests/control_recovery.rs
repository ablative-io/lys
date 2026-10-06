#![cfg(test)]
//! A prepared request's certainty survives without restoring a dead pipe.

use lys_core::Ed25519Identity;
use lys_runner::harness_control::{Kind, Pending};
use lys_runner::operations::{OperationState, TextDigest};
use lys_runner::{Act, Sessions};
use serde_json::{Value, json};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn prepared(certainty: &str) -> Result<tempfile::TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let pending = Pending::new(
        "delivery".to_owned(),
        Kind::Reminder,
        "private goal words".to_owned(),
    );
    let at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let outcome = json!({"operation":"delivery","session":"session","request":"goal_reminder",
        "state":if certainty == "safely_unsent" { "accepted" } else { "delivering" },
        "at":at,"words":"prepared","text":TextDigest::of("private goal words"),"ended":null});
    let control = json!({"prepared": {
        "binding":{"session":"session","generation":1,"leader":{"pid":42,"start":"proved-start"},
            "conversation":"conversation","entry":{"path":"entry","sha256":"entry-digest"},
            "harness":{"path":"harness","sha256":"harness-digest"},"harness_version":"9.8.7","adapter":"fixture-adapter/1"},
        "uuid":pending.uuid,"frame":{"type":"user","uuid":pending.uuid,"session_id":"conversation",
            "message":{"role":"user","content":"Lys reminder\nprivate goal words"}},
        "reference":{"goal":"goal","occurrence":"occurrence","version":"original","prior":null}},
        "original_text":TextDigest::of("private goal words"),"certainty":certainty,"admitted":null,"decision":null});
    let record = json!({"outcome":outcome,"control":control});
    let mut bytes = serde_json::to_vec(&record)?;
    bytes.push(b'\n');
    std::fs::write(dir.path().join("operations.v2.journal"), bytes)?;
    Ok(dir)
}

fn receipt(
    sessions: &std::sync::Arc<Sessions>,
    directory: &std::path::Path,
) -> Result<Value, Box<dyn Error>> {
    let act: Act = serde_json::from_value(json!({"act":"control_receipt","operation":"delivery"}))?;
    let key = Ed25519Identity::load_or_generate(&directory.join("key"))?;
    let greeting = lys_runner::protocol::Greeting::fresh("21");
    let request = lys_runner::protocol::sign_request(&key, &greeting, &act)?;
    let answer = lys_runner::socket::dispatch(
        sessions,
        &key.public_key_bytes(),
        &greeting,
        &request,
        &std::sync::atomic::AtomicBool::new(false),
    );
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
    assert!(sessions.status(None)?.sessions.is_empty());
    Ok(())
}

#[test]
fn a_crash_after_a_possible_write_keeps_the_original_operation_uncertain() -> TestResult {
    let dir = prepared("possibly_sent")?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    assert_eq!(
        sessions.outcome("delivery")?.state,
        OperationState::Uncertain
    );
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

#[test]
fn matching_saved_admission_reconciles_the_original_operation_without_a_pipe() -> TestResult {
    let dir = prepared("possibly_sent")?;
    let path = dir.path().join("operations.v2.journal");
    let mut record: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    record["control"]["admitted"] = json!({"binding":record["control"]["prepared"]["binding"],
        "uuid":record["control"]["prepared"]["uuid"],"turn":"admitted-turn"});
    record["control"]["certainty"] = json!("observed");
    let mut bytes = serde_json::to_vec(&record)?;
    bytes.push(b'\n');
    std::fs::write(&path, bytes)?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    assert_eq!(
        sessions.outcome("delivery")?.state,
        OperationState::Confirmed
    );
    assert_eq!(receipt(&sessions, dir.path())?["admitted"], true);
    assert!(sessions.status(None)?.sessions.is_empty());
    Ok(())
}

#[test]
fn a_second_restart_keeps_possible_delivery_uncertain_without_replaying() -> TestResult {
    let dir = prepared("possibly_sent")?;
    drop(Sessions::open(dir.path(), 4096)?);
    let path = dir.path().join("operations.v2.journal");
    let before = std::fs::read(&path)?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    assert_eq!(
        sessions.outcome("delivery")?.state,
        OperationState::Uncertain
    );
    assert_eq!(
        std::fs::read(path)?,
        before,
        "restart minted a recovery or replay operation"
    );
    assert!(sessions.status(None)?.sessions.is_empty());
    Ok(())
}

#[test]
fn a_person_cannot_attest_reconciliation_as_the_service() -> TestResult {
    let dir = prepared("possibly_sent")?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("key"))?;
    let greeting = lys_runner::protocol::Greeting::fresh("21");
    let act: Act = serde_json::from_value(
        json!({"act":"as_caller","caller":"unrelated-person","done":{
        "act":"reconcile_control","operation":"delivery","decision":{"operation":"decision","by":"responsible","at":0,"decision":{"choice":"seen"}}}}),
    )?;
    let request = lys_runner::protocol::sign_request(&key, &greeting, &act)?;
    let answer = lys_runner::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &request,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(
        matches!(answer, lys_runner::Answer::Refused { .. }),
        "{answer:?}"
    );
    assert!(receipt(&sessions, dir.path())?["reconciled"].is_null());
    Ok(())
}

#[test]
fn a_legacy_uncertain_older_than_a_day_still_offers_the_persons_decision() -> TestResult {
    let dir = tempfile::tempdir()?;
    let outcome = json!({"operation":"delivery","session":"session","request":"reminder",
        "state":"uncertain","at":0,"words":"old possible delivery","text":null,"ended":null});
    let mut bytes = serde_json::to_vec(&outcome)?;
    bytes.push(b'\n');
    std::fs::write(dir.path().join("operations.jsonl"), bytes)?;
    drop(Sessions::open(dir.path(), 4096)?);
    let sessions = Sessions::open(dir.path(), 4096)?;
    let original = receipt(&sessions, dir.path())?;
    assert_eq!(original["certainty"], "possibly_sent");
    assert_eq!(original["prepared"], false);
    assert_eq!(
        sessions.outcome("delivery")?.state,
        OperationState::Uncertain
    );
    let key = Ed25519Identity::load_or_generate(&dir.path().join("key"))?;
    let greeting = lys_runner::protocol::Greeting::fresh("21");
    let at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let act: Act = serde_json::from_value(
        json!({"act":"reconcile_control","operation":"delivery",
        "decision":{"operation":"personal-decision","by":"responsible-person","at":at,"decision":{"choice":"not_seen"}}}),
    )?;
    let line = lys_runner::protocol::sign_request(&key, &greeting, &act)?;
    let answer = lys_runner::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &line,
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(
        matches!(answer, lys_runner::Answer::ControlReceipt { .. }),
        "{answer:?}"
    );
    let current = receipt(&sessions, dir.path())?;
    assert_eq!(current["reconciled"]["by"], "responsible-person");
    assert_eq!(current["admitted"], false);
    assert_eq!(current["state"], "uncertain");
    assert!(sessions.status(None)?.sessions.is_empty());
    Ok(())
}

#[test]
fn an_original_ask_reads_the_current_prepared_receipt_but_changed_words_are_refused() -> TestResult
{
    use lys_runner::operations::{Operation, OperationRequest};
    let dir = prepared("possibly_sent")?;
    let path = dir.path().join("operations.v2.journal");
    let mut record: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    record["control"]["original_text"] = serde_json::to_value(TextDigest::of("original ask"))?;
    let mut bytes = serde_json::to_vec(&record)?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let reference = serde_json::from_value(record["control"]["prepared"]["reference"].clone())?;
    let mut operation = Operation {
        operation: "delivery".to_owned(),
        session: "session".to_owned(),
        request: OperationRequest::GoalReminder {
            text: "original ask".to_owned(),
            reference,
        },
    };
    assert_eq!(
        sessions.operate(operation.clone())?,
        sessions.outcome("delivery")?
    );
    if let OperationRequest::GoalReminder { text, .. } = &mut operation.request {
        *text = "changed ask".to_owned();
    }
    let error = sessions
        .operate(operation)
        .err()
        .ok_or("changed words reused a prepared operation")?;
    assert!(error.to_string().contains("operation_reused"), "{error}");
    assert!(sessions.status(None)?.sessions.is_empty());
    Ok(())
}
