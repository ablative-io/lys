//! Gates on the side leaf (HOME-009 R6): one `lys.translation` entry under
//! the head per translation, the head, its hash and the file's earlier
//! bytes unmoved, the hashes of what was written, exactly eight data keys
//! with a relative rollout path, and a second translation of the same head
//! listing the first leaf lost.

use std::error::Error;

use serde_json::Value;

use crate::harness::codex::rollout_tests::{Gate, STAMP, account, assistant, run, session_of, user};
use crate::record::Session;
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_TRANSLATION, Entry, EntryBody};

fn leaves(session: &Session) -> Result<Vec<Entry>, Box<dyn Error>> {
    Ok(session.customs_everywhere(CUSTOM_TRANSLATION)?)
}

fn data(entry: &Entry) -> Result<&Value, Box<dyn Error>> {
    match &entry.body {
        EntryBody::Custom { data: Some(data), .. } => Ok(data),
        _ => Err("the leaf holds no data".into()),
    }
}

/// Session `s1` of a fresh home: a user entry, then an assistant entry at
/// the head.
fn two_entries(dir: &std::path::Path) -> Result<Session, Box<dyn Error>> {
    let (_home, session) = session_of(
        dir,
        &[
            user("u1", None, STAMP, "fixture-text-1"),
            assistant("a1", Some("u1"), STAMP, "fixture-text-2"),
        ],
    )?;
    Ok(session)
}

#[test]
fn side_leaf_names_the_translation() -> Gate {
    let dir = tempfile::tempdir()?;
    let mut session = two_entries(dir.path())?;
    let before = std::fs::read(session.file())?;
    let hash_before = session.head_hash()?;
    let translation = run(&mut session, &dir.path().join("o"))?;
    let leaves = leaves(&session)?;
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].id(), translation.leaf);
    assert_eq!(leaves[0].parent_id(), Some("a1"));
    assert_eq!(session.head()?, Some("a1"));
    assert_eq!(session.head_hash()?, hash_before);
    let after = std::fs::read(session.file())?;
    assert!(after.len() > before.len());
    assert_eq!(&after[..before.len()], before.as_slice());
    let data = data(&leaves[0])?;
    let rollout = std::fs::read(&translation.rollout)?;
    let account_bytes = std::fs::read(&translation.account)?;
    assert_eq!(data["rollout_sha256"], Hash::of(&rollout).as_str());
    assert_eq!(data["account_sha256"], Hash::of(&account_bytes).as_str());
    assert_eq!(data["head_hash"], hash_before.as_str());
    assert_eq!(data["head"], "a1");
    let name = translation.rollout.to_string_lossy();
    let thread = data["thread"].as_str().ok_or("no thread")?;
    assert!(name.ends_with(&format!("-{thread}.jsonl")));
    assert_eq!(thread, translation.thread);
    Ok(())
}

#[test]
fn leaf_data_holds_eight_keys() -> Gate {
    let dir = tempfile::tempdir()?;
    let mut session = two_entries(dir.path())?;
    let out = dir.path().join("o");
    let translation = run(&mut session, &out)?;
    let leaves = leaves(&session)?;
    let data = data(&leaves[0])?;
    let keys: Vec<&String> = data.as_object().ok_or("not an object")?.keys().collect();
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
    let rollout = data["rollout"].as_str().ok_or("no rollout")?;
    assert!(!rollout.starts_with('/'));
    assert_eq!(out.join(rollout), translation.rollout);
    Ok(())
}

#[test]
fn second_translation_lists_the_first_leaf_lost() -> Gate {
    let dir = tempfile::tempdir()?;
    let mut session = two_entries(dir.path())?;
    let first = run(&mut session, &dir.path().join("o1"))?;
    let second = run(&mut session, &dir.path().join("o2"))?;
    let account = account(&second)?;
    let mut rows = Vec::new();
    for row in &account.lost {
        if row.kind == "lys.translation" {
            rows.push(row);
        }
    }
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].entry, first.leaf);
    assert_eq!(
        rows[0].reason,
        "Codex has no item for it and it is not conversation"
    );
    Ok(())
}
