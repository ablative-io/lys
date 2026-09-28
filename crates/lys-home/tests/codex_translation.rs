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

use serde_json::{Value, json};

use lys_home::Hash;
use lys_home::cli::{Cli, Command, run};
use lys_home::cli_translate::TranslateArgs;

use terms::{Walk, off_path_losses, on_path_losses, sidechain_losses};

#[path = "codex_translation/kinds.rs"]
mod kinds;

use kinds::{add_beside_and_at_head, every_kind_fixture};

#[path = "codex_translation/terms.rs"]
mod terms;

type Gate = Result<(), Box<dyn Error>>;

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

#[test]
fn second_translation_gives_equal_bytes() -> Gate {
    let (records, head) = every_kind_fixture();
    let (dir, home) = import(&records)?;
    add_beside_and_at_head(&home, &head)?;
    let (first_lines, first, first_path) = translate(&home, &dir.path().join("o1"))?;
    let (second_lines, mut second, second_path) = translate(&home, &dir.path().join("o2"))?;
    assert_eq!(
        serde_json::to_vec(&first_lines)?,
        serde_json::to_vec(&second_lines)?
    );
    let lost = second["lost"].as_array_mut().ok_or("no lost")?;
    let before = lost.len();
    lost.retain(|row| row["kind"] != "lys.translation");
    assert_eq!(lost.len() + 1, before);
    assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);
    assert_eq!(sha(&first_path)?, sha(&second_path)?);
    Ok(())
}

#[test]
fn claude_code_render_is_unchanged_by_hash() -> Gate {
    let (records, head) = every_kind_fixture();
    let (dir, home) = import(&records)?;
    add_beside_and_at_head(&home, &head)?;
    let render = |name: &str| -> Result<PathBuf, Box<dyn Error>> {
        let out = dir.path().join(name);
        run(Cli {
            command: Command::Render {
                home: home.clone(),
                session: "s1".to_owned(),
                uuid: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned(),
                cwd: "/w".to_owned(),
                model: "claude-fixture".to_owned(),
                out: Some(out.clone()),
                version: "2.1.281".to_owned(),
                canon: None,
            },
        })?;
        Ok(out)
    };
    let a = render("a.jsonl")?;
    translate(&home, &dir.path().join("o"))?;
    let b = render("b.jsonl")?;
    assert_eq!(sha(&a)?, sha(&b)?);
    Ok(())
}
