//! Gates on the loss account's shape (HOME-009 R2): exactly the eight
//! top-level keys and each row's own keys, a row about an entry with a null
//! hash, and the part hash equal to one computed outside this crate.

use std::error::Error;

use serde_json::{Value, json};

use crate::harness::codex::account::{Account, Rows, part_hash};

type Gate = Result<(), Box<dyn Error>>;

fn keys(value: &Value) -> Result<Vec<String>, Box<dyn Error>> {
    let object = value.as_object().ok_or("not an object")?;
    Ok(object.keys().cloned().collect())
}

fn first(value: &Value, list: &str) -> Result<Value, Box<dyn Error>> {
    let rows = value[list].as_array().ok_or("not a list")?;
    Ok(rows.first().ok_or("the list holds no row")?.clone())
}

const TOP: [&str; 8] = [
    "changed",
    "codex_version",
    "head",
    "head_hash",
    "kept",
    "lost",
    "session",
    "thread",
];
const KEPT: [&str; 4] = ["after", "before", "entry", "hash"];
const CHANGED: [&str; 5] = ["after", "before", "entry", "hash", "how"];
const LOST: [&str; 4] = ["entry", "hash", "kind", "reason"];

#[test]
fn account_holds_the_three_lists() -> Gate {
    let mut rows = Rows::default();
    rows.keep("e1", Some("a".repeat(64)), "text", "input_text");
    let how = "timestamp not carried";
    rows.change("e2", None, "user message", "message item", how);
    rows.lose("e3", Some("b".repeat(64)), "image", "a fixed reason");
    let account = Account {
        session: "s1".to_owned(),
        head: "e3".to_owned(),
        head_hash: "c".repeat(64),
        thread: "t".to_owned(),
        codex_version: "0.156.0".to_owned(),
        kept: rows.kept,
        changed: rows.changed,
        lost: rows.lost,
    };
    let value = serde_json::to_value(&account)?;
    assert_eq!(keys(&value)?, TOP);
    assert_eq!(keys(&first(&value, "kept")?)?, KEPT);
    let changed = first(&value, "changed")?;
    assert_eq!(keys(&changed)?, CHANGED);
    assert_eq!(changed["hash"], Value::Null);
    assert_eq!(keys(&first(&value, "lost")?)?, LOST);
    Ok(())
}

#[test]
fn part_hash_is_the_entry_bytes() -> Gate {
    // printf '%s' '{"text":"a","type":"text"}' | shasum -a 256
    assert_eq!(
        part_hash(&json!({"type": "text", "text": "a"}))?,
        "a7be9c132fa673d4eb9460cf2601c67404000175f92f2a576610aec68877d516"
    );
    Ok(())
}
