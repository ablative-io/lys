#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the side-leaf append and the session head hash: the head does not
//! move, the head file is untouched, the path is unchanged, the leaf survives
//! a reopen, and the hash is of the head line's bytes, newline included.

use serde_json::json;

use crate::record::blocks::Hash;
use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::index::Index;
use crate::record::{Home, Session};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/session.jsonl"
);
const CUSTOM: &str = "lys.test_beside";

fn four_entries(home: &Home) -> Session {
    let mut s = home.create_session("four", "/w", None).unwrap();
    let mut prev: Option<String> = None;
    for id in ["e1", "e2", "e3", "e4"] {
        s.append_entry(&Entry {
            base: EntryBase {
                id: id.to_owned(),
                parent_id: prev.clone(),
                timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
            },
            body: EntryBody::Message {
                message: json!({"role": "user", "content": [], "timestamp": 0}),
            },
        })
        .unwrap();
        prev = Some(id.to_owned());
    }
    s
}

fn custom() -> EntryBody {
    EntryBody::Custom {
        custom_type: CUSTOM.to_owned(),
        data: Some(json!({"n": 1})),
    }
}

#[test]
fn a_side_leaf_hangs_under_the_head_and_the_head_stays_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let mut s = four_entries(&home);
    let head_file = Index::head_path(s.file());
    let head_bytes = std::fs::read(&head_file).unwrap();
    let before = s.head_hash().unwrap();
    let id = s.append_beside(custom()).unwrap();
    let entry = s.entry(&id).unwrap();
    assert_eq!(entry.parent_id(), Some("e4"));
    assert!(entry.is_custom(CUSTOM));
    assert_eq!(s.head().unwrap(), Some("e4"));
    assert_eq!(std::fs::read(&head_file).unwrap(), head_bytes);
    let ctx = s.context_path().unwrap();
    assert_eq!(
        ctx.iter().map(Entry::id).collect::<Vec<_>>(),
        ["e1", "e2", "e3", "e4"]
    );
    assert_eq!(s.head_hash().unwrap(), before);
    assert_eq!(s.len().unwrap(), 5);
    let file = s.file().to_path_buf();
    drop(s);
    let s = Session::open(&file).unwrap();
    assert_eq!(s.head().unwrap(), Some("e4"));
    let leaves = s.customs_everywhere(CUSTOM).unwrap();
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].id(), id);
    assert_eq!(s.customs(CUSTOM).unwrap().len(), 0, "not on the path");
    drop(s);
    dir.close().unwrap();
}

#[test]
fn the_head_hash_is_the_last_line_of_the_fixture_with_its_newline() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let bytes = std::fs::read(FIXTURE).unwrap();
    std::fs::write(home.session_path("fixture").unwrap(), &bytes).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let last = text.lines().last().unwrap();
    let expected = Hash::of(format!("{last}\n").as_bytes());
    let mut s = home.open_session("fixture").unwrap();
    assert!(s.index_was_rebuilt());
    assert_eq!(s.head().unwrap(), Some("e4"));
    assert_eq!(s.len().unwrap(), 4);
    assert_eq!(s.head_hash().unwrap(), expected);
    let id = s.append_beside(custom()).unwrap();
    assert_eq!(s.head_hash().unwrap(), expected);
    assert_eq!(s.entry(&id).unwrap().parent_id(), Some("e4"));
    let file = s.file().to_path_buf();
    drop(s);
    let s = Session::open(&file).unwrap();
    assert_eq!(
        s.head().unwrap(),
        Some("e4"),
        "the head was persisted on the first open, so the leaf is not taken for it"
    );
    assert_eq!(s.head_hash().unwrap(), expected);
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_session_with_only_its_header_hashes_the_header_line() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let s = home.create_session("bare", "/w", None).unwrap();
    let text = std::fs::read_to_string(s.file()).unwrap();
    assert_eq!(text.lines().count(), 1);
    let expected = Hash::of(text.as_bytes());
    assert!(text.ends_with('\n'));
    assert_eq!(s.head().unwrap(), None);
    assert_eq!(s.head_hash().unwrap(), expected);
    drop(s);
    dir.close().unwrap();
}
