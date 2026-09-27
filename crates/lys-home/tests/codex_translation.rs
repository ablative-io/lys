#![cfg(test)]
//! The Codex translation through the public crate (HOME-009 R8): synthetic
//! Claude Code files, carrying no real transcript, imported into a fresh home
//! and translated through `lys-home translate-codex`. Every text part, tool
//! call and tool result on the context path is carried whole, readable
//! foreign thinking becomes text, every row names its kinds and reason, the
//! same head translates to the same bytes, and the Claude Code render of the
//! session is unchanged by the translation.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use lys_home::cli::{Cli, Command, run};
use lys_home::cli_lantern::LanternAction;
use lys_home::cli_translate::TranslateArgs;
use lys_home::{Entry, EntryBase, EntryBody, Hash, Home};

use terms::{Walk, off_path_losses, on_path_losses, sidechain_losses};

#[path = "codex_translation/terms.rs"]
mod terms;

type Gate = Result<(), Box<dyn Error>>;

#[path = "codex_translation/stability.rs"]
mod stability;

const STAMP: &str = "2026-01-01T00:00:00.000Z";

/// The chain a record is on.
#[derive(Clone, Copy)]
enum Chain<'a> {
    Main,
    Agent(&'a str),
    Unlabelled,
}

fn uuid(n: u32) -> String {
    format!("{n:08x}-0000-4000-8000-000000000000")
}

fn rec(id: &str, parent: Option<&str>, chain: Chain<'_>, message: &Value) -> Value {
    let mut record = json!({"parentUuid": parent, "isSidechain": !matches!(chain, Chain::Main),
        "userType": "external", "cwd": "/w", "sessionId": "s", "version": "2.1.281", "uuid": id,
        "timestamp": STAMP, "type": message["role"], "message": message});
    if let Chain::Agent(agent) = chain {
        record["agentId"] = json!(agent);
    }
    record
}

fn user(content: &Value) -> Value {
    json!({"role": "user", "content": content})
}

fn assistant(content: &Value) -> Value {
    json!({"role": "assistant", "model": "claude-fixture", "content": content, "stop_reason": "end_turn"})
}

fn text(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

fn base64_image() -> Value {
    json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}})
}

/// Import the records as session `s1` of a fresh home; the directory and home.
fn import(records: &[Value]) -> Result<(tempfile::TempDir, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut body = String::new();
    for record in records {
        body.push_str(&serde_json::to_string(record)?);
        body.push('\n');
    }
    let source = dir.path().join("source.jsonl");
    std::fs::write(&source, body)?;
    let home = dir.path().join("home");
    run(Cli {
        command: Command::Import {
            home: home.clone(),
            claude_code: source,
            session: "s1".to_owned(),
        },
    })?;
    Ok((dir, home))
}

/// Translate `s1` into `out`; the rollout's lines, the account, and the rollout's path.
fn translate(home: &Path, out: &Path) -> Result<(Vec<Value>, Value, PathBuf), Box<dyn Error>> {
    let printed = run(Cli {
        command: Command::TranslateCodex(TranslateArgs {
            home: home.to_path_buf(),
            session: "s1".to_owned(),
            out: out.to_path_buf(),
            codex_version: "0.156.0".to_owned(),
            zone: Some("UTC".to_owned()),
        }),
    })?;
    let rollout = printed["report"]["rollout"].as_str().ok_or("no rollout")?;
    let account = PathBuf::from(printed["report"]["account"].as_str().ok_or("no account")?);
    let mut lines = Vec::new();
    for line in std::fs::read_to_string(rollout)?.lines() {
        lines.push(serde_json::from_str(line)?);
    }
    Ok((
        lines,
        serde_json::from_slice(&std::fs::read(&account)?)?,
        PathBuf::from(rollout),
    ))
}

/// Every response item's payload.
fn items(lines: &[Value]) -> Vec<&Value> {
    lines
        .iter()
        .filter(|line| line["type"] == "response_item")
        .map(|line| &line["payload"])
        .collect()
}

/// Every `input_text` and `output_text` part's text.
fn texts(lines: &[Value]) -> Vec<String> {
    items(lines)
        .into_iter()
        .flat_map(|item| item["content"].as_array().into_iter().flatten())
        .filter_map(|part| part["text"].as_str().map(str::to_owned))
        .collect()
}

fn rows<'a>(account: &'a Value, list: &str) -> Vec<&'a Value> {
    account[list].as_array().into_iter().flatten().collect()
}

fn sha(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(Hash::of(&std::fs::read(path)?).to_string())
}

fn tool_fixture(output: &str) -> Vec<Value> {
    let (u1, a1, u2, a2) = (uuid(1), uuid(2), uuid(3), uuid(4));
    vec![
        rec(&u1, None, Chain::Main, &user(&json!("fixture question"))),
        rec(
            &a1,
            Some(&u1),
            Chain::Main,
            &assistant(&json!([text("fixture look"),
            {"type": "tool_use", "id": "toolu_1", "name": "Bash", "input": {"command": "ls -la", "n": 2}}])),
        ),
        rec(
            &u2,
            Some(&a1),
            Chain::Main,
            &user(&json!([
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": output, "is_error": false}])),
        ),
        rec(
            &a2,
            Some(&u2),
            Chain::Main,
            &assistant(&json!([text("fixture done")])),
        ),
    ]
}

#[test]
fn text_and_tools_are_carried_whole() -> Gate {
    let (dir, home) = import(&tool_fixture("fixture output\nsecond line"))?;
    let (lines, _, _) = translate(&home, &dir.path().join("o"))?;
    let texts = texts(&lines);
    for said in ["fixture question", "fixture look", "fixture done"] {
        assert!(texts.iter().any(|t| t == said), "{said}");
    }
    let items = items(&lines);
    let call = items
        .iter()
        .find(|item| item["type"] == "function_call")
        .ok_or("no call")?;
    let arguments: Value = serde_json::from_str(call["arguments"].as_str().ok_or("no arguments")?)?;
    assert_eq!(arguments, json!({"command": "ls -la", "n": 2}));
    assert_eq!(call["call_id"], "toolu_1");
    let output = items
        .iter()
        .find(|item| item["type"] == "function_call_output")
        .ok_or("no output")?;
    assert_eq!(output["output"], "fixture output\nsecond line");
    Ok(())
}

#[test]
fn tool_result_past_importer_clip_is_whole() -> Gate {
    let long: String = "abcdefghij".repeat(1_000);
    assert_eq!(long.chars().count(), 10_000);
    let (dir, home) = import(&tool_fixture(&long))?;
    let (lines, _, _) = translate(&home, &dir.path().join("o"))?;
    let output = items(&lines)
        .into_iter()
        .find(|item| item["type"] == "function_call_output")
        .ok_or("no output")?;
    assert_eq!(output["output"].as_str(), Some(long.as_str()));
    Ok(())
}

#[test]
fn foreign_thinking_renders_as_text() -> Gate {
    let (u1, a1) = (uuid(1), uuid(2));
    let records = [
        rec(&u1, None, Chain::Main, &user(&json!("fixture question"))),
        rec(
            &a1,
            Some(&u1),
            Chain::Main,
            &assistant(&json!([
            {"type": "thinking", "thinking": "fixture reasoning", "signature": "fixture-signature-bytes"},
            {"type": "redacted_thinking", "data": "fixture-redacted-bytes"},
            text("fixture answer")])),
        ),
    ];
    let (dir, home) = import(&records)?;
    let (lines, account, _) = translate(&home, &dir.path().join("o"))?;
    assert!(texts(&lines).iter().any(|t| t == "fixture reasoning"));
    let rollout = serde_json::to_string(&lines)?;
    assert!(!rollout.contains("fixture-signature-bytes"));
    assert!(!rollout.contains("fixture-redacted-bytes"));
    assert!(
        rows(&account, "changed")
            .iter()
            .any(|row| row["before"] == "thinking" && row["after"] == "output_text")
    );
    assert!(
        rows(&account, "lost")
            .iter()
            .any(|row| row["reason"] == "redacted thinking dropped: another provider")
    );
    Ok(())
}

/// The fixture of `every_row_names_its_kinds_and_reason`, and the main
/// chain's last message, where a live session's head stands.
fn every_kind_fixture() -> (Vec<Value>, String) {
    let id = |n: u32| uuid(n);
    let records = vec![
        rec(&id(1), None, Chain::Main, &user(&json!("fixture question"))),
        rec(
            &id(2),
            Some(&id(1)),
            Chain::Main,
            &assistant(&json!([text("fixture answer")])),
        ),
        rec(
            &id(10),
            None,
            Chain::Agent("b1"),
            &user(&json!("fixture b1 task")),
        ),
        rec(
            &id(11),
            Some(&id(10)),
            Chain::Agent("b1"),
            &assistant(&json!([
            {"type": "tool_use", "id": "toolu_b", "name": "Bash", "input": {}}])),
        ),
        rec(
            &id(12),
            Some(&id(11)),
            Chain::Agent("b1"),
            &user(&json!([
            {"type": "tool_result", "tool_use_id": "toolu_b", "content": "fixture b1 out"}])),
        ),
        json!({"type": "summary", "summary": "fixture summary", "leafUuid": id(2), "timestamp": STAMP}),
        json!({"type": "system", "subtype": "compact_boundary", "uuid": id(20), "parentUuid": null,
            "isSidechain": false, "timestamp": STAMP}),
        rec(
            &id(21),
            Some(&id(20)),
            Chain::Main,
            &user(&json!([text("fixture after"),
            {"type": "image", "source": {"type": "url", "url": "https://example.invalid/a.png"}},
            {"type": "document", "source": {"type": "text", "data": "fixture document"}}])),
        ),
        rec(
            &id(22),
            Some(&id(21)),
            Chain::Main,
            &assistant(&json!([
            {"type": "redacted_thinking", "data": "fixture-redacted"},
            {"type": "thinking", "thinking": ""},
            {"type": "tool_use", "name": "Bash", "input": {}}])),
        ),
        rec(
            &id(23),
            Some(&id(22)),
            Chain::Main,
            &user(&json!([
            {"type": "tool_result", "content": "fixture orphan"}])),
        ),
        json!({"type": "attachment", "uuid": id(24), "parentUuid": id(23), "isSidechain": false,
            "timestamp": STAMP, "attachment": {"type": "skill_listing"}}),
        rec(
            &id(25),
            Some(&id(24)),
            Chain::Main,
            &assistant(&json!([text("fixture last")])),
        ),
        rec(
            &id(30),
            None,
            Chain::Agent("a1"),
            &assistant(&json!([
            {"type": "redacted_thinking", "data": "fixture-a1-redacted"},
            {"type": "tool_use", "id": "toolu_a", "name": "Bash", "input": {}}])),
        ),
        rec(
            &id(31),
            Some(&id(30)),
            Chain::Agent("a1"),
            &user(&json!([
            {"type": "tool_result", "tool_use_id": "toolu_a", "content": "fixture a1 out"},
            base64_image()])),
        ),
        json!({"type": "attachment", "uuid": id(32), "parentUuid": id(31), "isSidechain": true,
            "timestamp": STAMP, "attachment": {"type": "skill_listing"}}),
        rec(
            &id(40),
            None,
            Chain::Unlabelled,
            &user(&json!("fixture stray")),
        ),
        rec(
            &id(41),
            Some(&id(40)),
            Chain::Unlabelled,
            &assistant(&json!([text("fixture stray answer")])),
        ),
    ];
    (records, id(25))
}

fn entry(id: &str, parent: &str, body: EntryBody) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: Some(parent.to_owned()),
            timestamp: STAMP.to_owned(),
        },
        body,
    }
}

/// The additions after import: four entries beside the path, three at the
/// head, then one lantern lit at the head.
fn add_beside_and_at_head(home: &Path, head: &str) -> Gate {
    let owned = Home::read(home)?;
    {
        let mut s = owned.open_session("s1")?;
        s.move_head(Some(head))?;
        s.append_beside(EntryBody::Label {
            target_id: head.to_owned(),
            label: Some("note".to_owned()),
        })?;
        s.append_beside(EntryBody::Compaction {
            summary: "fixture off".to_owned(),
            first_kept_entry_id: head.to_owned(),
            tokens_before: 0,
            rest: Map::new(),
        })?;
        s.append_beside(EntryBody::BranchSummary {
            from_id: head.to_owned(),
            summary: "fixture off branch".to_owned(),
            rest: Map::new(),
        })?;
        s.append_beside(EntryBody::ModelChange {
            provider: "fixture".to_owned(),
            model_id: "fixture-model".to_owned(),
        })?;
        s.append_entry(&entry(
            "bs1",
            head,
            EntryBody::BranchSummary {
                from_id: head.to_owned(),
                summary: "fixture branch".to_owned(),
                rest: Map::new(),
            },
        ))?;
        s.append_entry(&entry(
            "cm1",
            "bs1",
            EntryBody::CustomMessage {
                custom_type: "ext".to_owned(),
                content: json!([text("fixture custom"), base64_image()]),
                rest: Map::new(),
            },
        ))?;
        s.append_entry(&entry(
            "bx1",
            "cm1",
            EntryBody::Message {
                message: json!({"role": "bashExecution", "content": "fixture bash"}),
            },
        ))?;
    }
    run(Cli {
        command: Command::Lantern {
            action: LanternAction::Light {
                home: home.to_path_buf(),
                session: "s1".to_owned(),
                point: "bx1".to_owned(),
                note: "fixture note".to_owned(),
                by: "fixture-lighter".to_owned(),
            },
        },
    })?;
    Ok(())
}

#[test]
fn every_row_names_its_kinds_and_reason() -> Gate {
    let (records, head) = every_kind_fixture();
    let (dir, home) = import(&records)?;
    add_beside_and_at_head(&home, &head)?;
    let walk = Walk::read(&home)?;
    let (_, account, _) = translate(&home, &dir.path().join("o"))?;
    for row in rows(&account, "changed") {
        for key in ["before", "after", "how"] {
            assert!(row[key].as_str().is_some_and(|v| !v.is_empty()), "{row}");
        }
    }
    for row in rows(&account, "lost") {
        assert!(
            row["reason"].as_str().is_some_and(|v| !v.is_empty()),
            "{row}"
        );
    }
    let on_path = on_path_losses(&walk);
    let (left, beside) = off_path_losses(&walk);
    let (carried, unlabelled, events) = sidechain_losses(&records, "a1");
    let a1_events = walk
        .off_path()
        .filter(|e| e.is_custom("lys.harness_event"))
        .filter(|e| {
            walk.anchor(e.id())
                .is_some_and(|a| walk.context.contains(&a))
        })
        .count();
    assert_eq!(
        a1_events,
        events + 1,
        "the a1 sidechain's events and the main chain's orphan result's"
    );
    assert_eq!(
        rows(&account, "lost").len(),
        on_path + left + beside + carried + unlabelled
    );
    Ok(())
}
