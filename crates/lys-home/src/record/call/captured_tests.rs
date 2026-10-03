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
    assert!(restored.append(&mut session)?.already_recorded);
    Ok(())
}
