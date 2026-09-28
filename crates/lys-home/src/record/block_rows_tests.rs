#![cfg(test)]
//! Gates on the block rows (HOME-030 R2) over the compacted fixture: one row
//! per stored part, the hash the store returned for the part as the source
//! record held it, every row's block held, no content in the rows, and no
//! field added to a message entry.

use serde_json::{Value, json};

use crate::harness::claude_code::compaction_tests::{
    Gate, Got, content_strings, fixture, fixture_path, import_ok, keys_at_any_depth, u,
};
use crate::record::block_rows::{BlockRow, read_block_rows};
use crate::record::blocks::Hash;
use crate::record::entries::{Entry, EntryBody};
use crate::record::index::Index;
use crate::record::{Home, Session};

/// The rows beside a session, which must be there.
fn rows_of(session: &Session) -> Got<Vec<BlockRow>> {
    let rows = read_block_rows(session.file())?;
    Ok(rows.ok_or("no block rows file")?)
}

#[test]
fn the_fixture_import_writes_one_row_per_stored_part() -> Gate {
    let (imported, report) = import_ok(&fixture()?)?;
    let rows = rows_of(&imported.session)?;
    assert_eq!(rows.len(), 15);
    assert_eq!(report.blocks, 15);
    assert!(Index::blocks_path(imported.session.file()).is_file());
    Ok(())
}

#[test]
fn the_tool_use_row_carries_the_hash_of_the_part_as_the_source_held_it() -> Gate {
    let text = std::fs::read_to_string(fixture_path())?;
    let line_two: Value = serde_json::from_str(text.lines().nth(1).ok_or("no line 2")?)?;
    let tool_use = &line_two["message"]["content"][1];
    assert_eq!(tool_use["type"], json!("tool_use"));
    let expected = Hash::of(&serde_json::to_vec(tool_use)?);
    let (imported, _) = import_ok(&fixture()?)?;
    let rows = rows_of(&imported.session)?;
    let found: Vec<_> = rows
        .iter()
        .filter(|row| row.entry == u(2) && row.part == 1)
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].hash, expected.as_str());
    Ok(())
}

#[test]
fn every_row_names_a_block_the_store_holds() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let blocks = Home::open(&imported.root)?.blocks()?;
    let rows = rows_of(&imported.session)?;
    let mut held = 0;
    for row in &rows {
        assert!(blocks.contains(&Hash::parse(&row.hash)?), "{}", row.hash);
        held += 1;
    }
    assert_eq!(held, 15);
    Ok(())
}

#[test]
fn the_rows_file_holds_no_content_string() -> Gate {
    let records = fixture()?;
    let (imported, _) = import_ok(&records)?;
    let rows = std::fs::read_to_string(Index::blocks_path(imported.session.file()))?;
    let strings = content_strings(&records);
    assert!(!strings.is_empty());
    let mut scanned = 0;
    for s in &strings {
        assert!(!rows.contains(s.as_str()), "a content string in the rows");
        scanned += 1;
    }
    assert!(scanned > 0);
    assert_eq!(scanned, strings.len());
    Ok(())
}

#[test]
fn no_message_entry_carries_a_hash_or_blocks_key() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let text = std::fs::read_to_string(imported.session.file())?;
    let mut messages = 0;
    for line in text.lines().skip(1) {
        let entry: Entry = serde_json::from_str(line)?;
        if !matches!(entry.body, EntryBody::Message { .. }) {
            continue;
        }
        messages += 1;
        let mut keys = Vec::new();
        keys_at_any_depth(&serde_json::from_str::<Value>(line)?, &mut keys);
        assert!(
            !keys.iter().any(|k| k == "hash" || k == "blocks"),
            "entry {}",
            entry.id()
        );
    }
    assert_eq!(messages, 12);
    Ok(())
}
