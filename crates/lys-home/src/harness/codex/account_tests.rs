//! Gates on the loss account (HOME-009 R2): exactly the top-level keys and
//! row keys the brief names, a null hash on a row about an entry, and a part's
//! hash taken over its bytes as the home entry holds it.

use std::error::Error;

use serde_json::{Value, json};

use crate::harness::codex::account::{Account, Rows, part_hash};

type Gate = Result<(), Box<dyn Error>>;

fn keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .iter()
        .flat_map(|object| object.keys().cloned())
        .collect();
    keys.sort();
    keys
}

#[test]
fn account_holds_the_three_lists() -> Gate {
    let part = json!({"type": "text", "text": "a"});
    let mut rows = Rows::default();
    rows.kept("e1", Some(part_hash(&part)), "text", "input_text");
    rows.changed(
        "e2",
        None,
        "user message",
        "message item",
        "timestamp not carried",
    );
    rows.lost(
        "e3",
        Some(part_hash(&part)),
        "thinking",
        "empty thinking dropped",
    );
    let account = Account {
        session: "s1".to_owned(),
        head: "e3".to_owned(),
        head_hash: "h".to_owned(),
        thread: "t".to_owned(),
        codex_version: "0.156.0".to_owned(),
        rows,
    };
    let value = serde_json::to_value(&account)?;
    assert_eq!(
        keys(&value),
        [
            "changed",
            "codex_version",
            "head",
            "head_hash",
            "kept",
            "lost",
            "session",
            "thread"
        ]
    );
    assert_eq!(
        keys(&value["kept"][0]),
        ["after", "before", "entry", "hash"]
    );
    assert_eq!(
        keys(&value["changed"][0]),
        ["after", "before", "entry", "hash", "how"]
    );
    assert_eq!(keys(&value["lost"][0]), ["entry", "hash", "kind", "reason"]);
    assert_eq!(value["changed"][0]["hash"], Value::Null);
    assert_eq!(value["kept"][0]["hash"], json!(part_hash(&part)));
    let back: Account = serde_json::from_value(value)?;
    assert_eq!(back, account);
    Ok(())
}

#[test]
fn part_hash_is_the_entry_bytes() {
    assert_eq!(
        part_hash(&json!({"type": "text", "text": "a"})),
        "a7be9c132fa673d4eb9460cf2601c67404000175f92f2a576610aec68877d516"
    );
}
