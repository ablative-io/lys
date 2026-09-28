#![cfg(test)]
//! Gates on the staged import (HOME-020 R4): five syncs of the session's own
//! whatever the record count, the block store's syncs counted beside them, a
//! part-way import that leaves no session, and its remainder removed by the
//! next import or open of the id.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cli::{Cli, Command, run};
use crate::error::HomeError;
use crate::harness::claude_code::import::import_claude_code;
use crate::record::index::Index;
use crate::record::staged::staging_path;
use crate::record::{Home, Session};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_result.jsonl")
}

/// A transcript of `turns` user and `turns` assistant text records in one
/// parent chain.
fn chain(path: &Path, turns: usize) {
    let mut body = String::new();
    let mut parent: Option<String> = None;
    for n in 0..turns * 2 {
        let uuid = format!("{n:08}-0000-4000-8000-000000000000");
        let kind = if n % 2 == 0 { "user" } else { "assistant" };
        let message = if n % 2 == 0 {
            json!({"role": "user", "content": format!("question {n}")})
        } else {
            json!({"role": "assistant", "model": "m", "stop_reason": "end_turn",
                "content": [{"type": "text", "text": format!("answer {n}")}]})
        };
        let record = json!({"type": kind, "uuid": uuid, "parentUuid": parent,
            "timestamp": "2026-01-01T00:00:00.000Z", "message": message});
        body.push_str(&record.to_string());
        body.push('\n');
        parent = Some(uuid);
    }
    std::fs::write(path, body).unwrap();
}

/// Stage, import and publish; the session's syncs and the store's.
fn staged_import(source: &Path) -> (u64, u64, u64) {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let mut s = home.stage_session("multi", "").unwrap();
    let report = import_claude_code(source, &mut s, &blocks).unwrap();
    s.publish().unwrap();
    assert!(s.file().ends_with("sessions/multi.jsonl"));
    assert!(!staging_path(s.file()).exists());
    (s.io_counts().syncs, blocks.syncs(), report.blocks_new)
}

#[test]
fn the_fixture_publishes_with_five_syncs_and_two_store_syncs_per_new_block() {
    let (syncs, store_syncs, blocks_new) = staged_import(&fixture());
    println!("session syncs {syncs}, block store syncs {store_syncs}, blocks new {blocks_new}");
    assert_eq!(syncs, 5);
    assert!(blocks_new > 0);
    assert_eq!(store_syncs, 2 * blocks_new);
}

#[test]
fn three_hundred_records_publish_with_five_syncs() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("long.jsonl");
    chain(&source, 150);
    let text = std::fs::read_to_string(&source).unwrap();
    assert_eq!(text.lines().count(), 300);
    let (syncs, _, _) = staged_import(&source);
    assert_eq!(syncs, 5);
}

#[test]
fn an_import_dropped_before_publish_leaves_no_session_and_the_next_import_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("home");
    let home = Home::open(&root).unwrap();
    let blocks = home.blocks().unwrap();
    let file = home.session_path("multi").unwrap();
    let mut s = home.stage_session("multi", "").unwrap();
    import_claude_code(&fixture(), &mut s, &blocks).unwrap();
    drop(s);
    assert!(!file.exists());
    assert!(staging_path(&file).is_file());
    assert!(!home.session_ids().unwrap().contains(&"multi".to_owned()));
    let report = run(Cli {
        command: Command::Import {
            home: root,
            claude_code: fixture(),
            session: "multi".into(),
        },
    })
    .unwrap();
    assert!(!staging_path(&file).exists());
    let entries = report["report"]["entries"].as_u64().unwrap();
    let opened = Session::open(&file).unwrap();
    assert_eq!(opened.len().unwrap() as u64, entries);
    assert!(home.session_ids().unwrap().contains(&"multi".to_owned()));
}

#[test]
fn an_open_of_an_absent_session_removes_the_staging_file_index_and_head() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path()).unwrap();
    let file = home.session_path("gone").unwrap();
    let left = [
        staging_path(&file),
        Index::index_path(&file),
        Index::head_path(&file),
    ];
    for path in &left {
        std::fs::write(path, b"left part way\n").unwrap();
    }
    let err = Session::open(&file).unwrap_err();
    assert!(matches!(err, HomeError::Io { .. }), "{err}");
    for path in &left {
        assert!(!path.exists(), "{} was left", path.display());
    }
}

#[test]
fn the_import_command_reports_the_published_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("home");
    let report = run(Cli {
        command: Command::Import {
            home: root.clone(),
            claude_code: fixture(),
            session: "multi".into(),
        },
    })
    .unwrap();
    assert_eq!(
        report["file"].as_str().map(PathBuf::from),
        Some(root.join("sessions").join("multi.jsonl"))
    );
    let file = root.join("sessions").join("multi.jsonl");
    assert!(!staging_path(&file).exists());
}
