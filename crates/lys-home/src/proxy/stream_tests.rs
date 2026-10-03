#![cfg(test)]
//! Gates on the stream grammars: each api's stream assembled into the parts
//! its JSON response would hold, split at every byte, and every way a stream
//! can stop short read as partial, never as whole.

use serde_json::{Value, json};

use crate::proxy::stream::StreamReader;
use crate::proxy::stream_sse::{SseEvent, SseFramer};
use crate::record::call::Api;

/// Events framed as a server sends them, each named by its `type`.
fn sse(events: &[Value]) -> String {
    let mut out = String::new();
    for event in events {
        let kind = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("message");
        for piece in ["event: ", kind, "\ndata: ", &event.to_string(), "\n\n"] {
            out.push_str(piece);
        }
    }
    out
}

fn data_lines(chunks: &[Value]) -> String {
    let mut out = String::new();
    for chunk in chunks {
        for piece in ["data: ", &chunk.to_string(), "\n\n"] {
            out.push_str(piece);
        }
    }
    out
}

/// A `content_block_start` event for `block` at `index`.
fn start(index: u64, block: &Value) -> Value {
    json!({"type": "content_block_start", "index": index, "content_block": block})
}

fn delta(index: u64, kind: &str, key: &str, text: &str) -> Value {
    let mut delta = json!({ "type": kind });
    delta[key] = json!(text);
    json!({"type": "content_block_delta", "index": index, "delta": delta})
}

fn stop_block(index: u64) -> Value {
    json!({"type": "content_block_stop", "index": index})
}

fn messages_stream(stop: bool) -> String {
    let tool = json!({"type": "tool_use", "id": "toolu_1", "name": "probe", "input": {}});
    let mut events = vec![
        json!({"type": "message_start", "message": {"id": "msg_1", "content": []}}),
        start(0, &json!({"type": "thinking", "thinking": ""})),
        delta(0, "thinking_delta", "thinking", "alpha "),
        delta(0, "thinking_delta", "thinking", "beta"),
        delta(0, "signature_delta", "signature", "c2ln"),
        stop_block(0),
        json!({"type": "ping"}),
        start(1, &json!({"type": "text", "text": ""})),
        delta(1, "text_delta", "text", "gam"),
        delta(1, "text_delta", "text", "ma"),
        stop_block(1),
        start(2, &tool),
        delta(2, "input_json_delta", "partial_json", "{\"n\":"),
        delta(2, "input_json_delta", "partial_json", "7}"),
        stop_block(2),
        json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}}),
    ];
    if stop {
        events.push(json!({"type": "message_stop"}));
    }
    sse(&events)
}

fn read(api: Api, bytes: &[u8], step: usize) -> Option<Vec<Value>> {
    let mut reader = StreamReader::for_api(api);
    for chunk in bytes.chunks(step) {
        reader.feed(chunk);
    }
    reader.finish()
}

#[test]
fn a_messages_stream_names_its_message_whether_or_not_it_ends_whole() {
    for (stop, whole) in [(true, true), (false, false)] {
        let stream = messages_stream(stop);
        for step in [1, 3, stream.len()] {
            let mut reader = StreamReader::for_api(Api::Messages);
            assert_eq!(reader.message_id(), None, "no id before message_start");
            for chunk in stream.as_bytes().chunks(step) {
                reader.feed(chunk);
            }
            assert_eq!(
                reader.message_id(),
                Some("msg_1"),
                "stop {stop} step {step}"
            );
            assert_eq!(reader.finish().is_some(), whole, "stop {stop} step {step}");
        }
    }
    // A message_start that names no id leaves the id absent and the stream whole.
    let unnamed = sse(&[
        json!({"type": "message_start", "message": {"content": []}}),
        json!({"type": "message_stop"}),
    ]);
    let mut reader = StreamReader::for_api(Api::Messages);
    reader.feed(unnamed.as_bytes());
    assert_eq!(reader.message_id(), None);
    assert_eq!(reader.finish(), Some(Vec::new()));
    // The other grammars name no message here.
    for api in [Api::ChatCompletions, Api::Responses] {
        assert_eq!(StreamReader::for_api(api).message_id(), None);
    }
}

#[test]
fn a_messages_stream_assembles_to_the_content_a_json_response_holds_split_at_any_byte() {
    let stream = messages_stream(true);
    let expected = vec![
        json!({"type": "thinking", "thinking": "alpha beta", "signature": "c2ln"}),
        json!({"type": "text", "text": "gamma"}),
        json!({"type": "tool_use", "id": "toolu_1", "name": "probe", "input": {"n": 7}}),
    ];
    let mut splits = 0;
    for step in [1, 2, 3, 7, 64, stream.len()] {
        assert_eq!(
            read(Api::Messages, stream.as_bytes(), step),
            Some(expected.clone())
        );
        splits += 1;
    }
    assert_eq!(splits, 6);
}

#[test]
fn a_messages_stream_without_message_stop_is_partial() {
    assert_eq!(
        read(Api::Messages, messages_stream(false).as_bytes(), 5),
        None
    );
}

#[test]
fn a_messages_stream_cut_inside_an_event_is_partial() {
    let whole = messages_stream(true);
    let cut = &whole.as_bytes()[..whole.len() - 3];
    assert_eq!(read(Api::Messages, cut, 4), None);
}

#[test]
fn a_messages_stream_with_an_error_event_or_a_block_left_open_is_partial() {
    let error = sse(&[
        json!({"type": "message_start", "message": {}}),
        json!({"type": "error", "error": {"type": "overloaded_error"}}),
        json!({"type": "message_stop"}),
    ]);
    assert_eq!(read(Api::Messages, error.as_bytes(), 9), None);
    let open = sse(&[
        start(0, &json!({"type": "text", "text": ""})),
        json!({"type": "message_stop"}),
    ]);
    assert_eq!(read(Api::Messages, open.as_bytes(), 9), None);
}

#[test]
fn a_chat_stream_assembles_each_choice_with_its_tool_calls() {
    let choice = |delta: Value| json!({"choices": [{"index": 0, "delta": delta}]});
    let first_call = json!({
        "index": 0,
        "id": "call_1",
        "type": "function",
        "function": {"name": "probe", "arguments": "{\"n\""}
    });
    let rest_of_call = json!({"index": 0, "function": {"arguments": ":1}"}});
    let chunks = [
        choice(json!({"role": "assistant", "content": "del"})),
        choice(json!({"content": "ta"})),
        choice(json!({ "tool_calls": [first_call] })),
        choice(json!({ "tool_calls": [rest_of_call] })),
        json!({"choices": [], "usage": {"total_tokens": 3}}),
    ];
    let mut stream = data_lines(&chunks);
    stream.push_str("data: [DONE]\n\n");
    let call = json!({
        "id": "call_1",
        "type": "function",
        "function": {"name": "probe", "arguments": "{\"n\":1}"}
    });
    let expected = vec![json!({"role": "assistant", "content": "delta", "tool_calls": [call]})];
    assert_eq!(
        read(Api::ChatCompletions, stream.as_bytes(), 3),
        Some(expected)
    );
    let unfinished = data_lines(&chunks);
    assert_eq!(read(Api::ChatCompletions, unfinished.as_bytes(), 3), None);
}

#[test]
fn a_responses_stream_gives_the_completed_output_and_a_failed_one_gives_nothing() {
    let item = json!({
        "type": "message",
        "role": "assistant",
        "content": [{"type": "output_text", "text": "epsilon"}]
    });
    let done = json!({"type": "response.output_item.done", "output_index": 0, "item": item});
    let completed = sse(&[
        json!({"type": "response.created", "response": {"status": "in_progress"}}),
        json!({"type": "response.output_text.delta", "output_index": 0, "delta": "eps"}),
        done.clone(),
        json!({
            "type": "response.completed",
            "response": {"status": "completed", "output": [item]}
        }),
    ]);
    assert_eq!(
        read(Api::Responses, completed.as_bytes(), 11),
        Some(vec![item])
    );
    let failed = sse(&[
        done.clone(),
        json!({"type": "response.failed", "response": {"status": "failed"}}),
    ]);
    assert_eq!(read(Api::Responses, failed.as_bytes(), 11), None);
    let no_end = sse(&[done]);
    assert_eq!(read(Api::Responses, no_end.as_bytes(), 11), None);
}

#[test]
fn the_framer_reads_crlf_and_lone_cr_line_ends_and_joins_data_lines() {
    let mut framer = SseFramer::new();
    let mut out = Vec::new();
    framer.feed(
        b": comment\r\nevent: a\r\ndata: one\r\ndata: two\r\n\r\ndata: three\r\r",
        &mut out,
    );
    assert_eq!(
        out,
        vec![
            SseEvent {
                event: Some("a".to_owned()),
                data: "one\ntwo".to_owned()
            },
            SseEvent {
                event: None,
                data: "three".to_owned()
            },
        ]
    );
    assert!(!framer.pending());
    framer.feed(b"data: \xff\n\n", &mut out);
    assert!(framer.malformed());
    assert_eq!(out.len(), 2);
}
