//! Gates on the fork (HOME-006 R3): the child's lines hash-equal the
//! parent's, `lys.forked_from` is the child's head with the ancestry as
//! data, the header names the parent file relative to the home, a carried
//! user message is counted and never copied, the parent gains exactly one
//! `lys.fork` line at its head with no earlier byte changed, no block is
//! written, and a held parent is refused with nothing written.

use std::error::Error;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_FORK, CUSTOM_FORKED_FROM, EntryBody};
use crate::record::fork::fork;
use crate::record::fork_cut_tests::{PARENT, fixture_home, session_files};
use crate::record::index::Index;

type Gate = Result<(), Box<dyn Error>>;

/// The lines of a session file after its header, each with the SHA-256 of
/// its bytes, newline included.
fn line_hashes(file: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let bytes = std::fs::read(file)?;
    Ok(bytes
        .split_inclusive(|byte| *byte == b'\n')
        .skip(1)
        .map(|line| Hash::of(line).to_string())
        .collect())
}

/// The SHA-256 of one entry's line in a session file, by its index row.
fn line_hash_of(file: &Path, id: &str) -> Result<String, Box<dyn Error>> {
    let (_, index, _) = Index::read(file)?;
    let row = index.row(id).ok_or("a row")?;
    let line = crate::record::fork::read_line(file, row)?;
    Ok(Hash::of(&line).to_string())
}

/// The count of files and the total bytes under a directory, recursively.
pub(crate) fn files_and_bytes(dir: &Path) -> Result<(u64, u64), Box<dyn Error>> {
    let mut files = 0;
    let mut bytes = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let (f, b) = files_and_bytes(&entry.path())?;
            files += f;
            bytes += b;
        } else {
            files += 1;
            bytes += entry.metadata()?.len();
        }
    }
    Ok((files, bytes))
}

fn custom_data(home: &Home, session: &str, id: &str) -> Result<(String, Value), Box<dyn Error>> {
    let entry = home.read_session(session)?.entry(id)?;
    let EntryBody::Custom { custom_type, data } = entry.body else {
        return Err("not a custom entry".into());
    };
    Ok((custom_type, data.ok_or("no data")?))
}

#[test]
fn the_child_holds_the_parent_lines_then_forked_from_as_its_head() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let parent_file = home.session_path(PARENT)?;
    let report = fork(&home, &lanterns.l5, None)?;
    assert_ne!(report.child, PARENT);
    assert_eq!(report.entries, 7);
    let child_file = home.session_path(&report.child)?;
    let child_lines = line_hashes(&child_file)?;
    assert_eq!(child_lines.len(), 8);
    let copied = ["e1", "e2", "L2", "O2", "e3", "e4", "e5"];
    for (n, id) in copied.iter().enumerate() {
        assert_eq!(child_lines[n], line_hash_of(&parent_file, id)?, "{id}");
    }
    let reader = home.read_session(&report.child)?;
    assert_eq!(reader.len(), 8);
    assert_eq!(
        reader.header().parent_session.as_deref(),
        Some("sessions/parent.jsonl")
    );
    assert_eq!(reader.header().cwd, home.read_session(PARENT)?.header().cwd);
    let head = home.open_session(&report.child)?.head()?.map(str::to_owned);
    let eighth = reader.ids().nth(7).ok_or("an eighth entry")?.to_owned();
    assert_eq!(head.as_deref(), Some(eighth.as_str()));
    let entry = reader.entry(&eighth)?;
    assert_eq!(entry.parent_id(), Some("e5"));
    assert!(entry.is_custom(CUSTOM_FORKED_FROM));
    let (_, data) = custom_data(&home, &report.child, &eighth)?;
    assert_eq!(
        data,
        json!({"parent_session": "parent", "lantern": lanterns.l5, "point": "e5", "cut_at": "e5",
            "coordinate_carried": false, "carried": null, "seed_left_out": {}})
    );
    assert!(!report.coordinate_carried);
    assert_eq!(report.carried, None);
    Ok(())
}

#[test]
fn a_user_point_is_carried_and_counted_and_never_copied() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let report = fork(&home, &lanterns.l6, None)?;
    assert_eq!(report.cut_at, "e5");
    assert_eq!(report.point, "e6");
    assert!(report.coordinate_carried);
    assert_eq!(report.carried.as_deref(), Some("e6"));
    let reader = home.read_session(&report.child)?;
    assert!(!reader.contains("e6"));
    let head = reader.ids().last().ok_or("a head")?.to_owned();
    let (custom_type, data) = custom_data(&home, &report.child, &head)?;
    assert_eq!(custom_type, CUSTOM_FORKED_FROM);
    assert_eq!(
        data,
        json!({"parent_session": "parent", "lantern": lanterns.l6, "point": "e6", "cut_at": "e5",
            "coordinate_carried": true, "carried": "e6", "seed_left_out": {"image": 1}})
    );
    Ok(())
}

#[test]
fn the_parent_gains_one_fork_line_at_its_head_and_no_earlier_byte_changes() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let parent_file = home.session_path(PARENT)?;
    let before = std::fs::read(&parent_file)?;
    let head_before = home.open_session(PARENT)?.head()?.map(str::to_owned);
    let report = fork(&home, &lanterns.l5, None)?;
    let after = std::fs::read(&parent_file)?;
    assert_eq!(Hash::of(&after[..before.len()]), Hash::of(&before));
    let tail = &after[before.len()..];
    assert_eq!(
        tail.iter().position(|byte| *byte == b'\n'),
        Some(tail.len() - 1)
    );
    let line: Value = serde_json::from_slice(tail)?;
    assert_eq!(line["type"], "custom");
    assert_eq!(line["customType"], CUSTOM_FORK);
    assert_eq!(line["data"], json!({"child": report.child}));
    assert_eq!(line["parentId"], json!(head_before));
    let head_after = home.open_session(PARENT)?.head()?.map(str::to_owned);
    assert_eq!(head_after, line["id"].as_str().map(str::to_owned));
    assert!(head_after.is_some());
    Ok(())
}

#[test]
fn a_fork_writes_no_block() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let blocks = home.root().join("blocks");
    let before = files_and_bytes(&blocks)?;
    assert!(before.0 > 0);
    fork(&home, &lanterns.l5, None)?;
    fork(&home, &lanterns.l6, None)?;
    assert_eq!(files_and_bytes(&blocks)?, before);
    Ok(())
}

#[test]
fn a_parent_another_owner_holds_is_refused_and_nothing_is_written() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let before = session_files(&home)?;
    let held = home.open_session(PARENT)?;
    let refused = fork(&home, &lanterns.l5, None);
    assert!(
        matches!(&refused, Err(HomeError::SessionHeld { .. })),
        "{refused:?}"
    );
    drop(held);
    let after = session_files(&home)?;
    assert_eq!(after.len(), before.len());
    assert_eq!(after.get("parent.jsonl"), before.get("parent.jsonl"));
    Ok(())
}
