use super::*;
use crate::proxy::journal::{OpenCall, recover};
use crate::record::call::{Api, call_record};
use crate::record::entries::CUSTOM_CALL;

#[test]
fn a_crash_after_body_rename_recovers_complete_without_reparsing()
-> Result<(), Box<dyn std::error::Error>> {
    for checkpoint in 0..3 {
        recover_at(checkpoint)?;
    }
    Ok(())
}

fn recover_at(checkpoint: u8) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let journal = Journal::open(dir.path().join("journal"))?;
    let capture = dir.path().join("capture");
    std::fs::create_dir(&capture)?;
    let request = capture.join("c.request");
    let response = capture.join("c.response");
    let req = br#"{"model":"m","messages":[{"role":"user","content":"hello"}]}"#;
    let resp = b"stream already parsed, not JSON";
    std::fs::write(&request, req)?;
    std::fs::write(&response, resp)?;
    for path in [&request, &response] {
        std::fs::File::open(path)?.sync_all()?;
    }
    let blocks = home.blocks()?;
    let req_hash = Hash::of(req);
    let resp_hash = Hash::of(resp);
    let meta = OutcomeMeta {
        call_id: "c".into(),
        provider: "p".into(),
        api: Api::Messages,
        status: CallStatus::Complete,
        started_at: "2000-01-01T00:00:00Z".into(),
        duration_ms: Some(1),
        stream: true,
    };
    let parts = vec![serde_json::json!({"type":"text", "text":"whole"})];
    let completed = PreparedCall::prepare(
        &blocks,
        Captured {
            meta: &meta,
            request: Some(&request),
            response: Some(&response),
            response_parts: Some(&parts),
            raw_request: Some(req_hash.to_string()),
            raw_response: Some(resp_hash.to_string()),
            timing: CaptureTiming {
                admission_ns: Some(81),
                durable: DurableTime::Interrupted,
            },
        },
    )?;
    let mut job = Job {
        call: OpenCall {
            call_id: "c".into(),
            provider: "p".into(),
            api: Api::Messages,
            started_at: "2000-01-01T00:00:00Z".into(),
            session: None,
            admission_ns: Some(81),
            completed: Some(completed),
        },
        status: CallStatus::Complete,
        duration_ms: 1,
        stream: true,
        request: Some(request.clone()),
        response: Some(response.clone()),
        request_hash: Some(req_hash),
        response_hash: Some(resp_hash),
        parts: Some(parts),
        last_arrival: Some(std::time::Instant::now()),
    };
    journal.write(&job.call)?;
    if checkpoint == 0 {
        blocks.admit_spool(&request, &Hash::of(req))?;
        assert!(!request.exists());
        assert!(response.exists());
    } else {
        install(&blocks, &journal, &mut job)?;
        assert!(!request.exists());
        assert!(!response.exists());
        if checkpoint == 2 {
            journal.write(&job.call)?;
        }
    }
    // Only the intent is durable: the measured duration and call append were interrupted.
    drop(job);
    let reports = recover(&home, &journal, &capture)?;
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, CallStatus::Complete);
    assert!(reports[0].retired);
    let session = home.open_session("unlinked-2000-01-01")?;
    let calls = session.customs_everywhere(CUSTOM_CALL)?;
    let crate::record::entries::EntryBody::Custom {
        data: Some(data), ..
    } = &calls[0].body
    else {
        return Err("call data absent".into());
    };
    let record = call_record(data)?;
    assert_eq!(record.status, CallStatus::Complete);
    let durable = &record
        .capture
        .as_ref()
        .ok_or("capture timing absent")?
        .durable;
    if checkpoint == 2 {
        assert!(matches!(durable, DurableTime::Measured(_)));
    } else {
        assert_eq!(durable, &DurableTime::Interrupted);
    }
    assert_eq!(
        blocks.get(&Hash::parse(
            record.raw_response.as_deref().ok_or("raw body absent")?
        )?)?,
        resp
    );
    assert!(recover(&home, &journal, &capture)?.is_empty());
    Ok(())
}
