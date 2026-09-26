//! Gates on the fork (HOME-006 R3): the child's lines hash-equal the
//! parent's, `lys.forked_from` is the child's head with the ancestry as
//! data, the header names the parent file relative to the home, a carried
//! user message is counted and never copied, the parent gains exactly one
//! `lys.fork` line at its head with no earlier byte changed, no block is
//! written, and a held parent is refused with nothing written. The report
//! gates (R4) count entries and blocks, find an inline part's block by path
//! themselves, and search the report for every content sentinel.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_FORK, CUSTOM_FORKED_FROM, CUSTOM_HARNESS_EVENT, EntryBody};
use crate::record::fork::fork;
use crate::record::fork_cut_tests::{
    PARENT, SENTINELS, assistant, fixture_home, lantern_entry, put_parts, session_files, text_part,
    user,
};
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

/// The candidate hashes of a child's copied entries, computed here and not
/// by the crate: each message part's own SHA-256, and the record hash a
/// harness event names.
fn child_hashes(
    home: &Home,
    child: &str,
    copied: usize,
) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let reader = home.read_session(child)?;
    let ids: Vec<String> = reader.ids().take(copied).map(str::to_owned).collect();
    assert_eq!(ids.len(), copied);
    let mut hashes = BTreeSet::new();
    for id in ids {
        match reader.entry(&id)?.body {
            EntryBody::Message { message } => {
                for part in message["content"].as_array().ok_or("parts")? {
                    hashes.insert(Hash::of(&serde_json::to_vec(part)?).to_string());
                }
            }
            EntryBody::Custom {
                custom_type,
                data: Some(data),
            } if custom_type == CUSTOM_HARNESS_EVENT => {
                hashes.insert(data["record"].as_str().ok_or("a record hash")?.to_owned());
            }
            _ => {}
        }
    }
    Ok(hashes)
}

fn block_file(home: &Home, hash: &str) -> bool {
    home.root()
        .join("blocks")
        .join(&hash[..2])
        .join(hash)
        .is_file()
}

#[test]
fn the_report_counts_the_entries_copied_and_the_blocks_the_store_holds() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let report = fork(&home, &lanterns.l5, None)?;
    assert_eq!((report.entries, report.blocks, report.unstored), (7, 5, 0));
    let l6 = fork(&home, &lanterns.l6, None)?;
    assert_eq!(l6.cut_at, "e5");
    assert_eq!(l6.point, "e6");
    assert!(l6.coordinate_carried);
    assert_eq!(l6.carried.as_deref(), Some("e6"));
    Ok(())
}

#[test]
fn an_inline_part_whose_source_form_is_the_stored_block_counts_as_unstored() -> Gate {
    let (_dir, home, _) = fixture_home()?;
    let blocks = home.blocks()?;
    let m1_part = text_part("fixture-text-1");
    put_parts(&blocks, std::slice::from_ref(&m1_part))?;
    let thinking =
        json!({"type": "thinking", "thinking": "fixture-text-2", "thinkingSignature": "sig-m2"});
    let call =
        json!({"type": "toolCall", "id": "call-m2", "name": "fixture-tool", "arguments": {}});
    put_parts(
        &blocks,
        &[
            json!({"type": "thinking", "thinking": "fixture-text-2", "signature": "sig-m2"}),
            json!({"type": "tool_use", "id": "call-m2", "name": "fixture-tool", "input": {}}),
        ],
    )?;
    {
        let mut session = home.create_session("mixed", "/fixture", None)?;
        session.append_entry(&user("m1", None, &[m1_part]))?;
        session.append_entry(&assistant("m2", Some("m1"), &[thinking, call]))?;
        session.append_entry(&lantern_entry("M2", "m2", "m2", None))?;
    }
    let report = fork(&home, "M2", None)?;
    assert_eq!((report.entries, report.blocks, report.unstored), (2, 1, 2));
    let hashes = child_hashes(&home, &report.child, 2)?;
    assert_eq!(hashes.len(), 3);
    let held: Vec<&String> = hashes.iter().filter(|h| block_file(&home, h)).collect();
    assert_eq!(held.len(), 1);
    assert_eq!(hashes.len() - held.len(), 2);
    Ok(())
}

#[test]
fn two_forks_of_one_lantern_name_two_children_with_the_same_hashes() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let first = fork(&home, &lanterns.l5, None)?;
    let second = fork(&home, &lanterns.l5, None)?;
    assert_ne!(first.child, second.child);
    let a = child_hashes(&home, &first.child, 7)?;
    let b = child_hashes(&home, &second.child, 7)?;
    assert_eq!(a, b);
    assert_eq!(a.len(), 5);
    let text = serde_json::to_string(&first)?;
    let report: Value = serde_json::from_str(&text)?;
    let mut keys: Vec<&String> = report.as_object().ok_or("an object")?.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "blocks",
            "carried",
            "child",
            "coordinate_carried",
            "cut_at",
            "entries",
            "lantern",
            "parent",
            "point",
            "unstored"
        ]
    );
    for hash in &a {
        assert!(!text.contains(hash.as_str()));
    }
    Ok(())
}

#[test]
fn the_report_carries_no_content_sentinel() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let report = serde_json::to_string(&fork(&home, &lanterns.l6, None)?)?;
    let mut checked = 0;
    for sentinel in SENTINELS {
        assert!(!report.contains(sentinel));
        checked += 1;
    }
    assert_eq!(checked, 8);
    Ok(())
}
