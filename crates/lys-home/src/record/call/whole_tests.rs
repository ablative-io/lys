use super::*;
use crate::record::blocks::Hash;
use serde_json::json;

const REQUEST: &[u8] = br#"{"model":"m","messages":[{"role":"user","content":"hello"}]}"#;
const RESPONSE: &[u8] = br#"{"type":"message","content":[{"type":"text","text":"phi"}]}"#;
const STREAM: &[u8] =
    b"event: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: ping\ndata: not json\n\n";

struct Kept {
    _dir: tempfile::TempDir,
    home: Home,
    entry: String,
}

/// A home whose session `s` holds one call with these stored bodies.
fn kept(
    response: Option<&[u8]>,
    coding: Option<&str>,
    stream: bool,
) -> Result<Kept, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let blocks = home.blocks()?;
    blocks.put(REQUEST)?;
    let mut data = json!({
        "call_id": "c", "provider": "p", "api": "anthropic-messages",
        "request": [], "response": [], "status": "complete",
        "started_at": "2000-01-01T00:00:00Z", "stream": stream,
        "raw_request": Hash::of(REQUEST).as_str(),
        "head": {
            "status": 200,
            "request": { "names": [], "values": {} },
            "response": { "names": [], "values": {} },
        },
    });
    if let Some(response) = response {
        blocks.put(response)?;
        data["raw_response"] = json!(Hash::of(response).as_str());
    }
    if let Some(coding) = coding {
        data["head"]["response"]["values"]["content-encoding"] = json!([coding]);
    }
    let mut session = home.create_session("s", "", None)?;
    let entry = session.append(EntryBody::Custom {
        custom_type: CUSTOM_CALL.to_owned(),
        data: Some(data),
    })?;
    Ok(Kept {
        _dir: dir,
        home,
        entry,
    })
}

fn gzip(body: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(body)?;
    Ok(encoder.finish()?)
}

#[test]
fn a_call_is_read_whole_with_both_bodies_as_json() -> Result<(), Box<dyn std::error::Error>> {
    for (stored, coding) in [(RESPONSE.to_vec(), None), (gzip(RESPONSE)?, Some("gzip"))] {
        let kept = kept(Some(&stored), coding, false)?;
        let whole = call_whole(&kept.home, "s", &kept.entry)?;
        assert_eq!(whole.call.call_id, "c");
        assert_eq!(whole.request.json, Some(serde_json::from_slice(REQUEST)?));
        assert_eq!(whole.request.unreadable, None);
        assert_eq!(whole.response.json, Some(serde_json::from_slice(RESPONSE)?));
        assert_eq!(whole.response.unreadable, None, "{coding:?}");
    }
    Ok(())
}

#[test]
fn a_stream_is_its_events_in_order_each_data_as_json_when_it_is_json()
-> Result<(), Box<dyn std::error::Error>> {
    let kept = kept(Some(STREAM), None, true)?;
    let whole = call_whole(&kept.home, "s", &kept.entry)?;
    assert_eq!(
        whole.response.json,
        Some(json!([
            { "event": "message_start", "data": { "type": "message_start" } },
            { "event": "ping", "data": "not json" },
        ]))
    );
    assert_eq!(whole.response.unreadable, None);
    // A stream that stops inside an event shows what came before and says so.
    let cut = &STREAM[..STREAM.len() - 6];
    let short = self::kept(Some(cut), None, true)?;
    let whole = call_whole(&short.home, "s", &short.entry)?;
    assert_eq!(
        whole.response.json,
        Some(json!([{ "event": "message_start", "data": { "type": "message_start" } }]))
    );
    assert_eq!(
        whole.response.unreadable.as_deref(),
        Some("the stream stops inside an event; the events before it are shown")
    );
    Ok(())
}

#[test]
fn a_body_that_cannot_be_shown_says_why() -> Result<(), Box<dyn std::error::Error>> {
    // A coding that is not decoded is named, and its bytes are not guessed at.
    let coded = kept(Some(RESPONSE), Some("zstd"), false)?;
    let whole = call_whole(&coded.home, "s", &coded.entry)?;
    assert_eq!(whole.response.json, None);
    assert_eq!(
        whole.response.unreadable.as_deref(),
        Some("the body's content-encoding is zstd, which is not decoded")
    );
    // Bytes that are not the coding named.
    let wrong = kept(Some(RESPONSE), Some("gzip"), false)?;
    let reason = call_whole(&wrong.home, "s", &wrong.entry)?
        .response
        .unreadable
        .ok_or("no reason")?;
    assert!(
        reason.starts_with(
            "the body's content-encoding is gzip, and its stored bytes do not decode as that: "
        ),
        "{reason}"
    );
    // Bytes that are not JSON.
    let text = kept(Some(b"plain words"), None, false)?;
    let reason = call_whole(&text.home, "s", &text.entry)?
        .response
        .unreadable
        .ok_or("no reason")?;
    assert!(reason.starts_with("the body is not JSON: "), "{reason}");
    // No body at all.
    let none = kept(None, None, false)?;
    let whole = call_whole(&none.home, "s", &none.entry)?;
    assert_eq!(
        whole.response.unreadable.as_deref(),
        Some("no body was recorded")
    );
    assert_eq!(whole.request.unreadable, None);
    Ok(())
}

#[test]
fn an_entry_that_is_no_call_and_a_session_that_is_not_there_are_refused()
-> Result<(), Box<dyn std::error::Error>> {
    let kept = kept(Some(RESPONSE), None, false)?;
    let mut session = kept.home.open_session("s")?;
    let other = session.append(EntryBody::Custom {
        custom_type: "other".to_owned(),
        data: Some(json!({})),
    })?;
    drop(session);
    assert!(call_whole(&kept.home, "s", &other).is_err());
    assert!(call_whole(&kept.home, "s", "no-such-entry").is_err());
    assert!(call_whole(&kept.home, "absent", &kept.entry).is_err());
    Ok(())
}
