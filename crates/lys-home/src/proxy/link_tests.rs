#![cfg(test)]
//! Gates on linking: a call is linked by the key its own body carries, read
//! as it passes with bounded memory, and by nothing else; a call without
//! one lands under the day's `unlinked` session whatever came before it.

use hyper::StatusCode;
use serde_json::json;

use crate::proxy::forward_tests::{
    Harness, KEY, Res, fake, message_response, messages_body, messages_request, send, whole,
};
use crate::proxy::link::{KeyScanner, Link, METADATA_BUDGET, session_key};
use crate::record::call::CallStatus;

fn scan(body: &[u8], step: usize) -> Link {
    let mut scanner = KeyScanner::new();
    for chunk in body.chunks(step) {
        scanner.feed(chunk);
    }
    scanner.link()
}

#[test]
fn the_top_level_metadata_key_is_read_at_any_split() {
    let body = messages_body(Some(KEY), false);
    let mut splits = 0;
    for step in [1, 2, 5, 13, body.len()] {
        assert_eq!(scan(&body, step), Link::Session(KEY.to_owned()));
        splits += 1;
    }
    assert_eq!(splits, 5);
}

#[test]
fn a_metadata_key_inside_a_message_links_nothing() {
    let user_id = format!("user_{}_account_a1_session_{KEY}", "0".repeat(64));
    let body = json!({
        "model": "m",
        "messages": [{"role": "user", "content": "x", "metadata": {"user_id": user_id}}],
        "note": "\"metadata\": {\"user_id\": \"user_session_abc\"}"
    });
    assert_eq!(scan(body.to_string().as_bytes(), 3), Link::Unlinked);
}

#[test]
fn both_spellings_of_the_key_are_read_and_an_unsafe_one_links_nothing() {
    let object = json!({"device_id": "d", "session_id": KEY}).to_string();
    assert_eq!(session_key(&object), Some(KEY.to_owned()));
    let body = json!({"metadata": {"user_id": object}, "messages": []});
    assert_eq!(
        scan(body.to_string().as_bytes(), 7),
        Link::Session(KEY.to_owned())
    );
    assert_eq!(session_key("user_ab_account_cd_session_../x"), None);
    assert_eq!(session_key("user_ab_account_cd"), None);
    assert_eq!(session_key("{\"session_id\": \"a/b\"}"), None);
}

#[test]
fn a_metadata_value_over_the_budget_links_nothing() {
    let padding = "p".repeat(METADATA_BUDGET);
    let user_id = format!("user_{padding}_account_a1_session_{KEY}");
    let body = json!({"metadata": {"user_id": user_id}});
    assert_eq!(scan(body.to_string().as_bytes(), 64), Link::Unlinked);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_call_without_a_key_lands_under_unlinked_and_the_report_says_so() -> Res {
    let (upstream, _) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream, 4).await?;
    let (response, _connection) = send(harness.addr, messages_request(None, false)?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let report = harness.report()?;
    assert!(!report.linked);
    assert!(report.session.starts_with("unlinked-"));
    assert!(serde_json::to_string(&report)?.contains("\"session\":\"unlinked-"));
    assert_eq!(harness.calls(&report.session)?.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_unkeyed_calls_after_a_keyed_one_never_land_under_its_session() -> Res {
    let (upstream, _) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream, 4).await?;
    let mut sessions = Vec::new();
    for key in [Some(KEY), None, None] {
        let (response, _connection) = send(harness.addr, messages_request(key, false)?).await?;
        assert_eq!(response.status(), StatusCode::OK);
        sessions.push(harness.report()?.session);
    }
    assert_eq!(sessions[0], KEY);
    assert!(sessions[1].starts_with("unlinked-"));
    assert_eq!(sessions[1], sessions[2]);
    assert_eq!(harness.calls(KEY)?.len(), 1);
    assert_eq!(harness.calls(&sessions[1])?.len(), 2);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn with_no_capture_slot_a_keyed_call_is_unrecorded_under_its_own_session() -> Res {
    let (upstream, _) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream, 0).await?;
    let (response, _connection) = send(harness.addr, messages_request(Some(KEY), false)?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Unrecorded);
    assert_eq!(report.session, KEY);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Unrecorded);
    assert!(calls[0].raw_request.is_none());
    assert_eq!(harness.home()?.session_ids()?, vec![KEY.to_owned()]);
    Ok(())
}
