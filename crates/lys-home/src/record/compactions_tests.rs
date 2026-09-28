#![cfg(test)]
//! Gates on the compaction listing (HOME-030 R5), through the command's own
//! run so the exit status is the one the binary exits with: every span
//! entry read and every block held on the fixture; a removed block named by
//! hash; an absent rows file reported unverified; an unreadable entry named
//! and the walk continued; a missing and a completed summary; a compaction
//! with no loss entry; no content; the missing argument; and nothing written.

use std::path::Path;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::cli::{Cli, Command, run_with_status};
use crate::harness::claude_code::compaction_tests::{
    Gate, Got, Imported, SESSION, content_strings, fixture, import_ok, keys_at_any_depth, loss_of,
    u, with_late_summary, without_second_summary,
};
use crate::record::Home;
use crate::record::block_rows::read_block_rows;
use crate::record::compactions::NO_LOSS_ENTRY;
use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::index::Index;

/// The listing of the fixture session in `imported`'s home: the report
/// round-tripped through its printed form, and the exit status.
fn listing(imported: &Imported) -> Got<(Value, i32)> {
    listing_of(&imported.root, SESSION)
}

fn listing_of(root: &Path, session: &str) -> Got<(Value, i32)> {
    let outcome = run_with_status(Cli {
        command: Command::Compactions {
            home: root.to_path_buf(),
            session: session.to_owned(),
        },
    })?;
    let printed = serde_json::to_string(&outcome.report)?;
    Ok((serde_json::from_str(&printed)?, outcome.status))
}

/// The listed compaction at `at`.
fn listed(report: &Value, at: usize) -> Got<&Value> {
    report["compactions"]
        .get(at)
        .ok_or_else(|| format!("no compaction {at} listed").into())
}

#[test]
fn the_fixture_lists_both_compactions_whole() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let (report, status) = listing(&imported)?;
    assert_eq!(report["unreadable"], json!([]));
    assert_eq!(report["blocks_verified"], json!(true));
    assert_eq!(report["compactions"].as_array().map(Vec::len), Some(2));
    let (loss, _) = loss_of(&imported.session, &u(8))?;
    let first = listed(&report, 0)?;
    assert_eq!(first["compaction"], json!(u(8)));
    assert_eq!(first["summary"], json!("present"));
    assert_eq!(first["loss"], json!(loss.id()));
    for (key, value) in [
        ("entries_expected", json!(4)),
        ("entries_read", json!(4)),
        ("entries_missing", json!([])),
        ("blocks_expected", json!(5)),
        ("blocks_held", json!(5)),
        ("blocks_missing", json!([])),
        ("reason", Value::Null),
    ] {
        assert_eq!(first[key], value, "{key}");
    }
    let second = listed(&report, 1)?;
    assert_eq!(second["compaction"], json!(u(0x0e)));
    for (key, value) in [
        ("entries_expected", json!(5)),
        ("entries_read", json!(5)),
        ("entries_missing", json!([])),
        ("blocks_expected", json!(5)),
        ("blocks_held", json!(5)),
        ("blocks_missing", json!([])),
    ] {
        assert_eq!(second[key], value, "{key}");
    }
    assert_eq!(status, 0);
    Ok(())
}

#[test]
fn a_removed_block_is_named_by_hash_and_fails_the_listing() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let rows = read_block_rows(imported.session.file())?.ok_or("no rows")?;
    let hash = rows
        .iter()
        .find(|row| row.entry == u(2) && row.part == 1)
        .ok_or("no row for the tool use")?
        .hash
        .clone();
    let block = imported.root.join("blocks").join(&hash[..2]).join(&hash);
    std::fs::remove_file(block)?;
    let (report, status) = listing(&imported)?;
    let first = listed(&report, 0)?;
    assert_eq!(first["blocks_held"], json!(4));
    assert_eq!(first["blocks_missing"], json!([hash]));
    assert_eq!(listed(&report, 1)?["blocks_missing"], json!([]));
    assert_eq!(status, 1);
    Ok(())
}

#[test]
fn an_absent_rows_file_leaves_the_blocks_unverified() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    std::fs::remove_file(Index::blocks_path(imported.session.file()))?;
    let (report, status) = listing(&imported)?;
    assert_eq!(report["blocks_verified"], json!(false));
    assert_eq!(report["session"], json!(SESSION));
    let all = report["compactions"].as_array().ok_or("no compactions")?;
    assert_eq!(all.len(), 2);
    for compaction in all {
        for key in ["blocks_held", "blocks_missing", "blocks_sha256"] {
            assert_eq!(compaction[key], Value::Null, "{key}");
        }
    }
    assert_eq!(listed(&report, 0)?["entries_read"], json!(4));
    assert_eq!(status, 1);
    Ok(())
}

#[test]
fn an_unreadable_entry_is_named_and_the_walk_goes_on() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let row = Index::read(imported.session.file())?
        .1
        .row(&u(3))
        .ok_or("no row")?
        .clone();
    let mut bytes = std::fs::read(imported.session.file())?;
    let start = usize::try_from(row.offset)?;
    let end = usize::try_from(row.offset + row.len - 1)?;
    bytes[start..end].fill(b'x');
    std::fs::write(imported.session.file(), bytes)?;
    let (report, status) = listing(&imported)?;
    assert_eq!(report["unreadable"], json!([u(3)]));
    let first = listed(&report, 0)?;
    assert_eq!(first["entries_read"], json!(3));
    assert_eq!(first["entries_missing"], json!([u(3)]));
    let second = listed(&report, 1)?;
    assert_eq!(second["entries_read"], json!(5));
    assert_eq!(second["entries_missing"], json!([]));
    assert_eq!(status, 1);
    Ok(())
}

#[test]
fn a_summary_that_never_arrived_is_listed_missing() -> Gate {
    let (imported, _) = import_ok(&without_second_summary()?)?;
    let (report, _) = listing(&imported)?;
    assert_eq!(listed(&report, 0)?["summary"], json!("present"));
    let second = listed(&report, 1)?;
    assert_eq!(second["compaction"], json!(u(0x0d)));
    assert_eq!(second["summary"], json!("missing"));
    Ok(())
}

#[test]
fn a_completed_compaction_is_listed_present_with_what_completed_it() -> Gate {
    let (imported, _) = import_ok(&with_late_summary()?)?;
    let (report, status) = listing(&imported)?;
    assert_eq!(report["compactions"].as_array().map(Vec::len), Some(3));
    let first = listed(&report, 0)?;
    assert_eq!(first["compaction"], json!(u(7)));
    assert_eq!(first["summary"], json!("present"));
    assert_eq!(first["completes"], Value::Null);
    assert_eq!(first["completed_by"], json!(u(8)));
    let second = listed(&report, 1)?;
    assert_eq!(second["compaction"], json!(u(8)));
    assert_eq!(second["summary"], json!("present"));
    assert_eq!(second["completes"], json!(u(7)));
    assert_eq!(second["completed_by"], Value::Null);
    assert_eq!(second["entries_expected"], json!(4));
    assert_eq!(second["entries_read"], json!(4));
    assert_eq!(listed(&report, 2)?["compaction"], json!(u(0x0e)));
    assert_eq!(status, 0);
    Ok(())
}

#[test]
fn a_compaction_without_a_loss_entry_has_every_span_field_null_and_a_reason() -> Gate {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    let home = Home::open(&root)?;
    {
        let mut s = home.create_session("older", "/fixture", None)?;
        let entry = |id: String, parent: Option<String>, body: EntryBody| Entry {
            base: EntryBase {
                id,
                parent_id: parent,
                timestamp: "2026-09-28T00:00:00.000Z".to_owned(),
            },
            body,
        };
        s.append_entry(&entry(
            u(1),
            None,
            EntryBody::Message {
                message: json!({"role": "user", "content": [], "timestamp": 0}),
            },
        ))?;
        s.append_entry(&entry(
            u(2),
            Some(u(1)),
            EntryBody::Message {
                message: json!({"role": "assistant", "content": [], "timestamp": 0}),
            },
        ))?;
        s.append_entry(&entry(
            "c1".to_owned(),
            Some(u(2)),
            EntryBody::Compaction {
                summary: String::new(),
                first_kept_entry_id: u(2),
                tokens_before: 0,
                rest: Map::new(),
            },
        ))?;
    }
    let (report, status) = listing_of(&root, "older")?;
    let all = report["compactions"].as_array().ok_or("no compactions")?;
    assert_eq!(all.len(), 1);
    for key in [
        "loss",
        "span_first",
        "span_last",
        "entries_expected",
        "entries_read",
        "entries_missing",
        "blocks_expected",
        "blocks_held",
        "blocks_missing",
        "blocks_sha256",
    ] {
        assert_eq!(all[0][key], Value::Null, "{key}");
    }
    assert_eq!(all[0]["reason"], json!(NO_LOSS_ENTRY));
    assert_eq!(status, 1);
    Ok(())
}

#[test]
fn the_listing_carries_no_content() -> Gate {
    let records = fixture()?;
    let (imported, _) = import_ok(&records)?;
    let (report, _) = listing(&imported)?;
    let printed = serde_json::to_string(&report)?;
    let mut keys = Vec::new();
    keys_at_any_depth(&report, &mut keys);
    assert!(
        !keys
            .iter()
            .any(|k| k == "text" || k == "content" || k == "body")
    );
    let strings = content_strings(&records);
    let mut scanned = 0;
    for s in &strings {
        assert!(!printed.contains(s.as_str()), "a content string listed");
        scanned += 1;
    }
    assert!(scanned > 0);
    Ok(())
}

#[test]
fn a_listing_without_a_session_is_refused_naming_it() -> Gate {
    let argv = ["lys-home", "compactions", "--home", "h"];
    let refused = <Cli as clap::Parser>::try_parse_from(argv);
    let Err(error) = refused else {
        return Err("a listing without --session was accepted".into());
    };
    assert_eq!(error.exit_code(), 2);
    assert!(error.to_string().contains("--session"), "{error}");
    Ok(())
}

#[test]
fn a_listing_writes_nothing() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let file = imported.session.file().to_path_buf();
    let beside = [
        file.clone(),
        Index::index_path(&file),
        Index::head_path(&file),
        Index::blocks_path(&file),
    ];
    let hashes = |paths: &[std::path::PathBuf]| -> Got<Vec<Vec<u8>>> {
        let mut out = Vec::new();
        for path in paths {
            out.push(Sha256::digest(std::fs::read(path)?).to_vec());
        }
        Ok(out)
    };
    let before = hashes(&beside)?;
    listing(&imported)?;
    assert_eq!(hashes(&beside)?, before);
    assert!(imported.dir.path().join("home").is_dir());
    Ok(())
}
