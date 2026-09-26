#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the Claude Code renderer: the chain, the same-model thinking rule,
//! the loss account, the refusal of an existing path, and the round trip.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::harness::claude_code::import::import_claude_code;
use crate::harness::claude_code::import_tests::fixture;
use crate::harness::claude_code::render::{RenderTarget, record_uuid, render_claude_code};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::{Entry, EntryBase, EntryBody};

fn target(model: &str, out: std::path::PathBuf) -> RenderTarget {
    RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into(),
        cwd: "/elsewhere".into(),
        model: model.into(),
        version: "2.1.281".into(),
        out: Some(out),
        canon: None,
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
    let report = render_claude_code(
        &s,
        &target("claude-opus-5-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap();
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
    let err = render_claude_code(
        &s,
        &target("claude-opus-5-5", out.clone()),
        Some(dir.path()),
    )
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
    let report = render_claude_code(
        &s,
        &target("claude-sonnet-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap();
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

/// A session of message entries with the given ids, each under the one before.
fn session_of(home: &Home, session: &str, ids: &[&str]) -> crate::record::Session {
    let mut s = home.create_session(session, "/w", None).unwrap();
    let mut prev: Option<String> = None;
    for (n, id) in ids.iter().enumerate() {
        let role = if n % 2 == 0 { "user" } else { "assistant" };
        let message = if role == "user" {
            serde_json::json!({"role": "user", "content": [{"type": "text", "text": format!("turn {n}")}], "timestamp": 0})
        } else {
            serde_json::json!({"role": "assistant", "content": [{"type": "text", "text": format!("turn {n}")}],
                "api": "anthropic-messages", "provider": "anthropic", "model": "claude-opus-5-5", "stopReason": "stop", "timestamp": 0})
        };
        s.append_entry(&Entry {
            base: EntryBase {
                id: (*id).to_owned(),
                parent_id: prev.clone(),
                timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
            },
            body: EntryBody::Message { message },
        })
        .unwrap();
        prev = Some((*id).to_owned());
    }
    s
}

#[test]
fn a_session_renders_byte_identically_twice_with_distinct_chained_uuids() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = session_of(&home, "det", &["e1", "e2", "e3", "e4"]);
    let first = dir.path().join("first").join("r.jsonl");
    let second = dir.path().join("second").join("r.jsonl");
    let a = render_claude_code(
        &s,
        &target("claude-opus-5-5", first.clone()),
        Some(dir.path()),
    )
    .unwrap();
    let b = render_claude_code(
        &s,
        &target("claude-opus-5-5", second.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!((a.records, b.records), (4, 4));
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap()
    );
    assert_eq!(
        std::fs::read(&a.loss_path).unwrap(),
        std::fs::read(&b.loss_path).unwrap()
    );
    let lines: Vec<Value> = std::fs::read_to_string(&first)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 4);
    let uuids: std::collections::BTreeSet<&str> =
        lines.iter().map(|l| l["uuid"].as_str().unwrap()).collect();
    assert_eq!(uuids.len(), 4);
    for u in &uuids {
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        assert_eq!(&u[19..20], "8");
    }
    assert_eq!(lines[0]["parentUuid"], Value::Null);
    for pair in lines.windows(2) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
    }
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_uuid_shaped_entry_id_is_kept_and_any_other_maps_to_one_fixed_uuid() {
    const KEPT: &str = "5f0c0b8e-2a1d-4c3b-9e7f-0123456789ab";
    assert_eq!(record_uuid(KEPT), KEPT);
    assert_eq!(record_uuid("e1"), record_uuid("e1"));
    assert_ne!(record_uuid("e1"), record_uuid("e2"));
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = session_of(&home, "kept", &["e1", KEPT]);
    let out = dir.path().join("kept.jsonl");
    let report = render_claude_code(
        &s,
        &target("claude-opus-5-5", out.clone()),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!(report.records, 2);
    let lines: Vec<Value> = std::fs::read_to_string(&out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[1]["uuid"], KEPT);
    assert_eq!(lines[1]["parentUuid"], lines[0]["uuid"]);
    assert_ne!(lines[0]["uuid"], "e1");
    drop(s);
    dir.close().unwrap();
}

#[test]
fn record_uuid_is_the_documented_selection_of_the_sha256_of_the_entry_id() {
    use std::fmt::Write as _;
    let digest = Sha256::digest(b"fixed-entry-id");
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    });
    assert_eq!(hex.len(), 64);
    let expected = format!(
        "{}-{}-4{}-8{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    );
    assert_eq!(record_uuid("fixed-entry-id"), expected);
    assert_eq!(
        record_uuid("fixed-entry-id"),
        "69392357-b162-4c19-8db0-8c73793670c5"
    );
}

#[test]
fn a_render_with_no_out_and_no_home_directory_is_refused_by_name_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = session_of(&home, "noplace", &["e1", "e2"]);
    let mut target = target("claude-opus-5-5", dir.path().join("unused.jsonl"));
    target.out = None;
    let before: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let err = render_claude_code(&s, &target, None).unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::NoRenderPlace),
        "{err}"
    );
    assert!(err.to_string().contains("--out"), "{err}");
    let after: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(before, after);
    assert_eq!(before.len(), 1, "only the home");
    let report = render_claude_code(&s, &target, Some(dir.path())).unwrap();
    assert!(
        report
            .path
            .starts_with(dir.path().join(".claude").join("projects"))
    );
    assert_eq!(report.records, 2);
    drop(s);
    dir.close().unwrap();
}
