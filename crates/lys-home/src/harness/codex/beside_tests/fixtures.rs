#![cfg(test)]
//! The records these tests import: a main chain, a sidechain, and the home they
//! are imported into and translated from.

use std::error::Error;
use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cli::{Cli, Command, run as cli_run};
use crate::harness::codex::rollout::translate;
use crate::harness::codex::rollout_tests::{lines, read_json};
use crate::record::Home;
use crate::record::entries::EntryBody;

use super::{A1, Gate, LABEL_STAMP, S1, S2, U1};

pub(super) fn image() -> Value {
    json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}})
}

/// Which chain a record is on.
#[derive(Clone, Copy)]
pub(super) enum Chain<'a> {
    /// The main chain.
    Main,
    /// A sidechain whose records carry this agent id.
    Agent(&'a str),
    /// A sidechain whose records carry no agent id.
    Unlabelled,
}

/// One Claude Code record on the chain named.
pub(super) fn rec(uuid: &str, parent: Option<&str>, chain: Chain<'_>, message: &Value) -> Value {
    let role = message["role"].as_str().map_or("user", |r| r);
    let side = !matches!(chain, Chain::Main);
    let stamp = if side && parent.is_none() {
        LABEL_STAMP
    } else {
        "2026-01-01T00:00:01.000Z"
    };
    let mut record = json!({"parentUuid": parent, "isSidechain": side, "userType": "external",
        "cwd": "/w", "sessionId": "s", "version": "2.1.281", "uuid": uuid, "timestamp": stamp,
        "type": role, "message": message});
    if let Chain::Agent(agent) = chain {
        record["agentId"] = json!(agent);
    }
    record
}

pub(super) fn user(content: &Value) -> Value {
    json!({"role": "user", "content": content})
}

pub(super) fn assistant(content: &Value) -> Value {
    json!({"role": "assistant", "model": "claude-fixture", "content": content,
        "stop_reason": "end_turn", "usage": {"input_tokens": 1, "output_tokens": 1}})
}

pub(super) fn main_chain() -> Vec<Value> {
    vec![
        rec(U1, None, Chain::Main, &user(&json!("fixture question"))),
        rec(
            A1,
            Some(U1),
            Chain::Main,
            &assistant(&json!([{"type": "text", "text": "fixture answer"}])),
        ),
    ]
}

/// Import the records into session `s1` of a fresh home.
pub(crate) fn import(records: &[Value]) -> Result<(TempDir, Home), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source.jsonl");
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    std::fs::write(&source, text)?;
    let home = dir.path().join("home");
    cli_run(Cli {
        command: Command::Import {
            home: home.clone(),
            claude_code: source,
            session: "s1".to_owned(),
        },
    })?;
    Ok((dir, Home::read(home)?))
}

/// Move the head of `session` back to the main chain's leaf.
pub(crate) fn settle(home: &Home, session: &str, leaf: &str) -> Gate {
    home.open_session(session)?.move_head(Some(leaf))?;
    Ok(())
}

/// Translate `session` into `out` and read the rollout's lines and the account.
pub(super) fn translated(
    home: &Home,
    session: &str,
    out: &Path,
) -> Result<(Vec<Value>, Value), Box<dyn Error>> {
    let mut s = home.open_session(session)?;
    let done = translate(&mut s, out, "0.156.0", Some("UTC"))?;
    Ok((lines(&done.rollout)?, read_json(&done.account)?))
}

pub(super) fn rows<'a>(account: &'a Value, list: &str) -> Vec<&'a Value> {
    account[list].as_array().into_iter().flatten().collect()
}

/// The id of the `agent <name>` label entry.
pub(super) fn agent_label(home: &Home, name: &str) -> Result<String, Box<dyn Error>> {
    let reader = home.read_session("s1")?;
    for id in reader.ids() {
        if let EntryBody::Label {
            label: Some(label), ..
        } = reader.entry(id)?.body
            && label == format!("agent {name}")
        {
            return Ok(id.to_owned());
        }
    }
    Err("no agent label".into())
}

pub(super) fn sidechain_fixture() -> Vec<Value> {
    let mut records = main_chain();
    records.push(rec(
        S1,
        None,
        Chain::Agent("a1"),
        &user(&json!("fixture sub-question")),
    ));
    records.push(rec(
        S2,
        Some(S1),
        Chain::Agent("a1"),
        &assistant(&json!([{"type": "text", "text": "fixture sub-answer"}])),
    ));
    records
}
