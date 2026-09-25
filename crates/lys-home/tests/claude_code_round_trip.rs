#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The Claude Code round trip through the public crate (HOME-001 R5): a
//! transcript imported, rendered for the same model, and imported again keeps
//! every message part by hash; a rendered file resumed by Claude Code must not
//! repeat the tool actions it already holds, and `resume-check` refuses a fork
//! that does; an authored file's boundary to the real turn survives the trip.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use lys_home::cli::{Cli, Command, run};
use lys_home::harness::claude_code::import::import_claude_code;
use lys_home::harness::claude_code::render::{RenderTarget, render_claude_code};
use lys_home::{EntryBody, Hash, Home, HomeError, Session};

const U: [&str; 6] = [
    "11111111-1111-4111-8111-111111111111",
    "22222222-2222-4222-8222-222222222222",
    "33333333-3333-4333-8333-333333333333",
    "44444444-4444-4444-8444-444444444444",
    "55555555-5555-4555-8555-555555555555",
    "66666666-6666-4666-8666-666666666666",
];

fn rec(uuid: &str, parent: Option<&str>, kind: &str, message: &Value) -> Value {
    json!({
        "parentUuid": parent, "isSidechain": false, "userType": "external", "cwd": "/w",
        "sessionId": "s", "version": "2.1.281", "gitBranch": "", "uuid": uuid,
        "timestamp": "2026-09-24T03:30:00.000Z", "type": kind, "message": message
    })
}

fn assistant(model: &str, id: &str, content: &Value, stop: &str) -> Value {
    json!({"id": id, "type": "message", "role": "assistant", "model": model, "content": content,
        "stop_reason": stop, "stop_sequence": null, "usage": {"input_tokens": 1, "output_tokens": 1}})
}

/// A question, a signed-thinking tool call, a hook, the result, the answer.
fn transcript() -> Vec<Value> {
    vec![
        rec(
            U[0],
            None,
            "user",
            &json!({"role": "user", "content": "what is in the box?"}),
        ),
        rec(
            U[1],
            Some(U[0]),
            "assistant",
            &assistant(
                "claude-opus-5-5",
                "msg_1",
                &json!([
                    {"type": "thinking", "thinking": "look first", "signature": "sig-abc"},
                    {"type": "tool_use", "id": "toolu_1", "name": "Read", "input": {"file_path": "/box"}}
                ]),
                "tool_use",
            ),
        ),
        json!({"type": "attachment", "uuid": "a1", "parentUuid": U[1], "isSidechain": false,
            "timestamp": "2026-09-24T03:30:01.000Z",
            "attachment": {"type": "hook_success", "hookName": "gate", "exitCode": 0, "stdout": "", "stderr": ""}}),
        rec(
            U[2],
            Some("a1"),
            "user",
            &json!({"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "a walrus", "is_error": false}]}),
        ),
        rec(
            U[3],
            Some(U[2]),
            "assistant",
            &assistant(
                "claude-opus-5-5",
                "msg_2",
                &json!([{"type": "text", "text": "a walrus"}]),
                "end_turn",
            ),
        ),
    ]
}

fn write_jsonl(path: &Path, records: &[Value]) {
    let body: String = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap() + "\n")
        .collect();
    std::fs::write(path, body).unwrap();
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn target(model: &str, out: PathBuf) -> RenderTarget {
    RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into(),
        cwd: "/elsewhere".into(),
        model: model.into(),
        version: "2.1.281".into(),
        out: Some(out),
        canon: None,
    }
}

/// The hash of every message's content on the context path, in order, plus
/// the provider of each assistant message.
fn message_shape(s: &Session) -> Vec<(String, String)> {
    s.context_path()
        .unwrap()
        .iter()
        .filter_map(|e| match &e.body {
            EntryBody::Message { message } => Some((
                Hash::of(&serde_json::to_vec(&message["content"]).unwrap()).to_string(),
                message["provider"].as_str().unwrap_or("").to_owned(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn import_render_import_keeps_every_message_part_and_the_signature() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("t.jsonl");
    write_jsonl(&src, &transcript());
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut first = home.create_session("first", "/w", None).unwrap();
    let a = import_claude_code(&src, &mut first, &blocks).unwrap();
    assert_eq!(a.events.get("hook"), Some(&1));
    let out = dir.path().join("rendered.jsonl");
    let rendered =
        render_claude_code(&first, &target("claude-opus-5-5", out.clone()), dir.path()).unwrap();
    assert_eq!(
        (rendered.records, rendered.thinking_kept, rendered.dropped),
        (4, 1, 0)
    );
    let lines = read_jsonl(&out);
    assert_eq!(lines[1]["message"]["content"][0]["signature"], "sig-abc");
    let mut prev: Option<&str> = None;
    for l in &lines {
        assert_eq!(
            l["parentUuid"].as_str(),
            prev,
            "the rendered file is one chain"
        );
        prev = l["uuid"].as_str();
    }
    let mut second = home.create_session("second", "/w", None).unwrap();
    let b = import_claude_code(&out, &mut second, &blocks).unwrap();
    assert_eq!(
        b.blocks_new, 1,
        "every part is already held except the tool result, which the renderer writes in block form (the one shape the importer keeps) where the source had a string"
    );
    assert_eq!(message_shape(&first), message_shape(&second));
    drop((first, second));
    dir.close().unwrap();
}

#[test]
fn resume_check_refuses_a_fork_that_repeats_a_tool_action() {
    let dir = tempfile::tempdir().unwrap();
    let rendered = dir.path().join("rendered.jsonl");
    write_jsonl(&rendered, &transcript());
    // A fork that continues with a new tool action passes.
    let mut fresh = transcript();
    fresh.push(rec(
        U[4],
        Some(U[3]),
        "user",
        &json!({"role": "user", "content": "and the lid?"}),
    ));
    fresh.push(rec(
        U[5],
        Some(U[4]),
        "assistant",
        &assistant(
            "claude-opus-5-5",
            "msg_3",
            &json!([{"type": "tool_use", "id": "toolu_2", "name": "Read", "input": {"file_path": "/lid"}}]),
            "tool_use",
        ),
    ));
    let fork_ok = dir.path().join("fork-ok.jsonl");
    write_jsonl(&fork_ok, &fresh);
    let report = run(Cli {
        command: Command::ResumeCheck {
            rendered: rendered.clone(),
            forked: fork_ok,
        },
    })
    .unwrap();
    assert_eq!(report["report"]["repeated_tool_use_ids"], 0);
    assert_eq!(report["report"]["forked_new_records"], 2);
    assert_eq!(
        report["report"]["new_tool_uses"], 1,
        "the fork's own new action is counted, not refused"
    );
    // A fork that runs toolu_1 again is refused, and says how many.
    let mut again = transcript();
    again.push(rec(
        U[4],
        Some(U[3]),
        "user",
        &json!({"role": "user", "content": "again?"}),
    ));
    again.push(rec(
        U[5],
        Some(U[4]),
        "assistant",
        &assistant(
            "claude-opus-5-5",
            "msg_3",
            &json!([{"type": "tool_use", "id": "toolu_1", "name": "Read", "input": {"file_path": "/box"}}]),
            "tool_use",
        ),
    ));
    let fork_bad = dir.path().join("fork-bad.jsonl");
    write_jsonl(&fork_bad, &again);
    let err = run(Cli {
        command: Command::ResumeCheck {
            rendered,
            forked: fork_bad,
        },
    })
    .unwrap_err();
    assert!(
        matches!(err, HomeError::RepeatedToolActions { count: 1 }),
        "{err}"
    );
    dir.close().unwrap();
}

#[test]
fn the_authored_boundary_survives_fewshot_import_and_render() {
    let dir = tempfile::tempdir().unwrap();
    let turns = dir.path().join("turns.txt");
    std::fs::write(
        &turns,
        "user: Rule: one lowercase word.\nassistant: understood\nuser: The codeword is basalt. Say it back.\nassistant: basalt\nuser: Again.\nassistant: basalt\n",
    )
    .unwrap();
    let authored = dir.path().join("src").join("authored.jsonl");
    let report = run(Cli {
        command: Command::Fewshot {
            out: authored.clone(),
            turns,
            cwd: "/authored".into(),
        },
    })
    .unwrap();
    assert_eq!(report["records"], 6);
    assert!(
        report["resume"]
            .as_str()
            .unwrap()
            .starts_with("claude --resume ")
    );
    let mut records = read_jsonl(&authored);
    assert_eq!(records.len(), 6);
    assert!(
        records
            .iter()
            .filter(|r| r["type"] == "assistant")
            .all(|r| r["message"]["model"] == "authored")
    );
    // What Claude Code 2.1.281 writes beside the file on resume: the six copied, then the real turn.
    let last = records[5]["uuid"].as_str().unwrap().to_owned();
    records.push(rec(
        U[4],
        Some(&last),
        "user",
        &json!({"role": "user", "content": "What is the codeword?"}),
    ));
    records.push(rec(
        U[5],
        Some(U[4]),
        "assistant",
        &assistant(
            "claude-opus-5-5",
            "msg_real",
            &json!([{"type": "text", "text": "basalt"}]),
            "end_turn",
        ),
    ));
    let continuation = dir.path().join("src").join("continuation.jsonl");
    write_jsonl(&continuation, &records);
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("cont", "/authored", None).unwrap();
    let imported = import_claude_code(&continuation, &mut s, &blocks).unwrap();
    assert!(imported.authored);
    let providers: Vec<String> = message_shape(&s).into_iter().map(|(_, p)| p).collect();
    assert_eq!(
        providers,
        [
            "",
            "authored",
            "",
            "authored",
            "",
            "authored",
            "",
            "anthropic"
        ],
        "the boundary is a change of provider on the path"
    );
    assert_eq!(s.customs("lys.authored").unwrap().len(), 1);
    let out = dir.path().join("rendered.jsonl");
    let rendered =
        render_claude_code(&s, &target("claude-opus-5-5", out.clone()), dir.path()).unwrap();
    assert!(rendered.authored);
    let lines = read_jsonl(&out);
    let models: Vec<&str> = lines
        .iter()
        .filter(|l| l["type"] == "assistant")
        .map(|l| l["message"]["model"].as_str().unwrap())
        .collect();
    assert_eq!(
        models,
        ["authored", "authored", "authored", "claude-opus-5-5"]
    );
    drop(s);
    dir.close().unwrap();
}
