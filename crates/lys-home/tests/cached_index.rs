#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The cached index is a cache: a session opens from it only when its rows
//! are this file's, row by row, and rebuilds from the session file otherwise.
//! A cache whose final offset and length still match the file but whose rows
//! name a parent not yet indexed (itself included) used to be taken on trust,
//! and a self-parent row then made the ancestry walk forever.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use lys_home::record::index::Index;
use lys_home::{EntryBody, Home};

/// A home with one session of two entries, the second under the first; the
/// session file and the index beside it.
fn two_entries() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path()).unwrap();
    let mut session = home.create_session("s1", "/w", None).unwrap();
    for n in 1..=2 {
        session
            .append(EntryBody::Message {
                message: json!({"role": "user", "content": format!("entry {n}")}),
            })
            .unwrap();
    }
    let file = session.file().to_path_buf();
    drop(session);
    let index = Index::index_path(&file);
    (dir, file, index)
}

/// The index rows as written, one JSON object per line.
fn rows(index: &PathBuf) -> Vec<Value> {
    fs::read_to_string(index)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn write_rows(index: &PathBuf, rows: &[Value]) {
    let mut text = String::new();
    for row in rows {
        text.push_str(&row.to_string());
        text.push('\n');
    }
    fs::write(index, text).unwrap();
}

#[test]
fn an_untouched_index_is_taken_as_it_stands() {
    let (_dir, file, _index) = two_entries();
    let session = lys_home::Session::open(&file).unwrap();
    assert!(!session.index_was_rebuilt());
    assert_eq!(session.path().unwrap().0.len(), 2);
}

#[test]
fn a_self_parent_row_in_the_cache_is_rebuilt_from_the_file() {
    let (_dir, file, index) = two_entries();
    let mut cached = rows(&index);
    assert_eq!(cached.len(), 2);
    let own = cached[1]["id"].clone();
    cached[1]["parent"] = own;
    write_rows(&index, &cached);
    let session = lys_home::Session::open(&file).unwrap();
    assert!(session.index_was_rebuilt());
    let (path, _) = session.path().unwrap();
    assert_eq!(path.len(), 2);
    assert_eq!(rows(&index)[1]["parent"], rows(&index)[0]["id"]);
}

#[test]
fn a_row_naming_a_parent_not_yet_indexed_is_rebuilt_from_the_file() {
    let (_dir, file, index) = two_entries();
    let mut cached = rows(&index);
    cached[1]["parent"] = json!("nobody");
    write_rows(&index, &cached);
    let session = lys_home::Session::open(&file).unwrap();
    assert!(session.index_was_rebuilt());
    assert_eq!(session.path().unwrap().0.len(), 2);
}

#[test]
fn a_row_that_does_not_start_where_the_last_ended_is_rebuilt_from_the_file() {
    let (_dir, file, index) = two_entries();
    let mut cached = rows(&index);
    let first_len = cached[0]["len"].as_u64().unwrap();
    let second_len = cached[1]["len"].as_u64().unwrap();
    // The two rows still end where the file does, but the first claims one
    // byte more and the second one byte less.
    cached[0]["len"] = json!(first_len + 1);
    cached[1]["offset"] = json!(cached[1]["offset"].as_u64().unwrap() + 1);
    cached[1]["len"] = json!(second_len - 1);
    write_rows(&index, &cached);
    let session = lys_home::Session::open(&file).unwrap();
    assert!(session.index_was_rebuilt());
    assert_eq!(session.path().unwrap().0.len(), 2);
}
