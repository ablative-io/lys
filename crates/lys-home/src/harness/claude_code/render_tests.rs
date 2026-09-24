#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the Claude Code renderer: the chain, the same-model thinking rule,
//! the loss account, the refusal of an existing path, and the round trip.

use serde_json::Value;

use crate::harness::claude_code::import::import_claude_code;
use crate::harness::claude_code::import_tests::fixture;
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::EntryBody;

fn target(model: &str, out: std::path::PathBuf) -> RenderTarget {
    RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into(),
        cwd: "/elsewhere".into(),
        model: model.into(),
        version: "2.1.281".into(),
        out: Some(out),
    }
}

fn message_hashes(s: &crate::record::Session) -> Vec<String> {
    s.context_path()
        .unwrap()
        .iter()
        .filter_map(|e| match &e.body {
            EntryBody::Message { message } => {
                Some(Hash::of(&serde_json::to_vec(&message["content"]).unwrap()).to_string())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn same_model_keeps_signed_thinking_and_the_round_trip_keeps_every_block() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("t.jsonl");
    std::fs::write(&src, fixture()).unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("one", "/w", None).unwrap();
    import_claude_code(&src, &mut s, &blocks).unwrap();
    let out = dir.path().join("out").join("same.jsonl");
    let report =
        render_claude_code(&s, &target("claude-opus-5-5", out.clone()), dir.path()).unwrap();
    assert_eq!(
        (
            report.records,
            report.thinking_kept,
            report.dropped,
            report.authored
        ),
        (4, 1, 0, false)
    );
    let lines: Vec<Value> = std::fs::read_to_string(&out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[0]["parentUuid"], Value::Null);
    for pair in lines.windows(2) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
        assert_eq!(pair[1]["sessionId"], "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");
    }
    assert_eq!(lines[1]["message"]["content"][0]["signature"], "sig-abc");
    assert_eq!(lines[2]["message"]["content"][0]["type"], "tool_result");
    // Re-import the rendered file: the same content hashes on the path.
    let mut again = home.create_session("two", "/w", None).unwrap();
    import_claude_code(&out, &mut again, &blocks).unwrap();
    assert_eq!(message_hashes(&s), message_hashes(&again));
    // Existing path refused, bytes untouched.
    let before = std::fs::read(&out).unwrap();
    let err = render_claude_code(&s, &target("claude-opus-5-5", out.clone()), dir.path())
        .unwrap_err()
        .to_string();
    assert!(err.contains("same.jsonl"), "{err}");
    assert_eq!(std::fs::read(&out).unwrap(), before);
    drop((s, again));
    dir.close().unwrap();
}

#[test]
fn a_different_model_gets_thinking_as_text_and_a_loss_account_entry() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("t.jsonl");
    std::fs::write(&src, fixture()).unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.create_session("one", "/w", None).unwrap();
    import_claude_code(&src, &mut s, &blocks).unwrap();
    let out = dir.path().join("other.jsonl");
    let report =
        render_claude_code(&s, &target("claude-sonnet-5", out.clone()), dir.path()).unwrap();
    assert_eq!(
        (
            report.thinking_kept,
            report.thinking_as_text,
            report.dropped
        ),
        (0, 1, 1)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(!text.contains("\"signature\""));
    let account: Value = serde_json::from_slice(&std::fs::read(report.loss_path).unwrap()).unwrap();
    assert_eq!(account["dropped"].as_array().unwrap().len(), 1);
    assert_eq!(account["dropped"][0]["hash"].as_str().unwrap().len(), 64);
    assert_eq!(account["authored"], false);
    drop(s);
    dir.close().unwrap();
}
