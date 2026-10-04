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
                run: None,
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
    // A coding that is not decoded: the stored bytes are read as they are,
    // and the reason says which coding the response named.
    let encoded = prepared(
        REQUEST,
        &[0x83, 0x38, 0, 0],
        CallStatus::Complete,
        &answered(Some("zstd")),
    )?;
    assert_eq!(encoded.status, CallStatus::Unrecorded);
    let reason = encoded.unrecorded_reason.ok_or("no reason")?;
    assert!(reason.starts_with("the response is not JSON: "), "{reason}");
    assert!(
        reason.contains("the response's content-encoding is zstd, which is not decoded"),
        "{reason}"
    );
    // Two codings are not guessed at either.
    let twice = prepared(
        REQUEST,
        &encoded_as("gzip", RESPONSE)?,
        CallStatus::Complete,
        &answered(Some("gzip, br")),
    )?;
    let reason = twice.unrecorded_reason.ok_or("no reason")?;
    assert!(
        reason.contains("gzip, br, which is not decoded"),
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

/// `body` under the content coding `coding`.
fn encoded_as(coding: &str, body: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use flate2::Compression;
    use std::io::Write;
    match coding {
        "gzip" => {
            let mut encoder = flate2::write::GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(body)?;
            Ok(encoder.finish()?)
        }
        "deflate" => {
            let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(body)?;
            Ok(encoder.finish()?)
        }
        "br" => {
            let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 5, 22);
            encoder.write_all(body)?;
            Ok(encoder.into_inner())
        }
        _ => Err("fixture coding is not one of the three".into()),
    }
}

#[test]
fn a_response_that_is_not_an_event_stream_is_decoded_from_the_coding_it_names()
-> Result<(), Box<dyn std::error::Error>> {
    const REFUSED: &[u8] =
        br#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;
    for coding in ["gzip", "deflate", "br", "BR", " gzip "] {
        let named = coding.trim().to_ascii_lowercase();
        // A whole response reads as it would have read unencoded.
        let whole = prepared(
            REQUEST,
            &encoded_as(&named, RESPONSE)?,
            CallStatus::Complete,
            &answered(Some(coding)),
        )?;
        assert_eq!(whole.status, CallStatus::Complete, "{coding}");
        assert_eq!(whole.unrecorded_reason, None, "{coding}");
        assert_eq!(whole.response.len(), 1, "{coding}");
        // A provider's refusal is read as what it is: an error body, not
        // bytes that are not JSON.
        let refused = prepared(
            REQUEST,
            &encoded_as(&named, REFUSED)?,
            CallStatus::Complete,
            &answered(Some(coding)),
        )?;
        assert_eq!(refused.status, CallStatus::Unrecorded, "{coding}");
        let reason = refused.unrecorded_reason.ok_or("no reason")?;
        assert!(
            reason.ends_with("an error body is not a complete response"),
            "{reason}"
        );
        // Bytes that are not the coding the response named: said, with the
        // coding, and never read as something else.
        let wrong = prepared(
            REQUEST,
            RESPONSE,
            CallStatus::Complete,
            &answered(Some(coding)),
        )?;
        assert_eq!(wrong.status, CallStatus::Unrecorded, "{coding}");
        let reason = wrong.unrecorded_reason.ok_or("no reason")?;
        assert!(
            reason.starts_with(&format!(
                "the response's content-encoding is {named}, and its stored bytes do not decode as that: "
            )),
            "{reason}"
        );
        // Decoded bytes that are not JSON say the coding was decoded.
        let text = prepared(
            REQUEST,
            &encoded_as(&named, b"not JSON")?,
            CallStatus::Complete,
            &answered(Some(coding)),
        )?;
        let reason = text.unrecorded_reason.ok_or("no reason")?;
        assert!(
            reason.ends_with(&format!(", after its {named} coding was decoded")),
            "{reason}"
        );
    }
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
