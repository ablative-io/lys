#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the Claude Code importer: the tree is kept, parts become blocks,
//! tool results become their own messages, thinking keeps its signature, an
//! authored turn is marked, an unknown parent is refused by uuid.

use serde_json::{Value, json};

use crate::harness::claude_code::import::import_claude_code;
use crate::record::Home;
use crate::record::entries::{CUSTOM_AUTHORED, EntryBody};

fn rec(uuid: &str, parent: Option<&str>, kind: &str, message: &Value) -> String {
    serde_json::to_string(&json!({
        "parentUuid": parent, "isSidechain": false, "userType": "external", "cwd": "/w",
        "sessionId": "s", "version": "2.1.281", "gitBranch": "", "uuid": uuid,
        "timestamp": "2026-09-24T03:30:00.000Z", "type": kind, "message": message
    }))
    .unwrap()
}

const U1: &str = "11111111-1111-4111-8111-111111111111";
const U2: &str = "22222222-2222-4222-8222-222222222222";
const U3: &str = "33333333-3333-4333-8333-333333333333";
const U4: &str = "44444444-4444-4444-8444-444444444444";

/// A four-record transcript: a question, a signed-thinking answer with a tool
/// call, the tool's result, and the final answer.
pub(crate) fn fixture() -> String {
    let mut lines = vec![
        rec(
            U1,
            None,
            "user",
            &json!({"role": "user", "content": "what is in the box?"}),
        ),
        rec(
            U2,
            Some(U1),
            "assistant",
            &json!({"id": "msg_1", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
            "content": [
                {"type": "thinking", "thinking": "look first", "signature": "sig-abc"},
                {"type": "tool_use", "id": "toolu_1", "name": "Read", "input": {"file_path": "/box"}}
            ], "stop_reason": "tool_use", "stop_sequence": null, "usage": {"input_tokens": 10, "output_tokens": 5}}),
        ),
        rec(
            U3,
            Some(U2),
            "user",
            &json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": "a walrus", "is_error": false}]}),
        ),
        rec(
            U4,
            Some(U3),
            "assistant",
            &json!({"id": "msg_2", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
            "content": [{"type": "text", "text": "a walrus"}], "stop_reason": "end_turn", "stop_sequence": null, "usage": {"input_tokens": 12, "output_tokens": 2}}),
        ),
    ];
    lines.push(serde_json::to_string(&json!({"type": "attachment", "uuid": "a1", "parentUuid": U4, "attachment": {"type": "hook_success"}})).unwrap());
    lines.join("\n") + "\n"
}

#[test]
fn the_tree_parts_tool_results_and_signatures_survive_import() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("t.jsonl");
    std::fs::write(&src, fixture()).unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("imported", "/w", None).unwrap();
    let report = import_claude_code(&src, &mut s, &blocks).unwrap();
    assert_eq!(report.records, 5);
    assert_eq!(report.entries, 4);
    assert_eq!(report.counted_types.get("attachment"), Some(&1));
    assert_eq!(
        report.blocks, 5,
        "user text, thinking, tool call, tool result, answer"
    );
    assert!(!report.authored);
    let (path, _) = s.path().unwrap();
    assert_eq!(path.len(), 4);
    assert_eq!(path[1].parent_id(), Some(U1));
    let EntryBody::Message { message } = &path[1].body else {
        panic!()
    };
    assert_eq!(message["provider"], "anthropic");
    assert_eq!(message["content"][0]["thinkingSignature"], "sig-abc");
    assert_eq!(message["content"][1]["type"], "toolCall");
    assert_eq!(message["stopReason"], "toolUse");
    let EntryBody::Message { message } = &path[2].body else {
        panic!()
    };
    assert_eq!(message["role"], "toolResult");
    assert_eq!(message["toolName"], "Read");
    assert_eq!(message["toolCallId"], "toolu_1");
    assert!(blocks.verify_all().unwrap().iter().all(|(_, ok)| *ok));
    drop(s);
    dir.close().unwrap();
}

#[test]
fn an_authored_turn_is_marked_and_an_unknown_parent_is_refused_by_uuid() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.jsonl");
    let lines = [
        rec(
            U1,
            None,
            "user",
            &json!({"role": "user", "content": "the secret word is walrus"}),
        ),
        rec(
            U2,
            Some(U1),
            "assistant",
            &json!({"id": "m", "type": "message", "role": "assistant", "model": "authored",
            "content": [{"type": "text", "text": "noted"}], "stop_reason": "end_turn", "stop_sequence": null, "usage": {"input_tokens": 0, "output_tokens": 0}}),
        ),
    ];
    std::fs::write(&src, lines.join("\n") + "\n").unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("authored", "/w", None).unwrap();
    let report = import_claude_code(&src, &mut s, &blocks).unwrap();
    assert!(report.authored);
    let (path, _) = s.path().unwrap();
    assert_eq!(path.len(), 3);
    assert!(path[1].is_custom(CUSTOM_AUTHORED));
    let EntryBody::Message { message } = &path[2].body else {
        panic!()
    };
    assert_eq!(
        (
            message["provider"].as_str(),
            message["api"].as_str(),
            message["model"].as_str()
        ),
        (Some("authored"), Some("authored"), Some("authored"))
    );
    // An orphan.
    let bad = dir.path().join("b.jsonl");
    std::fs::write(
        &bad,
        rec(
            U3,
            Some("99999999-9999-4999-8999-999999999999"),
            "user",
            &json!({"role": "user", "content": "?"}),
        ) + "\n",
    )
    .unwrap();
    let mut s2 = home.create_session("orphan", "/w", None).unwrap();
    let err = import_claude_code(&bad, &mut s2, &blocks)
        .unwrap_err()
        .to_string();
    assert!(err.contains(U3) && err.contains("99999999"), "{err}");
    assert_eq!(s2.len(), 0);
    drop((s, s2));
    dir.close().unwrap();
}
