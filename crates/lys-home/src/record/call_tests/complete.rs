#![cfg(test)]
//! A complete call never hides a malformed, missing or partless response,
//! from files or from bytes.

use serde_json::json;

use crate::record::Home;
use crate::record::call::{
    Api, CallStatus, OutcomeMeta, ingest_call, ingest_call_files, ingest_outcome,
};
use crate::record::entries::CUSTOM_CALL;

use super::meta;

#[test]
fn a_complete_call_never_hides_a_malformed_or_missing_response() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("c6", "/w", None).unwrap();
    let rf = dir.path().join("req.json");
    std::fs::write(
        &rf,
        serde_json::to_vec(&json!({"model":"m","messages":[{"role":"user","content":"u"}]}))
            .unwrap(),
    )
    .unwrap();
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, b"{not json").unwrap();
    let mut m = meta(Api::Messages, "bad-resp");
    m.stream = false;
    let err = ingest_call_files(&mut s, &blocks, &m, &rf, &bad, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not JSON"), "{err}");
    m.stream = true;
    let err = ingest_call_files(&mut s, &blocks, &m, &rf, &bad, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("assembled parts"), "{err}");
    assert_eq!(s.customs_everywhere(CUSTOM_CALL).unwrap().len(), 0);
    // An outcome whose request file cannot be read is an error, not an absence.
    let missing = dir.path().join("nope.json");
    let err = ingest_outcome(
        &mut s,
        &blocks,
        &OutcomeMeta {
            call_id: "io-1".into(),
            provider: "anthropic".into(),
            api: Api::Messages,
            status: CallStatus::Lost,
            started_at: "t".into(),
            duration_ms: None,
            stream: false,
        },
        Some(&missing),
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("nope.json"), "{err}");
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_complete_call_from_bytes_refuses_a_malformed_or_partless_response_and_a_stream_without_parts()
{
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("c7", "/w", None).unwrap();
    let req = serde_json::to_vec(&json!({"model":"m","messages":[{"role":"user","content":"u"}]}))
        .unwrap();
    let mut m = meta(Api::Messages, "bytes-bad");
    m.stream = false;
    let err = ingest_call(&mut s, &blocks, &m, &req, b"{not json", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not JSON"), "{err}");
    let err = ingest_call(&mut s, &blocks, &m, &req, b"{}", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("content is not an array"), "{err}");
    let err = ingest_call(&mut s, &blocks, &m, &req, b"{\"content\":null}", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("content is not an array"), "{err}");
    let mut chat = meta(Api::ChatCompletions, "chat-bad");
    chat.stream = false;
    let chat_req =
        serde_json::to_vec(&json!({"model":"m","messages":[{"role":"user","content":"u"}]}))
            .unwrap();
    let err = ingest_call(
        &mut s,
        &blocks,
        &chat,
        &chat_req,
        b"{\"choices\":[{}]}",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("without a message"), "{err}");
    let err = ingest_call(&mut s, &blocks, &chat, &chat_req, b"{\"choices\":[]}", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no choices"), "{err}");
    let mut resp = meta(Api::Responses, "resp-bad");
    resp.stream = false;
    let resp_req = serde_json::to_vec(&json!({"model":"m","input":"u"})).unwrap();
    let err = ingest_call(
        &mut s,
        &blocks,
        &resp,
        &resp_req,
        b"{\"status\":\"failed\",\"output\":[]}",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("not completed"), "{err}");
    let err = ingest_call(
        &mut s,
        &blocks,
        &m,
        &req,
        b"{\"type\":\"error\",\"error\":{\"type\":\"x\"}}",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("error body"), "{err}");
    m.stream = true;
    let err = ingest_call(&mut s, &blocks, &m, &req, b"event: x\n", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("assembled parts"), "{err}");
    assert_eq!(s.customs_everywhere(CUSTOM_CALL).unwrap().len(), 0);
    // The proxy's assembled parts stand in for the stream body.
    let r = ingest_call(
        &mut s,
        &blocks,
        &m,
        &req,
        b"event: x\n",
        Some(vec![json!({"type":"text","text":"a"})]),
    )
    .unwrap();
    assert_eq!((r.response_parts, r.already_recorded), (1, false));
    // An outcome whose request file is a directory is an I/O error, not an absence.
    let dirpath = dir.path().join("a-directory");
    std::fs::create_dir(&dirpath).unwrap();
    let err = ingest_outcome(
        &mut s,
        &blocks,
        &OutcomeMeta {
            call_id: "io-2".into(),
            provider: "anthropic".into(),
            api: Api::Messages,
            status: CallStatus::Partial,
            started_at: "t".into(),
            duration_ms: None,
            stream: true,
        },
        Some(&dirpath),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, crate::error::HomeError::Io { .. }), "{err}");
    // A request file that is not JSON is recorded as absence.
    let half = dir.path().join("half.json");
    std::fs::write(&half, b"{\"model\":\"m\",\"mess").unwrap();
    let r = ingest_outcome(
        &mut s,
        &blocks,
        &OutcomeMeta {
            call_id: "io-3".into(),
            provider: "anthropic".into(),
            api: Api::Messages,
            status: CallStatus::Lost,
            started_at: "t".into(),
            duration_ms: None,
            stream: true,
        },
        Some(&half),
        None,
    )
    .unwrap();
    assert_eq!((r.request_parts, r.response_parts, r.raw_blocks), (0, 0, 1));
    drop(s);
    dir.close().unwrap();
}
