#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the call record: parts once by hash, raw bodies counted apart,
//! idempotent on the call id, no response parts unless complete, absence
//! recorded as absence, no header ever in the record.

use serde_json::json;

use crate::record::Home;
use crate::record::call::{
    Api, CallMeta, CallStatus, OutcomeMeta, call_record, ingest_call, ingest_call_files,
    ingest_outcome,
};
use crate::record::entries::{CUSTOM_CALL, EntryBody};

fn meta(api: Api, call_id: &str) -> CallMeta {
    CallMeta {
        call_id: call_id.into(),
        provider: "anthropic".into(),
        api,
        model: "m".into(),
        started_at: "2026-09-24T03:00:00Z".into(),
        duration_ms: 12,
        stream: false,
    }
}

#[test]
fn a_resent_conversation_adds_only_the_new_turn_as_part_blocks_and_raw_bodies_apart() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path()).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("c1", "/w", None).unwrap();
    let first = json!({"model":"m","system":"be brief","messages":[{"role":"user","content":[{"type":"text","text":"hi"}]}]});
    let reply1 = json!({"content":[{"type":"text","text":"hello"}]});
    let r1 = ingest_call(
        &mut s,
        &blocks,
        &meta(Api::Messages, "call-1"),
        &serde_json::to_vec(&first).unwrap(),
        &serde_json::to_vec(&reply1).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!((r1.request_parts, r1.response_parts), (2, 1));
    assert_eq!(
        (r1.part_blocks_new, r1.part_blocks_reused, r1.raw_blocks),
        (3, 0, 2)
    );
    let second = json!({"model":"m","system":"be brief","messages":[
        {"role":"user","content":[{"type":"text","text":"hi"}]},
        {"role":"assistant","content":[{"type":"text","text":"hello"}]},
        {"role":"user","content":[{"type":"text","text":"and?"}]}]});
    let reply2 = json!({"content":[{"type":"text","text":"more"}]});
    let r2 = ingest_call(
        &mut s,
        &blocks,
        &meta(Api::Messages, "call-2"),
        &serde_json::to_vec(&second).unwrap(),
        &serde_json::to_vec(&reply2).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(r2.request_parts, 4);
    assert_eq!(
        (r2.part_blocks_new, r2.part_blocks_reused, r2.raw_blocks),
        (2, 3, 2)
    );
    let calls = s.customs(CUSTOM_CALL).unwrap();
    assert_eq!(calls.len(), 2);
    let EntryBody::Custom {
        data: Some(data), ..
    } = &calls[1].body
    else {
        panic!("not a custom entry")
    };
    let record = call_record(data).unwrap();
    assert_eq!(record.request.len(), 4);
    assert_eq!(record.status, CallStatus::Complete);
    let raw = blocks
        .get(&crate::record::call::named_hash(record.raw_request.as_deref().unwrap()).unwrap())
        .unwrap();
    assert_eq!(raw, serde_json::to_vec(&second).unwrap());
    let text = serde_json::to_string(data).unwrap().to_lowercase();
    for forbidden in ["authorization", "cookie", "x-api-key"] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    drop(s);
    dir.close().unwrap();
}

#[test]
fn chat_completions_and_responses_bodies_split_into_their_items() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path()).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("c2", "/w", None).unwrap();
    let chat = json!({"model":"m","messages":[{"role":"system","content":"s"},{"role":"user","content":"u"},{"role":"assistant","content":"a"}]});
    let chat_reply = json!({"choices":[{"message":{"role":"assistant","content":"r"}}]});
    let r = ingest_call(
        &mut s,
        &blocks,
        &meta(Api::ChatCompletions, "chat-1"),
        &serde_json::to_vec(&chat).unwrap(),
        &serde_json::to_vec(&chat_reply).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!((r.request_parts, r.response_parts), (3, 1));
    let responses = json!({"model":"m","instructions":"i","input":[{"role":"user","content":"u"},{"type":"reasoning","encrypted_content":"x"}]});
    let responses_reply = json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"t"}]}]});
    let r = ingest_call(
        &mut s,
        &blocks,
        &meta(Api::Responses, "resp-1"),
        &serde_json::to_vec(&responses).unwrap(),
        &serde_json::to_vec(&responses_reply).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!((r.request_parts, r.response_parts), (3, 1));
    let err = ingest_call(
        &mut s,
        &blocks,
        &meta(Api::Messages, "bad-1"),
        b"{\"model\":\"m\"}",
        b"",
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("no messages array"), "{err}");
    drop(s);
    dir.close().unwrap();
}

#[test]
fn ingesting_from_files_streams_the_raw_bodies_and_a_repeated_call_id_records_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("c3", "/w", None).unwrap();
    let req = json!({"model":"m","messages":[{"role":"user","content":"u"}]});
    let big = "x".repeat(3 << 20);
    let resp = json!({"content":[{"type":"text","text": big}]});
    let rf = dir.path().join("req.json");
    let pf = dir.path().join("resp.json");
    std::fs::write(&rf, serde_json::to_vec(&req).unwrap()).unwrap();
    std::fs::write(&pf, serde_json::to_vec(&resp).unwrap()).unwrap();
    // A JSON response file is a non-streamed call; a streamed one needs the proxy's parts.
    let mut plain = meta(Api::Messages, "f-1");
    plain.stream = false;
    let a = ingest_call_files(&mut s, &blocks, &plain, &rf, &pf, None).unwrap();
    assert_eq!(
        (
            a.request_parts,
            a.response_parts,
            a.raw_blocks,
            a.already_recorded
        ),
        (1, 1, 2, false)
    );
    // The same call id again: nothing written, the same entry answered.
    let again = ingest_call_files(&mut s, &blocks, &plain, &rf, &pf, None).unwrap();
    assert!(again.already_recorded);
    assert_eq!(again.entry_id, a.entry_id);
    assert_eq!(s.customs(CUSTOM_CALL).unwrap().len(), 1);
    // A stream body: the proxy supplies the parts it assembled.
    let sf = dir.path().join("stream.sse");
    std::fs::write(&sf, b"event: message_start\ndata: {}\n\n").unwrap();
    let b = ingest_call_files(
        &mut s,
        &blocks,
        &meta(Api::Messages, "f-2"),
        &rf,
        &sf,
        Some(vec![json!({"type":"text","text":"assembled"})]),
    )
    .unwrap();
    assert_eq!(
        (b.response_parts, b.part_blocks_reused, b.raw_blocks),
        (1, 1, 1)
    );
    assert!(blocks.verify_all().unwrap().iter().all(|(_, ok)| *ok));
    drop(s);
    dir.close().unwrap();
}

#[test]
fn an_outcome_that_did_not_complete_records_absence_and_never_response_parts() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("c4", "/w", None).unwrap();
    let outcome = |status, id: &str| OutcomeMeta {
        call_id: id.into(),
        provider: "anthropic".into(),
        api: Api::Messages,
        status,
        started_at: "2026-09-24T03:00:00Z".into(),
        duration_ms: None,
        stream: true,
    };
    // Lost mid-upload: a request file that is not whole JSON, no response.
    let half = dir.path().join("half.json");
    std::fs::write(
        &half,
        b"{\"model\":\"m\",\"messages\":[{\"role\":\"user\",\"con",
    )
    .unwrap();
    let r = ingest_outcome(
        &mut s,
        &blocks,
        &outcome(CallStatus::Lost, "lost-1"),
        Some(&half),
        None,
    )
    .unwrap();
    assert_eq!((r.request_parts, r.response_parts, r.raw_blocks), (0, 0, 1));
    let calls = s.customs(CUSTOM_CALL).unwrap();
    let EntryBody::Custom {
        data: Some(data), ..
    } = &calls[0].body
    else {
        panic!()
    };
    let rec = call_record(data).unwrap();
    assert_eq!(rec.status, CallStatus::Lost);
    assert!(rec.model.is_none() && rec.raw_response.is_none() && rec.raw_request.is_some());
    assert!(rec.response.is_empty());
    // Partial: whole request, a partial stream file; parts from the request, none from the response.
    let rf = dir.path().join("req.json");
    std::fs::write(
        &rf,
        serde_json::to_vec(&json!({"model":"m","messages":[{"role":"user","content":"u"}]}))
            .unwrap(),
    )
    .unwrap();
    let sf = dir.path().join("part.sse");
    std::fs::write(&sf, b"event: message_start\n").unwrap();
    let r = ingest_outcome(
        &mut s,
        &blocks,
        &outcome(CallStatus::Partial, "part-1"),
        Some(&rf),
        Some(&sf),
    )
    .unwrap();
    assert_eq!((r.request_parts, r.response_parts, r.raw_blocks), (1, 0, 2));
    let calls = s.customs(CUSTOM_CALL).unwrap();
    let EntryBody::Custom {
        data: Some(data), ..
    } = &calls[1].body
    else {
        panic!()
    };
    let rec = call_record(data).unwrap();
    assert_eq!(rec.model.as_deref(), Some("m"));
    assert!(rec.response.is_empty() && rec.raw_response.is_some());
    // Unrecorded with nothing at all: still one honest record.
    let r = ingest_outcome(
        &mut s,
        &blocks,
        &outcome(CallStatus::Unrecorded, "un-1"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(r.raw_blocks, 0);
    // A second report of the same lost call adds nothing.
    let r = ingest_outcome(
        &mut s,
        &blocks,
        &outcome(CallStatus::Lost, "lost-1"),
        None,
        None,
    )
    .unwrap();
    assert!(r.already_recorded);
    assert_eq!(s.customs(CUSTOM_CALL).unwrap().len(), 3);
    // Complete is refused here.
    assert!(
        ingest_outcome(
            &mut s,
            &blocks,
            &outcome(CallStatus::Complete, "c-1"),
            None,
            None
        )
        .is_err()
    );
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_call_is_found_after_a_crash_before_the_head_advanced_and_after_the_head_moved() {
    use crate::record::index::write_head;
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let file = {
        let mut s = home.create_session("c5", "/w", None).unwrap();
        let first = s
            .append(crate::record::entries::EntryBody::Label {
                target_id: "x".into(),
                label: None,
            })
            .unwrap();
        let req =
            serde_json::to_vec(&json!({"model":"m","messages":[{"role":"user","content":"u"}]}))
                .unwrap();
        ingest_call(
            &mut s,
            &blocks,
            &meta(Api::Messages, "crash-1"),
            &req,
            b"{\"content\":[{\"type\":\"text\",\"text\":\"a\"}]}",
            None,
        )
        .unwrap();
        // Simulate the crash: the entry and its index row are durable, the head file still names the label.
        write_head(s.file(), Some(&first)).unwrap();
        s.file().to_path_buf()
    };
    let mut s = crate::record::Session::open(&file).unwrap();
    assert_eq!(
        s.customs(CUSTOM_CALL).unwrap().len(),
        0,
        "the path does not reach the call"
    );
    let r = ingest_outcome(
        &mut s,
        &blocks,
        &OutcomeMeta {
            call_id: "crash-1".into(),
            provider: "anthropic".into(),
            api: Api::Messages,
            status: CallStatus::Lost,
            started_at: "t".into(),
            duration_ms: None,
            stream: false,
        },
        None,
        None,
    )
    .unwrap();
    assert!(
        r.already_recorded,
        "recovery must find the durable call, not add a lost twin"
    );
    assert_eq!(s.customs_everywhere(CUSTOM_CALL).unwrap().len(), 1);
    // Move the head to the root and replay the same call id: still found.
    s.move_head(None).unwrap();
    let again = ingest_call(
        &mut s,
        &blocks,
        &meta(Api::Messages, "crash-1"),
        b"{\"model\":\"m\",\"messages\":[]}",
        b"{\"content\":[]}",
        None,
    )
    .unwrap();
    assert!(again.already_recorded);
    drop(s);
    dir.close().unwrap();
}

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
