//! Gates on one message entry into Codex 0.156.0 response items (HOME-009
//! R3): each part carried whole as Codex's own item, thinking as text, a
//! base64 image as input_image, opaque and unplaceable parts lost with their
//! reason, and every field not carried named.

use serde_json::{Value, json};

use crate::harness::codex::account::{Rows, part_hash};
use crate::harness::codex::parts::message_items;

fn walk(message: &Value) -> (Vec<Value>, Rows) {
    let mut rows = Rows::default();
    let items = message_items("m1", message, &mut rows);
    (items, rows)
}

fn assistant(parts: &Value) -> Value {
    json!({"role": "assistant", "content": parts})
}

#[test]
fn tool_call_becomes_function_call() {
    let part = json!({"type": "toolCall", "id": "toolu_1", "name": "Bash", "arguments": {"command": "ls"}});
    let (items, rows) = walk(&assistant(&json!([part])));
    assert_eq!(
        items,
        [
            json!({"type": "function_call", "name": "Bash", "arguments": "{\"command\":\"ls\"}", "call_id": "toolu_1"})
        ]
    );
    assert_eq!(rows.kept.len(), 1);
    assert_eq!(rows.kept[0].before, "toolCall");
    assert_eq!(rows.kept[0].after, "function_call");
    assert_eq!(rows.kept[0].hash, Some(part_hash(&part)));
    assert!(rows.changed.is_empty() && rows.lost.is_empty());
}

#[test]
fn extra_part_field_is_changed() {
    let (items, rows) = walk(&assistant(
        &json!([{"type": "text", "text": "a", "textSignature": "s"}]),
    ));
    assert_eq!(
        items,
        [
            json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "a"}]})
        ]
    );
    assert!(rows.kept.is_empty());
    assert_eq!(rows.changed.len(), 1);
    assert_eq!(rows.changed[0].before, "text");
    assert_eq!(rows.changed[0].after, "output_text");
    assert_eq!(rows.changed[0].how, "textSignature not carried");
}

#[test]
fn tool_call_without_id_or_name_is_lost() {
    let no_id = json!({"type": "toolCall", "id": "", "name": "Bash", "arguments": {}});
    let no_name = json!({"type": "toolCall", "id": "toolu_2", "arguments": {}});
    let (items, rows) = walk(&assistant(&json!([no_id, no_name])));
    assert!(
        items
            .iter()
            .all(|item| item["type"] != json!("function_call"))
    );
    assert_eq!(rows.lost.len(), 2);
    assert!(rows.lost.iter().all(|row| row.kind == "toolCall"));
    assert_eq!(rows.lost[0].hash, Some(part_hash(&no_id)));
    assert_eq!(rows.lost[1].hash, Some(part_hash(&no_name)));
    assert_eq!(
        rows.lost[0].reason,
        "toolCall with no id: Codex's function_call needs a call_id"
    );
    assert_eq!(
        rows.lost[1].reason,
        "toolCall with no name: Codex's function_call needs a name"
    );
}

#[test]
fn tool_result_without_call_id_is_lost() {
    let empty = json!({"role": "toolResult", "toolCallId": "", "content": [{"type": "text", "text": "ok"}]});
    let absent = json!({"role": "toolResult", "content": [{"type": "text", "text": "ok"}]});
    for message in [empty, absent] {
        let (items, rows) = walk(&message);
        assert!(items.is_empty());
        assert!(rows.kept.is_empty() && rows.changed.is_empty());
        assert_eq!(rows.lost.len(), 1);
        assert_eq!(rows.lost[0].entry, "m1");
        assert_eq!(rows.lost[0].kind, "toolResult");
        assert_eq!(rows.lost[0].hash, Some(part_hash(&message)));
        assert_eq!(
            rows.lost[0].reason,
            "toolResult with no toolCallId: Codex's function_call_output needs a call_id"
        );
    }
}

#[test]
fn unknown_role_is_lost() {
    let message = json!({"role": "bashExecution", "content": "x"});
    let (items, rows) = walk(&message);
    assert!(items.is_empty());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.lost[0].entry, "m1");
    assert_eq!(rows.lost[0].hash, Some(part_hash(&message)));
    assert_eq!(rows.lost[0].kind, "bashExecution message");
    assert_eq!(
        rows.lost[0].reason,
        "Codex has no item for a message of this role"
    );
    let (items, rows) = walk(&json!({"content": "x"}));
    assert!(items.is_empty());
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].kind, "message with no role");
}

#[test]
fn one_item_result_is_a_string() {
    let message = json!({"role": "toolResult", "toolCallId": "toolu_1", "toolName": "Bash",
        "content": [{"type": "text", "text": "ok"}], "isError": false, "timestamp": 1});
    let (items, rows) = walk(&message);
    assert_eq!(
        items,
        [json!({"type": "function_call_output", "call_id": "toolu_1", "output": "ok"})]
    );
    assert!(rows.kept.is_empty() && rows.lost.is_empty());
    assert_eq!(rows.changed.len(), 1);
    assert_eq!(rows.changed[0].hash, Some(part_hash(&message)));
    assert_eq!(rows.changed[0].before, "toolResult");
    assert_eq!(rows.changed[0].after, "function_call_output");
    assert_eq!(
        rows.changed[0].how,
        "content: one-item text array written as a string; isError not carried; timestamp not carried; toolName not carried"
    );
}

#[test]
fn two_item_result_is_a_list() {
    let message = json!({"role": "toolResult", "toolCallId": "toolu_1",
        "content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]});
    let (items, _) = walk(&message);
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["output"],
        json!([{"type": "input_text", "text": "a"}, {"type": "input_text", "text": "b"}])
    );
}

#[test]
fn assistant_turn_fields_are_named() {
    let message = json!({"role": "assistant", "content": [{"type": "text", "text": "a"}],
        "api": "anthropic-messages", "provider": "anthropic", "model": "m", "usage": {},
        "stopReason": "stop", "timestamp": 1});
    let (_, rows) = walk(&message);
    let entry_rows: Vec<_> = rows
        .changed
        .iter()
        .filter(|row| row.hash.is_none())
        .collect();
    assert_eq!(entry_rows.len(), 1);
    assert_eq!(entry_rows[0].before, "assistant message");
    assert_eq!(entry_rows[0].after, "message item");
    assert_eq!(
        entry_rows[0].how,
        "api not carried; model not carried; provider not carried; stopReason not carried; timestamp not carried; usage not carried"
    );
    assert!(rows.lost.iter().all(|row| row.hash.is_some()));
    assert!(rows.kept.iter().all(|row| row.hash.is_some()));
}

#[test]
fn thinking_is_text_and_opaque_is_lost() {
    let redacted = json!({"type": "thinking", "thinking": "", "thinkingSignature": "opaque", "redacted": true});
    let (items, rows) = walk(&assistant(&json!([
        {"type": "thinking", "thinking": "t", "thinkingSignature": "sig"},
        redacted
    ])));
    assert_eq!(
        items,
        [
            json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "t"}]})
        ]
    );
    assert_eq!(rows.changed.len(), 1);
    assert_eq!(rows.changed[0].before, "thinking");
    assert_eq!(rows.changed[0].after, "output_text");
    assert_eq!(rows.changed[0].how, "thinkingSignature not carried");
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].hash, Some(part_hash(&redacted)));
    assert_eq!(
        rows.lost[0].reason,
        "redacted thinking dropped: another provider"
    );
    let written = serde_json::to_string(&items).unwrap_or_else(|e| e.to_string());
    assert!(!written.contains("sig"));
    assert!(!written.contains("opaque"));
    assert!(!written.contains("encrypted_content"));
    let (items, rows) = walk(&assistant(&json!([{"type": "thinking", "thinking": " "}])));
    assert!(items.is_empty());
    assert_eq!(rows.lost[0].reason, "empty thinking dropped");
}

#[test]
fn base64_image_is_input_image_and_url_image_is_lost() {
    let base64 = json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}});
    let url =
        json!({"type": "image", "source": {"type": "url", "url": "https://example.invalid/a.png"}});
    let message =
        json!({"role": "user", "content": [{"type": "text", "text": "look"}, base64, url]});
    let (items, rows) = walk(&message);
    let data_url =
        json!({"type": "input_image", "image_url": "data:image/png;base64,iVBORw0KGgo="});
    assert_eq!(
        items,
        [
            json!({"type": "message", "role": "user", "content": [{"type": "input_text", "text": "look"}, data_url]})
        ]
    );
    let images: Vec<_> = rows
        .changed
        .iter()
        .filter(|row| row.before == "image")
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].after, "input_image");
    assert_eq!(images[0].hash, Some(part_hash(&base64)));
    assert_eq!(images[0].how, "source written as a data URL image_url");
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].kind, "image");
    assert_eq!(rows.lost[0].hash, Some(part_hash(&url)));
    assert_eq!(
        rows.lost[0].reason,
        "image source url not carried: part 2, never fetched"
    );
    let written = serde_json::to_string(&items).unwrap_or_else(|e| e.to_string());
    let account = serde_json::to_string(&rows).unwrap_or_else(|e| e.to_string());
    assert!(!written.contains("example.invalid"));
    assert!(!account.contains("example.invalid"));
    assert!(!written.contains("detail"));
    let result = json!({"role": "toolResult", "toolCallId": "toolu_1",
        "content": [{"type": "text", "text": "shot"}, base64]});
    let (items, _) = walk(&result);
    assert_eq!(
        items[0]["output"],
        json!([{"type": "input_text", "text": "shot"}, data_url])
    );
}
