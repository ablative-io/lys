#![cfg(test)]
//! Gates on the render's write stage: a loss account whose serialiser fails
//! refuses the render with nothing created, and a render that serialises
//! writes the record lines compactly and the loss account pretty-printed.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::render_write::write_render;
use crate::harness::claude_code::seed::{Seed, seed_path};

type Gate = Result<(), Box<dyn Error>>;

/// A loss account whose serialiser always fails.
struct Unserialisable;

impl Serialize for Unserialisable {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("fixture serialiser refuses"))
    }
}

fn seed() -> Seed {
    Seed {
        parent: "parent".to_owned(),
        point: "e1".to_owned(),
        lantern: "l1".to_owned(),
        text: "fixture seed".to_owned(),
    }
}

fn record() -> Value {
    json!({"type": "user", "uuid": "u1", "message": {"role": "user", "content": "fixture"}})
}

/// The write stage over one record from entry `u1`, `account` and a seed,
/// rendering to `nested/r.jsonl` under `dir`.
fn write<A: Serialize>(dir: &Path, account: &A) -> Result<PathBuf, HomeError> {
    let rendered = dir.join("nested").join("r.jsonl");
    let seed = seed();
    let seed_file = seed_path(&rendered);
    let records = [(record(), Some("u1".to_owned()))];
    write_render("s", &rendered, &records, account, Some((&seed, &seed_file)))
}

#[test]
fn a_loss_account_that_will_not_serialise_refuses_and_nothing_is_created() -> Gate {
    let dir = tempfile::tempdir()?;
    let result = write(dir.path(), &Unserialisable);
    let named = match &result {
        Err(HomeError::RenderUnserialisable {
            session,
            what,
            entry,
            ..
        }) => Some((session.as_str(), *what, entry.clone())),
        _ => None,
    };
    assert_eq!(named, Some(("s", "loss account", None)), "{result:?}");
    let nested = dir.path().join("nested");
    let mut absent = 0;
    for path in [
        nested.clone(),
        nested.join("r.jsonl"),
        nested.join("r.loss.json"),
        seed_path(&nested.join("r.jsonl")),
    ] {
        assert!(!path.exists(), "{}", path.display());
        absent += 1;
    }
    assert_eq!(absent, 4);
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 0);
    Ok(())
}

#[test]
fn a_render_that_serialises_writes_compact_lines_and_a_pretty_account() -> Gate {
    let dir = tempfile::tempdir()?;
    let account = json!({"session_id": "s", "model": "m", "authored": false, "dropped": []});
    let loss_path = write(dir.path(), &account)?;
    let nested = dir.path().join("nested");
    let rendered = nested.join("r.jsonl");
    assert_eq!(loss_path, nested.join("r.loss.json"));
    let mut line = serde_json::to_string(&record())?;
    line.push('\n');
    assert_eq!(std::fs::read(&rendered)?, line.into_bytes());
    assert_eq!(
        std::fs::read(&loss_path)?,
        serde_json::to_vec_pretty(&account)?
    );
    assert_eq!(std::fs::read(seed_path(&rendered))?, seed().bytes());
    assert_eq!(std::fs::read_dir(&nested)?.count(), 3);
    Ok(())
}
