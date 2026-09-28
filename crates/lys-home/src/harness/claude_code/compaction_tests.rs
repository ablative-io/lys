#![cfg(test)]
//! Gates on the compaction mapping (HOME-030 R3) over the hand-built
//! compacted fixture: the pair becomes one compaction under the summary's
//! uuid with its parent, first kept entry, tokens and details; a missing or
//! dangling preserved segment is never refused; a boundary whose summary
//! comes late or never becomes a compaction without one, completed by a
//! second; an unknown first kept entry is refused by uuid. The compaction's
//! render (R6) is gated here too, beside the mapping it reverses. Also the
//! fixture helpers the record's compaction gates share.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::error::HomeError;
use crate::harness::claude_code::import::{ImportReport, import_claude_code};
use crate::harness::claude_code::render::{RenderTarget, render_claude_code};
use crate::record::entries::{CUSTOM_LOSS, Entry, EntryBody};
use crate::record::index::Index;
use crate::record::{Home, Session};

/// What a gate returns.
pub(crate) type Gate = Result<(), Box<dyn Error>>;
/// What a helper returns.
pub(crate) type Got<T> = Result<T, Box<dyn Error>>;

/// The session id every fixture import creates.
pub(crate) const SESSION: &str = "compacted";

/// The fixture's uuid numbered `n`: `00000000-0000-4000-8000-0000000000NN`.
pub(crate) fn u(n: u8) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// The fixture file.
pub(crate) fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude_code_compacted.jsonl")
}

/// The fixture's records, parsed from the file, in file order.
pub(crate) fn fixture() -> Got<Vec<Value>> {
    let text = std::fs::read_to_string(fixture_path())?;
    let records = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    Ok(records)
}

/// A home holding one session imported from some records.
pub(crate) struct Imported {
    /// The directory holding the source file and the home.
    pub(crate) dir: TempDir,
    /// The home's root.
    pub(crate) root: PathBuf,
    /// The session, still owned.
    pub(crate) session: Session,
    /// What the import answered.
    pub(crate) report: Result<ImportReport, HomeError>,
}

/// Import `records`, written one per line, into a fresh home.
pub(crate) fn import(records: &[Value]) -> Got<Imported> {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source.jsonl");
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    std::fs::write(&source, text)?;
    import_into(dir, &source)
}

/// Import the file `source` as it stands into a fresh home, and require the
/// import to succeed.
pub(crate) fn import_file(source: &Path) -> Got<(Imported, ImportReport)> {
    let mut imported = import_into(tempfile::tempdir()?, source)?;
    let report = std::mem::replace(&mut imported.report, Ok(ImportReport::default()))?;
    Ok((imported, report))
}

/// Import `source` into a home made under `dir`.
fn import_into(dir: TempDir, source: &Path) -> Got<Imported> {
    let root = dir.path().join("home");
    let home = Home::open(&root)?;
    let blocks = home.blocks()?;
    let mut session = home.create_session(SESSION, "/fixture", None)?;
    let report = import_claude_code(source, &mut session, &blocks);
    Ok(Imported {
        dir,
        root,
        session,
        report,
    })
}

/// Import `records` and require the import to succeed; returns the home
/// and the report.
pub(crate) fn import_ok(records: &[Value]) -> Got<(Imported, ImportReport)> {
    let mut imported = import(records)?;
    let report = std::mem::replace(&mut imported.report, Ok(ImportReport::default()))?;
    Ok((imported, report))
}

/// The fixture with both boundaries' `preservedSegment` removed.
pub(crate) fn without_segments() -> Got<Vec<Value>> {
    let mut records = fixture()?;
    for at in [6, 12] {
        records[at]["compactMetadata"]
            .as_object_mut()
            .ok_or("a boundary without compactMetadata")?
            .remove("preservedSegment");
    }
    Ok(records)
}

/// The fixture with lines 14 to 16 removed: the second boundary's summary
/// never arrives.
pub(crate) fn without_second_summary() -> Got<Vec<Value>> {
    let mut records = fixture()?;
    records.truncate(13);
    Ok(records)
}

/// The fixture with a user record `…11`, whose parent is the first
/// boundary, inserted between lines 7 and 8.
pub(crate) fn with_late_summary() -> Got<Vec<Value>> {
    let mut records = fixture()?;
    let mut inserted = records[8].clone();
    inserted["uuid"] = json!(u(0x11));
    inserted["parentUuid"] = json!(u(7));
    inserted["message"]["content"] = json!("Fixture ask inserted.");
    records.insert(7, inserted);
    Ok(records)
}

/// The three-line file: a user record, an assistant record, a `summary`.
pub(crate) fn summary_file() -> Got<Vec<Value>> {
    let records = fixture()?;
    Ok(vec![
        records[0].clone(),
        records[1].clone(),
        json!({"type": "summary", "summary": "Fixture summary.", "leafUuid": u(2)}),
    ])
}

/// Every content string of `records`: text, thinking, strings inside a tool
/// input, a tool result's text, a string message content, and a `summary`.
pub(crate) fn content_strings(records: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for record in records {
        if let Some(s) = record.get("summary").and_then(Value::as_str) {
            out.push(s.to_owned());
        }
        match record.get("message").and_then(|m| m.get("content")) {
            Some(Value::String(s)) => out.push(s.clone()),
            Some(Value::Array(parts)) => {
                for part in parts {
                    part_strings(part, &mut out);
                }
            }
            _ => {}
        }
    }
    out.retain(|s| !s.is_empty());
    out
}

fn part_strings(part: &Value, out: &mut Vec<String>) {
    match part.get("type").and_then(Value::as_str) {
        Some("text") => strings_in(&part["text"], out),
        Some("thinking") => strings_in(&part["thinking"], out),
        Some("tool_use") => strings_in(&part["input"], out),
        Some("tool_result") => match &part["content"] {
            Value::String(s) => out.push(s.clone()),
            Value::Array(inner) => {
                for text in inner.iter().filter(|p| p["type"] == "text") {
                    strings_in(&text["text"], out);
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn strings_in(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| strings_in(v, out)),
        Value::Object(map) => map.values().for_each(|v| strings_in(v, out)),
        _ => {}
    }
}

/// Every key at any depth of a JSON value.
pub(crate) fn keys_at_any_depth(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Array(items) => items.iter().for_each(|v| keys_at_any_depth(v, out)),
        Value::Object(map) => {
            for (key, v) in map {
                out.push(key.clone());
                keys_at_any_depth(v, out);
            }
        }
        _ => {}
    }
}

/// A compaction entry as the gates read it.
#[derive(Debug, PartialEq)]
pub(crate) struct Seen {
    pub(crate) parent: Option<String>,
    pub(crate) summary: String,
    pub(crate) first_kept: String,
    pub(crate) tokens_before: u64,
    pub(crate) details: Value,
}

/// The compaction entry `id`, read by id.
pub(crate) fn seen(session: &Session, id: &str) -> Got<Seen> {
    let entry = session.entry(id)?;
    let EntryBody::Compaction {
        summary,
        first_kept_entry_id,
        tokens_before,
        rest,
    } = entry.body
    else {
        return Err(format!("entry {id} is not a compaction").into());
    };
    Ok(Seen {
        parent: entry.base.parent_id,
        summary,
        first_kept: first_kept_entry_id,
        tokens_before,
        details: rest.get("details").cloned().unwrap_or(Value::Null),
    })
}

/// A custom entry's data.
pub(crate) fn data_of(entry: &Entry) -> Got<Value> {
    let EntryBody::Custom {
        data: Some(data), ..
    } = &entry.body
    else {
        let id = entry.id();
        return Err(format!("entry {id} carries no custom data").into());
    };
    Ok(data.clone())
}

/// Every `lys.loss` entry of the session, in file order, with its data.
pub(crate) fn losses(session: &Session) -> Got<Vec<(Entry, Value)>> {
    let mut out = Vec::new();
    for entry in session.customs_everywhere(CUSTOM_LOSS)? {
        let data = data_of(&entry)?;
        out.push((entry, data));
    }
    Ok(out)
}

/// The `lys.loss` entry naming the compaction `id`.
pub(crate) fn loss_of(session: &Session, id: &str) -> Got<(Entry, Value)> {
    losses(session)?
        .into_iter()
        .find(|(_, data)| data["compaction"] == id)
        .ok_or_else(|| format!("no loss entry names {id}").into())
}

/// The session's index, read from beside its file.
pub(crate) fn index_of(session: &Session) -> Got<Index> {
    Ok(Index::read(session.file())?.1)
}

/// The bytes of an entry's line in the session file.
pub(crate) fn line_of(session: &Session, id: &str) -> Got<Vec<u8>> {
    let index = index_of(session)?;
    let row = index.row(id).ok_or("no row")?;
    let bytes = std::fs::read(session.file())?;
    let start = usize::try_from(row.offset)?;
    let end = usize::try_from(row.offset + row.len)?;
    Ok(bytes[start..end].to_vec())
}

/// Whether the line directly after `id`'s is the loss entry naming `id`.
fn loss_directly_after(session: &Session, id: &str) -> Got<bool> {
    let index = index_of(session)?;
    let row = index.row(id).ok_or("no row")?;
    let (loss, _) = loss_of(session, id)?;
    let loss_row = index.row(loss.id()).ok_or("no loss row")?;
    Ok(loss_row.offset == row.offset + row.len)
}

#[test]
fn the_fixture_pairs_become_two_compactions_under_the_summaries() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let s = &imported.session;
    assert_eq!(
        seen(s, &u(8))?,
        Seen {
            parent: Some(u(6)),
            summary: "Fixture summary one.".to_owned(),
            first_kept: u(5),
            tokens_before: 12345,
            details: json!({"boundaryUuid": u(7), "summaryUuid": u(8)}),
        }
    );
    let second = seen(s, &u(0x0e))?;
    assert_eq!(
        (second.parent, second.first_kept, second.tokens_before),
        (Some(u(0x0c)), u(0x0b), 23456)
    );
    Ok(())
}

#[test]
fn the_report_counts_compactions_and_their_sources_and_no_boundary_is_an_event() -> Gate {
    let (imported, report) = import_ok(&fixture()?)?;
    assert_eq!((report.compactions, report.compaction_sources), (2, 4));
    assert_eq!(
        report.events,
        BTreeMap::from([("tool_completed".to_owned(), 1)])
    );
    assert!(!report.counted_types.contains_key("system"));
    assert!(!imported.session.contains(&u(7))?);
    assert!(!imported.session.contains(&u(0x0d))?);
    Ok(())
}

#[test]
fn every_message_entry_keeps_its_source_parent() -> Gate {
    let records = fixture()?;
    let (imported, _) = import_ok(&records)?;
    let s = &imported.session;
    assert_eq!(s.entry(&u(9))?.parent_id(), Some(u(8).as_str()));
    assert_eq!(s.entry(&u(0x0f))?.parent_id(), Some(u(0x0e).as_str()));
    let mut checked = 0;
    for record in records
        .iter()
        .filter(|r| r["type"] == "user" || r["type"] == "assistant")
        .filter(|r| r.get("isCompactSummary").is_none())
    {
        let entry = s.entry(record["uuid"].as_str().ok_or("no uuid")?)?;
        assert!(matches!(entry.body, EntryBody::Message { .. }));
        assert_eq!(entry.parent_id(), record["parentUuid"].as_str());
        checked += 1;
    }
    assert_eq!(checked, 12);
    Ok(())
}

#[test]
fn a_boundary_without_a_preserved_segment_keeps_nothing_and_is_not_refused() -> Gate {
    let (imported, _) = import_ok(&without_segments()?)?;
    let first = seen(&imported.session, &u(8))?;
    assert_eq!((first.first_kept, first.parent), (u(8), Some(u(6))));
    Ok(())
}

#[test]
fn a_tail_off_record_falls_back_to_the_logical_parent_then_the_chain() -> Gate {
    let mut records = fixture()?;
    records[6]["compactMetadata"]["preservedSegment"]["tailUuid"] = json!(u(0xfe));
    let (logical, _) = import_ok(&records)?;
    assert_eq!(seen(&logical.session, &u(8))?.parent, Some(u(6)));
    records[6]["logicalParentUuid"] = json!(u(0xfd));
    let (chain, _) = import_ok(&records)?;
    assert_eq!(seen(&chain.session, &u(8))?.parent, Some(u(6)));
    Ok(())
}

#[test]
fn the_pair_is_matched_by_parent_not_by_adjacency() -> Gate {
    let (unmoved, _) = import_ok(&fixture()?)?;
    let mut records = fixture()?;
    let boundary = records.remove(6);
    records.insert(5, boundary);
    let (moved, _) = import_ok(&records)?;
    assert_eq!(seen(&moved.session, &u(8))?, seen(&unmoved.session, &u(8))?);
    Ok(())
}

#[test]
fn an_unknown_first_kept_record_is_refused_by_its_uuid_with_nothing_appended() -> Gate {
    let mut records = fixture()?;
    records[6]["compactMetadata"]["preservedSegment"]["headUuid"] = json!(u(0xff));
    let refused = import(&records)?;
    let Err(error) = &refused.report else {
        return Err("an unknown first kept record was accepted".into());
    };
    assert!(matches!(error, HomeError::UnknownFirstKept { .. }));
    assert!(error.to_string().contains(&u(0xff)));
    assert!(!refused.session.contains(&u(8))?);
    assert!(losses(&refused.session)?.is_empty());
    Ok(())
}

#[test]
fn a_summary_that_never_arrives_leaves_a_compaction_without_one_at_the_end() -> Gate {
    let (imported, report) = import_ok(&without_second_summary()?)?;
    assert_eq!((report.compactions, report.compaction_sources), (2, 3));
    let s = &imported.session;
    let text = std::fs::read_to_string(s.file())?;
    let last = text
        .lines()
        .rev()
        .take(2)
        .map(serde_json::from_str)
        .collect::<Result<Vec<Entry>, _>>()?;
    assert_eq!(last.len(), 2);
    assert_eq!(last[1].id(), u(0x0d));
    assert_eq!(
        seen(s, &u(0x0d))?,
        Seen {
            parent: Some(u(0x0c)),
            summary: String::new(),
            first_kept: u(0x0b),
            tokens_before: 23456,
            details: json!({"boundaryUuid": u(0x0d), "summary_missing": true}),
        }
    );
    assert!(last[0].is_custom(CUSTOM_LOSS));
    assert_eq!(data_of(&last[0])?["compaction"], json!(u(0x0d)));
    assert!(seen(s, &u(8))?.details.get("summary_missing").is_none());
    Ok(())
}

#[test]
fn the_head_is_the_last_compaction_or_the_last_record() -> Gate {
    let (cut, _) = import_ok(&without_second_summary()?)?;
    assert_eq!(cut.session.head()?, Some(u(0x0d).as_str()));
    let (whole, _) = import_ok(&fixture()?)?;
    assert_eq!(whole.session.head()?, Some(u(0x10).as_str()));
    Ok(())
}

#[test]
fn a_summary_record_keeps_nothing_and_counts_no_tokens() -> Gate {
    let (imported, report) = import_ok(&summary_file()?)?;
    assert_eq!((report.compactions, report.compaction_sources), (1, 1));
    let compactions: Vec<Entry> = imported
        .session
        .entries()?
        .into_iter()
        .filter(|e| matches!(e.body, EntryBody::Compaction { .. }))
        .collect();
    assert_eq!(compactions.len(), 1);
    let one = seen(&imported.session, compactions[0].id())?;
    assert_eq!(one.first_kept, compactions[0].id());
    assert_eq!(one.tokens_before, 0);
    Ok(())
}

#[test]
fn a_late_summary_completes_the_compaction_written_without_it() -> Gate {
    let records = with_late_summary()?;
    let (imported, report) = import_ok(&records)?;
    assert_eq!((report.compactions, report.compaction_sources), (3, 4));
    let s = &imported.session;
    assert_eq!(
        seen(s, &u(7))?,
        Seen {
            parent: Some(u(6)),
            summary: String::new(),
            first_kept: u(5),
            tokens_before: 12345,
            details: json!({"boundaryUuid": u(7), "summary_missing": true}),
        }
    );
    assert!(loss_directly_after(s, &u(7))?);
    let inserted = s.entry(&u(0x11))?;
    let EntryBody::Message { message } = &inserted.body else {
        return Err("the inserted record is not a message entry".into());
    };
    assert_eq!(message["role"], "user");
    assert_eq!(inserted.parent_id(), Some(u(7).as_str()));
    let summary = records[8]["message"]["content"]
        .as_str()
        .ok_or("no summary")?;
    assert_eq!(
        seen(s, &u(8))?,
        Seen {
            parent: Some(u(7)),
            summary: summary.to_owned(),
            first_kept: u(5),
            tokens_before: 12345,
            details: json!({"boundaryUuid": u(7), "summaryUuid": u(8), "completes": u(7)}),
        }
    );
    assert!(loss_directly_after(s, &u(8))?);
    assert_eq!(s.entry(&u(9))?.parent_id(), Some(u(8).as_str()));
    // The line `…07` had when `…11` was read: the import of the file cut
    // right after `…11`, where nothing followed that could rewrite it.
    let (cut, _) = import_ok(&records[..8])?;
    assert_eq!(line_of(s, &u(7))?, line_of(&cut.session, &u(7))?);
    Ok(())
}

/// A compaction renders as Claude Code 2.1.281 writes one (HOME-030 R6): the
/// boundary, the summary under it, then the kept and later records as one
/// chain, with no `summary` line, no custom entry and no loss entry; and the
/// rendered file, read as it was written, imports back as one compaction
/// with the same tokens and summary.
#[test]
fn a_compaction_renders_as_its_boundary_and_summary_and_imports_back_as_one() -> Gate {
    let records = fixture()?;
    let (imported, _) = import_ok(&records)?;
    let out = imported.dir.path().join("out").join("compacted.jsonl");
    let target = RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned(),
        cwd: "/elsewhere".to_owned(),
        model: "claude-fixture".to_owned(),
        version: "2.1.281".to_owned(),
        out: Some(out.clone()),
        canon: None,
    };
    render_claude_code(&imported.session, &target, None)?;
    let text = std::fs::read_to_string(&out)?;
    let lines = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?;
    let summary = &records[13]["message"]["content"];
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[0]["type"], "system");
    assert_eq!(lines[0]["subtype"], "compact_boundary");
    assert_eq!(lines[0]["parentUuid"], Value::Null);
    assert_eq!(lines[0]["compactMetadata"]["preTokens"], 23456);
    assert_eq!(lines[1]["type"], "user");
    assert_eq!(lines[1]["isCompactSummary"], true);
    assert_eq!(lines[1]["parentUuid"], lines[0]["uuid"]);
    assert_eq!(&lines[1]["message"]["content"], summary);
    assert_eq!(lines[2]["uuid"], json!(u(0x0b)));
    for pair in lines.windows(2).skip(1) {
        assert_eq!(pair[1]["parentUuid"], pair[0]["uuid"]);
    }
    assert!(
        lines
            .iter()
            .all(|l| l["type"] != "summary" && l["type"] != "custom")
    );
    assert!(!text.contains(CUSTOM_LOSS));
    let (again, _) = import_file(&out)?;
    let compactions: Vec<(String, u64)> = again
        .session
        .entries()?
        .into_iter()
        .filter_map(|e| match e.body {
            EntryBody::Compaction {
                summary,
                tokens_before,
                ..
            } => Some((summary, tokens_before)),
            _ => None,
        })
        .collect();
    let summary = summary.as_str().ok_or("no summary")?.to_owned();
    assert_eq!(compactions, [(summary, 23456)]);
    Ok(())
}
