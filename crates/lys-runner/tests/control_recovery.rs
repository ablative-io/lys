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

fn ask(
    sessions: &std::sync::Arc<Sessions>,
    directory: &std::path::Path,
    value: Value,
) -> Result<Value, Box<dyn Error>> {
    let act: Act = serde_json::from_value(value)?;
    let key = Ed25519Identity::load_or_generate(&directory.join("key"))?;
    let greeting = lys_runner::protocol::Greeting::fresh("21");
    let line = lys_runner::protocol::sign_request(&key, &greeting, &act)?;
    let answer = lys_runner::socket::dispatch(
        sessions,
        &key.public_key_bytes(),
        &greeting,
        &line,
        &std::sync::atomic::AtomicBool::new(false),
    );
    Ok(serde_json::to_value(answer)?)
}

#[test]
fn current_control_pages_keep_every_operation_across_the_existing_page_bound() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let count = lys_runner::tracking_store::PAGE_MAX * 2 + 1;
    let mut bytes = Vec::new();
    for number in 0..count {
        let outcome = json!({"operation":format!("operation-{number:06}"),"session":"session","request":"reminder",
            "state":"uncertain","at":at,"words":"private goal words","text":null,"ended":null});
        serde_json::to_writer(&mut bytes, &outcome)?;
        bytes.push(b'\n');
    }
    std::fs::write(dir.path().join("operations.jsonl"), bytes)?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let mut after: Option<String> = None;
    let mut ids = Vec::new();
    loop {
        let answer = ask(
            &sessions,
            dir.path(),
            json!({"act":"control_receipts","session":"session","after":after}),
        )?;
        assert_eq!(answer["kind"], "control_receipts", "{answer}");
        let page = &answer["page"];
        assert_eq!(page["session"], "session");
        let entries = page["receipts"].as_array().ok_or("page has no receipts")?;
        assert!(entries.len() <= lys_runner::tracking_store::PAGE_MAX);
        for entry in entries {
            ids.push(
                entry["operation"]
                    .as_str()
                    .ok_or("receipt has no identity")?
                    .to_owned(),
            );
            assert!(entry.get("words").is_none());
            assert!(entry.get("frame").is_none());
            assert_eq!(entry["state"], "uncertain");
            assert_eq!(entry["prepared"], false);
        }
        assert!(!serde_json::to_string(page)?.contains("private goal words"));
        after = page["after"].as_str().map(str::to_owned);
        if after.is_none() {
            break;
        }
    }
    assert_eq!(
        ids,
        (0..count)
            .map(|number| format!("operation-{number:06}"))
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn a_known_empty_session_has_an_empty_page_and_an_unknown_session_is_named() -> TestResult {
    use lys_runner::state::{Kept, KeptSession, StateFile};
    let dir = tempfile::tempdir()?;
    {
        let file = StateFile::open(dir.path())?;
        file.write(&Kept::new(vec![KeptSession {
            session: "empty".to_owned(),
            pid: None,
            leader_start: None,
            started_at: 0,
            columns: 80,
            rows: 24,
            ended: Some(lys_runner::Ended {
                how: lys_runner::EndedHow::Exited,
                at: 0,
                status: Some(0),
                signal: None,
                reason: None,
                stopped: None,
            }),
        }]))?;
    }
    let sessions = Sessions::open(dir.path(), 4096)?;
    let answer = ask(
        &sessions,
        dir.path(),
        json!({"act":"control_receipts","session":"empty","after":null}),
    )?;
    assert_eq!(answer["kind"], "control_receipts");
    assert_eq!(answer["page"]["session"], "empty");
    assert_eq!(answer["page"]["receipts"], json!([]));
    assert!(answer["page"]["after"].is_null());
    let answer = ask(
        &sessions,
        dir.path(),
        json!({"act":"control_receipts","session":"unknown","after":null}),
    )?;
    assert_eq!(answer["refusal"], "session_unknown");
    Ok(())
}

#[test]
fn an_unreconciled_boundary_reports_unknown_phase_without_guessing() -> TestResult {
    let dir = prepared("possibly_sent")?;
    let record: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("operations.v2.journal"))?)?;
    let binding = serde_json::from_value(record["control"]["prepared"]["binding"].clone())?;
    let controller = lys_runner::harness_control::Controller::new(
        binding,
        lys_runner::harness_control::Transport::Claude,
    )?;
    let status = serde_json::to_value(controller.control_status())?;
    assert_eq!(status["phase"], "unknown");
    assert!(status["active"].is_null());
    assert!(status.get("threshold").is_none());
    assert!(status.get("words").is_none());
    Ok(())
}

#[test]
fn an_uncertain_without_control_metadata_does_not_expire_before_a_person_decides() -> TestResult {
    let dir = tempfile::tempdir()?;
    let outcome = json!({"operation":"delivery","session":"session","request":"reminder",
        "state":"uncertain","at":0,"words":"delivery evidence missing","text":null,"ended":null});
    let mut bytes = serde_json::to_vec(&json!({"outcome":outcome,"control":null}))?;
    bytes.push(b'\n');
    std::fs::write(dir.path().join("operations.v2.journal"), bytes)?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    assert_eq!(
        sessions.outcome("delivery")?.state,
        OperationState::Uncertain
    );
    let at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let answer = ask(
        &sessions,
        dir.path(),
        json!({"act":"reconcile_control","operation":"delivery",
        "decision":{"operation":"manual-decision","by":"responsible-person","at":at,"decision":{"choice":"seen"}}}),
    )?;
    assert_eq!(answer["kind"], "control_receipt", "{answer}");
    assert_eq!(answer["receipt"]["prepared"], false);
    assert_eq!(answer["receipt"]["state"], "uncertain");
    assert_eq!(answer["receipt"]["admitted"], false);
    Ok(())
}

#[test]
fn runner_answers_keep_their_existing_wire_bytes() -> TestResult {
    use lys_runner::protocol::{Answer, Reply};
    let delivered = Reply {
        version: 1,
        answer: Answer::Delivered {
            session: "session".to_owned(),
        },
    };
    assert_eq!(
        serde_json::to_string(&delivered)?,
        r#"{"version":1,"answer":{"kind":"delivered","session":"session"}}"#
    );
    let refusal = Reply {
        version: 1,
        answer: Answer::Refused {
            refusal: "session_unknown".to_owned(),
            words: "the session is not held".to_owned(),
            oldest: None,
        },
    };
    assert_eq!(
        serde_json::to_string(&refusal)?,
        r#"{"version":1,"answer":{"kind":"refused","refusal":"session_unknown","words":"the session is not held"}}"#
    );
    let expired = Reply {
        version: 1,
        answer: Answer::Refused {
            refusal: "cursor_expired".to_owned(),
            words: "the requested output is no longer held".to_owned(),
            oldest: Some(42),
        },
    };
    assert_eq!(
        serde_json::to_string(&expired)?,
        r#"{"version":1,"answer":{"kind":"refused","refusal":"cursor_expired","words":"the requested output is no longer held","oldest":42}}"#
    );
    Ok(())
}
