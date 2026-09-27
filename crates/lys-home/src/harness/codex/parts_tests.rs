//! Gates on one message as Codex items (HOME-009 R3): a tool call is a
//! `function_call` carried whole, a field beyond an item's is counted
//! changed, a missing id, name, `toolCallId` or role is a lost row and never
//! a default, a tool result's one text part is a string and two are a list,
//! an assistant turn's own fields are named, readable thinking is text and
//! opaque thinking is lost, and a base64 image is an `input_image` while a
//! URL image is lost and never fetched.

use std::error::Error;

use serde_json::{Value, json};

use crate::harness::codex::account::{Rows, part_hash};
use crate::harness::codex::parts::message_items;

type Gate = Result<(), Box<dyn Error>>;

/// The items and rows of one message entry `m1`.
fn items_of(message: &Value) -> Result<(Vec<Value>, Rows), Box<dyn Error>> {
    let mut rows = Rows::default();
    let items = message_items("m1", message, &mut rows)?;
    Ok((items, rows))
}

fn assistant(parts: &[Value]) -> Value {
    json!({"role": "assistant", "content": parts})
}

fn said(parts: &Value) -> Value {
    json!({"type": "message", "role": "assistant", "content": parts})
}

#[test]
fn tool_call_becomes_function_call() -> Gate {
    let call = json!({"type": "toolCall", "id": "toolu_1", "name": "Bash",
        "arguments": {"command": "ls"}});
    let (items, rows) = items_of(&assistant(std::slice::from_ref(&call)))?;
    let expected = json!({"type": "function_call", "name": "Bash",
        "arguments": "{\"command\":\"ls\"}", "call_id": "toolu_1"});
    assert_eq!(items, [expected]);
    assert_eq!(rows.kept.len(), 1);
    assert_eq!(rows.kept[0].before, "toolCall");
    assert_eq!(rows.kept[0].after, "function_call");
    assert_eq!(rows.kept[0].hash, Some(part_hash(&call)?));
    assert!(rows.changed.is_empty());
    assert!(rows.lost.is_empty());
    Ok(())
}

#[test]
fn extra_part_field_is_changed() -> Gate {
    let part = json!({"type": "text", "text": "a", "textSignature": "s"});
    let (items, rows) = items_of(&assistant(&[part]))?;
    let expected = said(&json!([{"type": "output_text", "text": "a"}]));
    assert_eq!(items, [expected]);
    assert_eq!(rows.changed.len(), 1);
    assert_eq!(rows.changed[0].before, "text");
    assert_eq!(rows.changed[0].after, "output_text");
    assert_eq!(rows.changed[0].how, "textSignature not carried");
    assert!(rows.kept.is_empty());
    Ok(())
}

#[test]
fn tool_call_without_id_or_name_is_lost() -> Gate {
    let no_id = json!({"type": "toolCall", "id": "", "name": "Bash", "arguments": {}});
    let no_name = json!({"type": "toolCall", "id": "toolu_2", "arguments": {}});
    let (items, rows) = items_of(&assistant(&[no_id.clone(), no_name.clone()]))?;
    let calls = items.iter().filter(|i| i["type"] == "function_call");
    assert_eq!(calls.count(), 0);
    assert_eq!(rows.lost.len(), 2);
    assert!(rows.lost.iter().all(|row| row.kind == "toolCall"));
    assert_eq!(rows.lost[0].hash, Some(part_hash(&no_id)?));
    assert_eq!(rows.lost[1].hash, Some(part_hash(&no_name)?));
    assert_eq!(
        rows.lost[0].reason,
        "toolCall with no id: Codex's function_call needs a call_id"
    );
    assert_eq!(
        rows.lost[1].reason,
        "toolCall with no name: Codex's function_call needs a name"
    );
    Ok(())
}

#[test]
fn tool_result_without_call_id_is_lost() -> Gate {
    let empty = json!({"role": "toolResult", "toolCallId": "",
        "content": [{"type": "text", "text": "ok"}]});
    let absent = json!({"role": "toolResult", "content": [{"type": "text", "text": "ok"}]});
    let mut checked = 0;
    for message in [empty, absent] {
        let (items, rows) = items_of(&message)?;
        assert!(items.is_empty());
        assert_eq!(rows.lost.len(), 1);
        assert_eq!(rows.lost[0].kind, "toolResult");
        assert_eq!(rows.lost[0].hash, Some(part_hash(&message)?));
        assert_eq!(
            rows.lost[0].reason,
            "toolResult with no toolCallId: Codex's function_call_output needs a call_id"
        );
        assert!(rows.kept.is_empty() && rows.changed.is_empty());
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}

#[test]
fn unknown_role_is_lost() -> Gate {
    let message = json!({"role": "bashExecution", "content": "x"});
    let (items, rows) = items_of(&message)?;
    assert!(items.is_empty());
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].entry, "m1");
    assert_eq!(rows.lost[0].hash, Some(part_hash(&message)?));
    assert_eq!(rows.lost[0].kind, "bashExecution message");
    assert_eq!(
        rows.lost[0].reason,
        "Codex has no item for a message of this role"
    );
    let (items, rows) = items_of(&json!({"content": "x"}))?;
    assert!(items.is_empty());
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].kind, "message with no role");
    Ok(())
}

#[test]
fn one_item_result_is_a_string() -> Gate {
    let message = json!({"role": "toolResult", "toolCallId": "toolu_1", "toolName": "Bash",
        "content": [{"type": "text", "text": "ok"}], "isError": false, "timestamp": 1});
    let (items, rows) = items_of(&message)?;
    let expected = json!({"type": "function_call_output", "call_id": "toolu_1",
        "output": "ok"});
    assert_eq!(items, [expected]);
    assert_eq!(rows.changed.len(), 1);
    assert_eq!(
        rows.changed[0].how,
        "content: one-item text array written as a string; isError not carried; \
         timestamp not carried; toolName not carried"
    );
    assert_eq!(rows.changed[0].hash, Some(part_hash(&message)?));
    assert_eq!(rows.changed[0].before, "toolResult");
    assert_eq!(rows.changed[0].after, "function_call_output");
    Ok(())
}

#[test]
fn two_item_result_is_a_list() -> Gate {
    let message = json!({"role": "toolResult", "toolCallId": "toolu_1",
        "content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]});
    let (items, _) = items_of(&message)?;
    assert_eq!(items.len(), 1);
    let expected = json!([{"type": "input_text", "text": "a"},
        {"type": "input_text", "text": "b"}]);
    assert_eq!(items[0]["output"], expected);
    Ok(())
}

#[test]
fn assistant_turn_fields_are_named() -> Gate {
    let message = json!({"role": "assistant", "content": [{"type": "text", "text": "a"}],
        "api": "anthropic-messages", "provider": "anthropic", "model": "m", "usage": {},
        "stopReason": "stop", "timestamp": 1});
    let (_, rows) = items_of(&message)?;
    let entry_rows: Vec<_> = rows.changed.iter().filter(|r| r.hash.is_none()).collect();
    assert_eq!(entry_rows.len(), 1);
    assert_eq!(
        entry_rows[0].how,
        "api not carried; model not carried; provider not carried; \
         stopReason not carried; timestamp not carried; usage not carried"
    );
    assert!(rows.kept.iter().all(|row| row.hash.is_some()));
    assert!(rows.lost.iter().all(|row| row.hash.is_some()));
    Ok(())
}

#[test]
fn thinking_is_text_and_opaque_is_lost() -> Gate {
    let signed = json!({"type": "thinking", "thinking": "t", "thinkingSignature": "sig"});
    let redacted = json!({"type": "thinking", "thinking": "",
        "thinkingSignature": "opaque", "redacted": true});
    let (items, rows) = items_of(&assistant(&[signed, redacted.clone()]))?;
    let expected = said(&json!([{"type": "output_text", "text": "t"}]));
    assert_eq!(items, [expected]);
    assert_eq!(rows.changed.len(), 1);
    assert_eq!(rows.changed[0].how, "thinkingSignature not carried");
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].hash, Some(part_hash(&redacted)?));
    assert_eq!(
        rows.lost[0].reason,
        "redacted thinking dropped: another provider"
    );
    let written = serde_json::to_string(&items)?;
    assert!(!written.contains("sig"));
    assert!(!written.contains("opaque"));
    assert!(!written.contains("encrypted_content"));
    Ok(())
}

#[test]
fn base64_image_is_input_image_and_url_image_is_lost() -> Gate {
    let base64 = json!({"type": "image", "source": {"type": "base64",
        "media_type": "image/png", "data": "iVBORw0KGgo="}});
    let url = json!({"type": "image", "source": {"type": "url",
        "url": "https://example.invalid/a.png"}});
    let look = json!({"type": "text", "text": "look"});
    let user = json!({"role": "user", "content": [look, base64, url]});
    let (items, rows) = items_of(&user)?;
    let image = json!({"type": "input_image",
        "image_url": "data:image/png;base64,iVBORw0KGgo="});
    let content = json!([{"type": "input_text", "text": "look"}, image]);
    let expected = json!({"type": "message", "role": "user", "content": content});
    assert_eq!(items, [expected]);
    let images = rows.changed.iter().filter(|r| r.before == "image");
    let images: Vec<_> = images.collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].after, "input_image");
    assert_eq!(images[0].hash, Some(part_hash(&base64)?));
    assert_eq!(rows.lost.len(), 1);
    assert_eq!(rows.lost[0].kind, "image");
    assert_eq!(rows.lost[0].hash, Some(part_hash(&url)?));
    assert_eq!(
        rows.lost[0].reason,
        "image source url not carried: part 2, never fetched"
    );
    let rows_text = format!("{rows:?}");
    let items_text = serde_json::to_string(&items)?;
    assert!(!items_text.contains("example.invalid"));
    assert!(!rows_text.contains("example.invalid"));
    let shot = json!({"type": "text", "text": "shot"});
    let result = json!({"role": "toolResult", "toolCallId": "toolu_1",
        "content": [shot, base64]});
    let (items, _) = items_of(&result)?;
    assert_eq!(items.len(), 1);
    let expected = json!([{"type": "input_text", "text": "shot"}, image]);
    assert_eq!(items[0]["output"], expected);
    Ok(())
}
