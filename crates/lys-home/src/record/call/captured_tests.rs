use super::*;
use crate::record::{Home, call::Api};

#[test]
fn prepared_stream_parts_survive_without_a_second_body_parse()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let request = dir.path().join("request");
    let response = dir.path().join("response");
    std::fs::write(
        &request,
        br#"{"model":"m","messages":[{"role":"user","content":"hello"}]}"#,
    )?;
    std::fs::write(&response, b"not JSON: parsed stream is supplied separately")?;
    let meta = OutcomeMeta {
        call_id: "c".into(),
        provider: "p".into(),
        api: Api::Messages,
        status: CallStatus::Complete,
        started_at: "2000-01-01T00:00:00Z".into(),
        duration_ms: Some(1),
        stream: true,
    };
    let parts = vec![serde_json::json!({"type":"text", "text":"answer"})];
    let ready = PreparedCall::prepare(
        &home.blocks()?,
        Captured {
            meta: &meta,
            request: Some(&request),
            response: Some(&response),
            response_parts: Some(&parts),
            raw_request: None,
            raw_response: None,
            timing: CaptureTiming::interrupted(Some(42)),
            seen: &Seen {
                message_id: Some("msg_1".to_owned()),
                tokens: None,
                head: Head::default(),
                unrecorded: None,
            },
        },
    )?;
    std::fs::remove_file(&request)?;
    std::fs::remove_file(&response)?;
    let restored: PreparedCall = serde_json::from_slice(&serde_json::to_vec(&ready)?)?;
    let mut session = home.create_session("captured", "", None)?;
    let report = restored.append(&mut session)?;
    assert!(!report.already_recorded);
    assert_eq!(report.request_parts, 1);
    assert_eq!(report.response_parts, 1);
    assert_eq!(restored.record.status, CallStatus::Complete);
    assert_eq!(
        restored.record.message_id.as_deref(),
        Some("msg_1"),
        "the message's id is on the record and survives the journal"
    );
    let entry = serde_json::to_value(&restored.record)?;
    assert_eq!(entry["message_id"], "msg_1");
    // A call nothing was seen of names no head at all, not an empty one.
    assert!(entry.get("head").is_none());
    assert!(entry.get("request_id").is_none());
    assert!(restored.append(&mut session)?.already_recorded);
    Ok(())
}

/// A complete call whose stored bodies are `request` and `response`, not an
/// event stream, prepared with what was `seen`.
fn prepared(
    request: &[u8],
    response: &[u8],
    status: CallStatus,
    seen: &Seen,
) -> Result<CallRecord, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let (request_file, response_file) = (dir.path().join("request"), dir.path().join("response"));
    std::fs::write(&request_file, request)?;
    std::fs::write(&response_file, response)?;
    let meta = OutcomeMeta {
        call_id: "c".into(),
        provider: "p".into(),
        api: Api::Messages,
        status,
        started_at: "2000-01-01T00:00:00Z".into(),
        duration_ms: Some(1),
        stream: false,
    };
    let ready = PreparedCall::prepare(
        &home.blocks()?,
        Captured {
            meta: &meta,
            request: Some(&request_file),
            response: Some(&response_file),
            response_parts: None,
            raw_request: None,
            raw_response: None,
            timing: CaptureTiming::interrupted(None),
            seen,
        },
    )?;
    Ok(ready.record)
}

const REQUEST: &[u8] = br#"{"model":"m","messages":[{"role":"user","content":"hello"}]}"#;
const RESPONSE: &[u8] =
    br#"{"type":"message","role":"assistant","content":[{"type":"text","text":"phi"}]}"#;

fn answered(coding: Option<&str>) -> Seen {
    let mut seen = Seen::default();
    seen.head.status = Some(200);
    if let Some(coding) = coding {
        seen.head
            .response
            .values
            .insert("content-encoding".to_owned(), vec![coding.to_owned()]);
    }
    seen
}

#[test]
fn an_unrecorded_call_names_its_reason_and_the_encoding_the_response_named()
-> Result<(), Box<dyn std::error::Error>> {
    // The read of 3 October: a whole response that is not JSON as stored.
    let encoded = prepared(
        REQUEST,
        &[0x83, 0x38, 0, 0],
        CallStatus::Complete,
        &answered(Some("br")),
    )?;
    assert_eq!(encoded.status, CallStatus::Unrecorded);
    let reason = encoded.unrecorded_reason.ok_or("no reason")?;
    assert!(reason.starts_with("the response is not JSON: "), "{reason}");
    assert!(
        reason.contains("the response's content-encoding is br"),
        "{reason}"
    );

    let plain = prepared(REQUEST, b"not JSON", CallStatus::Complete, &answered(None))?;
    let reason = plain.unrecorded_reason.ok_or("no reason")?;
    assert!(
        reason.ends_with("the response names no content-encoding"),
        "{reason}"
    );

    // A request that cannot be read: the reason is that, and says nothing of encodings.
    let garbled = prepared(
        b"not JSON",
        RESPONSE,
        CallStatus::Complete,
        &answered(Some("br")),
    )?;
    assert_eq!(garbled.status, CallStatus::Unrecorded);
    let reason = garbled.unrecorded_reason.ok_or("no reason")?;
    assert!(reason.contains("needs both bodies and a model"), "{reason}");
    assert!(!reason.contains("content-encoding"), "{reason}");
    Ok(())
}

#[test]
fn the_reason_is_the_capture_steps_when_it_marked_the_call_and_absent_otherwise()
-> Result<(), Box<dyn std::error::Error>> {
    let mut seen = answered(None);
    seen.unrecorded = Some("a capture spool could not be created or written".to_owned());
    let marked = prepared(REQUEST, RESPONSE, CallStatus::Unrecorded, &seen)?;
    assert_eq!(marked.status, CallStatus::Unrecorded);
    assert_eq!(marked.unrecorded_reason, seen.unrecorded);

    let complete = prepared(REQUEST, RESPONSE, CallStatus::Complete, &answered(None))?;
    assert_eq!(complete.status, CallStatus::Complete);
    assert_eq!(complete.unrecorded_reason, None);
    // A reason never rides on a call that is not unrecorded.
    let partial = prepared(REQUEST, RESPONSE, CallStatus::Partial, &seen)?;
    assert_eq!(partial.unrecorded_reason, None);
    Ok(())
}
