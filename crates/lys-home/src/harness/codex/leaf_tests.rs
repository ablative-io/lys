//! Gates on the side leaf (HOME-009 R6): one `lys.translation` entry beside
//! the context path naming the translation by hashes, the head and every
//! earlier byte of the session unchanged, and a second translation listing
//! the first one's leaf as lost.

use std::error::Error;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::harness::codex::rollout_tests::{entry, home_with, read_json, run, said};
use crate::record::Home;
use crate::record::blocks::hex_of;
use crate::record::entries::{CUSTOM_TRANSLATION, Entry, EntryBody};

type Gate = Result<(), Box<dyn Error>>;

const STAMP: &str = "2000-01-02T03:04:05.678Z";

fn two_entries() -> Result<(tempfile::TempDir, Home), Box<dyn Error>> {
    home_with(
        "s1",
        &[
            entry("u1", None, STAMP, said("user", "one")),
            entry("a1", Some("u1"), STAMP, said("assistant", "two")),
        ],
    )
}

fn sha256(bytes: &[u8]) -> String {
    hex_of(&Sha256::digest(bytes))
}

fn leaves(home: &Home) -> Result<Vec<Entry>, Box<dyn Error>> {
    Ok(home
        .read_session("s1")?
        .customs_everywhere(CUSTOM_TRANSLATION)?)
}

fn data(leaf: &Entry) -> Value {
    match &leaf.body {
        EntryBody::Custom {
            data: Some(data), ..
        } => data.clone(),
        _ => Value::Null,
    }
}

#[test]
fn side_leaf_names_the_translation() -> Gate {
    let (dir, home) = two_entries()?;
    let (before_hash, file) = {
        let s = home.open_session("s1")?;
        (s.head_hash()?, s.file().to_path_buf())
    };
    let before = std::fs::read(&file)?;
    let done = run(&home, "s1", &dir.path().join("o"))?;
    let s = home.open_session("s1")?;
    assert_eq!(s.head()?, Some("a1"));
    assert_eq!(s.head_hash()?, before_hash);
    let after = std::fs::read(&file)?;
    assert_eq!(&after[..before.len()], before.as_slice());
    let leaves = leaves(&home)?;
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].parent_id(), Some("a1"));
    let data = data(&leaves[0]);
    assert_eq!(
        data["rollout_sha256"],
        json!(sha256(&std::fs::read(&done.rollout)?))
    );
    assert_eq!(
        data["account_sha256"],
        json!(sha256(&std::fs::read(&done.account)?))
    );
    assert_eq!(data["head_hash"], json!(before_hash.to_string()));
    let name = done
        .rollout
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("no file name")?;
    let thread = data["thread"].as_str().ok_or("no thread")?;
    assert!(name.ends_with(&format!("-{thread}.jsonl")));
    assert_eq!(thread, done.thread);
    Ok(())
}

#[test]
fn leaf_data_holds_eight_keys() -> Gate {
    let (dir, home) = two_entries()?;
    let out = dir.path().join("o");
    let done = run(&home, "s1", &out)?;
    let leaves = leaves(&home)?;
    let data = data(&leaves[0]);
    let mut keys: Vec<&String> = data
        .as_object()
        .into_iter()
        .flat_map(|o| o.keys())
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "account_sha256",
            "codex_version",
            "harness",
            "head",
            "head_hash",
            "rollout",
            "rollout_sha256",
            "thread"
        ]
    );
    assert_eq!(data["harness"], "codex");
    assert_eq!(data["codex_version"], "0.156.0");
    assert_eq!(data["head"], "a1");
    let rollout = data["rollout"].as_str().ok_or("no rollout")?;
    assert!(!rollout.starts_with('/'));
    assert_eq!(out.join(rollout), done.rollout);
    Ok(())
}

#[test]
fn second_translation_lists_the_first_leaf_lost() -> Gate {
    let (dir, home) = two_entries()?;
    run(&home, "s1", &dir.path().join("o1"))?;
    let first = leaves(&home)?;
    let done = run(&home, "s1", &dir.path().join("o2"))?;
    let account = read_json(&done.account)?;
    let lost: Vec<&Value> = account["lost"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["kind"] == CUSTOM_TRANSLATION)
        .collect();
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0]["entry"], json!(first[0].id()));
    assert_eq!(
        lost[0]["reason"],
        "Codex has no item for it and it is not conversation"
    );
    Ok(())
}
