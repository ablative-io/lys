#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the session record: Pi's grammar, the head, the path by seeking,
//! and the context path.

use std::io::Write;

use serde_json::json;

use crate::record::entries::{CUSTOM_AUTHORED, Entry, EntryBody};
use crate::record::{Home, Session};

fn message(role: &str, text: &str) -> EntryBody {
    EntryBody::Message {
        message: json!({"role": role, "content": text, "timestamp": 0}),
    }
}

fn home() -> (tempfile::TempDir, Home) {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    (dir, home)
}

#[test]
fn a_file_begins_with_pi_header_and_every_entry_carries_id_parent_and_timestamp() {
    let (dir, home) = home();
    let mut s = home.create_session("s1", "/work", None).unwrap();
    let a = s.append(message("user", "one")).unwrap();
    let b = s.append(message("assistant", "two")).unwrap();
    let text = std::fs::read_to_string(s.file()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(header["type"], "session");
    assert_eq!(header["version"], 2);
    assert_eq!(header["id"], "s1");
    assert_eq!(header["cwd"], "/work");
    for (line, (id, parent)) in lines[1..]
        .iter()
        .zip([(a.as_str(), None), (b.as_str(), Some(a.as_str()))])
    {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["type"], "message");
        assert_eq!(v["id"], id);
        assert_eq!(
            v["parentId"],
            parent.map_or(serde_json::Value::Null, |p| json!(p))
        );
        assert!(v["timestamp"].as_str().unwrap().ends_with('Z'));
    }
    drop(s);
    dir.close().unwrap();
}

#[test]
fn twelve_entries_one_compaction_and_a_moved_leaf_round_trip_through_pi_grammar() {
    let (dir, home) = home();
    let mut s = home.create_session("s2", "/work", None).unwrap();
    let mut ids = Vec::new();
    for i in 0..6 {
        ids.push(
            s.append(message(if i % 2 == 0 { "user" } else { "assistant" }, "t"))
                .unwrap(),
        );
    }
    let kept = ids[4].clone();
    ids.push(
        s.append(EntryBody::Compaction {
            summary: "so far".into(),
            first_kept_entry_id: kept.clone(),
            tokens_before: 1000,
            rest: serde_json::Map::new(),
        })
        .unwrap(),
    );
    for _ in 0..3 {
        ids.push(s.append(message("user", "after")).unwrap());
    }
    ids.push(
        s.append(EntryBody::Label {
            target_id: ids[0].clone(),
            label: Some("start".into()),
        })
        .unwrap(),
    );
    ids.push(
        s.append(EntryBody::Custom {
            custom_type: CUSTOM_AUTHORED.into(),
            data: None,
        })
        .unwrap(),
    );
    assert_eq!(s.len().unwrap(), 12);
    // Every line parses back to the same entry.
    let text = std::fs::read_to_string(s.file()).unwrap();
    for line in text.lines().skip(1) {
        let e: Entry = serde_json::from_str(line).unwrap();
        let again = serde_json::to_string(&e).unwrap();
        let v1: serde_json::Value = serde_json::from_str(line).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&again).unwrap();
        assert_eq!(v1, v2);
    }
    // The context path: compaction first, then from the kept entry on.
    let ctx = s.context_path().unwrap();
    assert!(matches!(ctx[0].body, EntryBody::Compaction { .. }));
    assert_eq!(ctx[1].id(), kept);
    assert_eq!(ctx[2].id(), ids[5]);
    assert_eq!(ctx.len(), 1 + 2 + 5);
    // Move the leaf back to entry 4 and append: earlier bytes unchanged.
    let before = std::fs::read(s.file()).unwrap();
    s.move_head(Some(&ids[3])).unwrap();
    let thirteenth = s.append(message("user", "branch")).unwrap();
    let after = std::fs::read(s.file()).unwrap();
    assert_eq!(&after[..before.len()], &before[..]);
    let e = s.entry(&thirteenth).unwrap();
    assert_eq!(e.parent_id(), Some(ids[3].as_str()));
    assert_eq!(s.path().unwrap().0.len(), 5);
    drop(s);
    dir.close().unwrap();
}

#[test]
fn reopening_restores_the_persisted_head_not_the_last_entry() {
    let (dir, home) = home();
    let file = {
        let mut s = home.create_session("s3", "/work", None).unwrap();
        let a = s.append(message("user", "a")).unwrap();
        let _b = s.append(message("assistant", "b")).unwrap();
        let _c = s.append(message("user", "c")).unwrap();
        s.move_head(Some(&a)).unwrap();
        s.file().to_path_buf()
    };
    let s = Session::open(&file).unwrap();
    assert!(!s.index_was_rebuilt());
    let head = s.head().unwrap().unwrap().to_owned();
    assert_eq!(s.entry(&head).unwrap().parent_id(), None);
    assert_eq!(s.path().unwrap().0.len(), 1);
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_missing_index_is_rebuilt_and_a_stale_one_is_refused_then_rebuilt() {
    let (dir, home) = home();
    let (file, index) = {
        let mut s = home.create_session("s4", "/work", None).unwrap();
        for _ in 0..4 {
            s.append(message("user", "x")).unwrap();
        }
        (
            s.file().to_path_buf(),
            crate::record::index::Index::index_path(s.file()),
        )
    };
    std::fs::remove_file(&index).unwrap();
    let s = Session::open(&file).unwrap();
    assert!(s.index_was_rebuilt());
    assert_eq!(s.len().unwrap(), 4);
    drop(s);
    // Truncate the index to two rows: it no longer reaches the end of the file.
    let rows = std::fs::read_to_string(&index).unwrap();
    let two = rows.lines().take(2).fold(String::new(), |mut s, l| {
        s.push_str(l);
        s.push('\n');
        s
    });
    std::fs::write(&index, two).unwrap();
    let s = Session::open(&file).unwrap();
    assert!(s.index_was_rebuilt());
    assert_eq!(s.len().unwrap(), 4);
    drop(s);
    dir.close().unwrap();
}

#[test]
fn reading_a_short_path_of_a_large_file_reads_only_that_path() {
    let (dir, home) = home();
    let mut s = home.create_session("s5", "/work", None).unwrap();
    let filler = "f".repeat(4096);
    // 50 entries on the main path, then 2,000 entries on a side branch.
    let mut main = Vec::new();
    for _ in 0..50 {
        main.push(s.append(message("user", "m")).unwrap());
    }
    s.move_head(Some(&main[0])).unwrap();
    for _ in 0..2000 {
        s.append(message("assistant", &filler)).unwrap();
    }
    let size = std::fs::metadata(s.file()).unwrap().len();
    assert!(size > 8_000_000, "file is {size} bytes");
    s.move_head(Some(&main[49])).unwrap();
    let (path, read) = s.path().unwrap();
    assert_eq!(path.len(), 50);
    assert!(read < 1_000_000, "read {read} bytes");
    drop(s);
    dir.close().unwrap();
}

#[test]
fn an_unknown_parent_is_refused_by_name_and_creating_over_a_file_is_refused() {
    let (dir, home) = home();
    let mut s = home.create_session("s6", "/work", None).unwrap();
    let e = Entry {
        base: crate::record::entries::EntryBase {
            id: "x".into(),
            parent_id: Some("nope".into()),
            timestamp: "t".into(),
        },
        body: message("user", "?"),
    };
    let err = s.append_entry(&e).unwrap_err().to_string();
    assert!(err.contains("`x`") && err.contains("`nope`"), "{err}");
    assert_eq!(s.len().unwrap(), 0);
    let err = s.move_head(Some("ghost")).unwrap_err().to_string();
    assert!(err.contains("`ghost`"), "{err}");
    // While this owner holds it, a second creator is refused as a second owner;
    // once released, creating over the file is refused as an existing file.
    let err = home.create_session("s6", "/work", None).unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::SessionHeld { .. }),
        "{err}"
    );
    drop(s);
    let err = home
        .create_session("s6", "/work", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("already exists"), "{err}");
    dir.close().unwrap();
}

#[test]
fn one_owner_at_a_time_and_a_duplicate_or_unsafe_id_is_refused_by_name() {
    let (dir, home) = home();
    let mut s = home.create_session("owned", "/work", None).unwrap();
    let err = home.open_session("owned").unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::SessionHeld { .. }),
        "{err}"
    );
    let err = home.create_session("owned", "/work", None).unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::SessionHeld { .. }),
        "{err}"
    );
    let first = s.append(message("user", "hello")).unwrap();
    let again = Entry {
        base: crate::record::entries::EntryBase {
            id: first.clone(),
            parent_id: Some(first.clone()),
            timestamp: "t".into(),
        },
        body: message("user", "again"),
    };
    let err = s.append_entry(&again).unwrap_err();
    assert!(
        matches!(err, crate::error::HomeError::DuplicateEntry { .. }),
        "{err}"
    );
    assert_eq!(s.len().unwrap(), 1);
    assert_eq!(s.path().unwrap().0.len(), 1, "the ancestry still ends");
    drop(s);
    let s = home.open_session("owned").unwrap();
    assert_eq!(
        s.head().unwrap(),
        Some(first.as_str()),
        "released on drop, the next owner opens it"
    );
    drop(s);
    for bad in ["../x", "a/b", "", ".hidden", "with space", "/abs"] {
        let err = home.open_session(bad).unwrap_err();
        assert!(
            matches!(err, crate::error::HomeError::BadName { .. }),
            "{bad}: {err}"
        );
    }
    assert!(
        !home
            .root()
            .join("sessions")
            .join("..")
            .join("x.jsonl")
            .exists()
    );
    dir.close().unwrap();
}

#[test]
fn a_failed_index_write_after_a_durable_line_is_reconciled_before_the_next_append() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, home) = home();
    let mut s = home.create_session("recon", "/work", None).unwrap();
    let a = s.append(message("user", "one")).unwrap();
    let index_file = crate::record::index::Index::index_path(s.file());
    // Inject: the index file refuses appends; the session file still takes them.
    std::fs::set_permissions(&index_file, std::fs::Permissions::from_mode(0o444)).unwrap();
    let b = s.append(message("assistant", "two")).unwrap();
    assert_eq!(
        s.reconciliations(),
        1,
        "the index row could not be appended, so the index was rebuilt from the file"
    );
    assert_eq!(s.len().unwrap(), 2);
    assert_eq!(s.head().unwrap(), Some(b.as_str()));
    assert_eq!(s.entry(&b).unwrap().parent_id(), Some(a.as_str()));
    // The rebuilt index is a fresh file that takes appends again.
    let c = s.append(message("user", "three")).unwrap();
    assert_eq!(s.reconciliations(), 1);
    assert_eq!(s.len().unwrap(), 3);
    drop(s);
    let s = home.open_session("recon").unwrap();
    assert!(
        !s.index_was_rebuilt(),
        "on disk the index and the file agree"
    );
    assert_eq!(s.head().unwrap(), Some(c.as_str()));
    let (path, _) = s.path().unwrap();
    assert_eq!(
        path.iter().map(Entry::id).collect::<Vec<_>>(),
        [a.as_str(), b.as_str(), c.as_str()]
    );
    drop(s);
    dir.close().unwrap();
}

#[test]
fn a_file_with_a_duplicate_id_or_a_parent_after_its_child_is_refused_on_rebuild() {
    let (dir, home) = home();
    let s = home.create_session("bent", "/work", None).unwrap();
    let file = s.file().to_path_buf();
    drop(s);
    let index_file = crate::record::index::Index::index_path(&file);
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap();
    writeln!(
        f,
        r#"{{"type":"label","id":"x","parentId":null,"timestamp":"t","targetId":"x","label":null}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"type":"label","id":"x","parentId":"x","timestamp":"t","targetId":"x","label":null}}"#
    )
    .unwrap();
    drop(f);
    assert!(
        !index_file.exists(),
        "a fresh session has no index file until an entry is appended"
    );
    let err = Session::open(&file).unwrap_err().to_string();
    assert!(err.contains("already on record"), "{err}");
    std::fs::write(
        &file,
        std::fs::read_to_string(&file)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_owned()
            + "\n",
    )
    .unwrap();
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap();
    writeln!(f, r#"{{"type":"label","id":"y","parentId":"later","timestamp":"t","targetId":"y","label":null}}"#).unwrap();
    drop(f);
    let err = Session::open(&file).unwrap_err().to_string();
    assert!(err.contains("not on record before it"), "{err}");
    dir.close().unwrap();
}

#[test]
fn a_primary_write_fault_and_a_head_fault_each_leave_the_session_in_step_with_the_disk() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, home) = home();
    let mut s = home.create_session("faults", "/work", None).unwrap();
    let a = s.append(message("user", "one")).unwrap();
    // The session file refuses appends: nothing is written, the session is stale
    // until it has looked, and reads refuse by name meanwhile.
    std::fs::set_permissions(s.file(), std::fs::Permissions::from_mode(0o444)).unwrap();
    let err = s.append(message("assistant", "two")).unwrap_err();
    assert!(matches!(err, crate::error::HomeError::Io { .. }), "{err}");
    assert_eq!(
        s.reconciliations(),
        1,
        "looked at the file before answering anything"
    );
    assert_eq!(s.len().unwrap(), 1);
    assert_eq!(s.head().unwrap(), Some(a.as_str()));
    std::fs::set_permissions(s.file(), std::fs::Permissions::from_mode(0o644)).unwrap();
    let b = s.append(message("assistant", "two")).unwrap();
    assert_eq!(s.len().unwrap(), 2);
    // The head cannot be published (its directory refuses the temporary file):
    // the entry is durable and indexed, the head stays where the disk has it.
    let sessions = s.file().parent().unwrap().to_path_buf();
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o555)).unwrap();
    let err = s.append(message("user", "three")).unwrap_err();
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(err, crate::error::HomeError::Io { .. }), "{err}");
    assert_eq!(
        s.len().unwrap(),
        3,
        "the third entry is durable and indexed"
    );
    assert_eq!(
        s.head().unwrap(),
        Some(b.as_str()),
        "the head is what the disk holds"
    );
    let err = s.move_head(Some("nowhere")).unwrap_err().to_string();
    assert!(err.contains("`nowhere`"), "{err}");
    drop(s);
    let s = home.open_session("faults").unwrap();
    assert_eq!(s.head().unwrap(), Some(b.as_str()));
    assert_eq!(s.len().unwrap(), 3);
    assert!(!s.index_was_rebuilt());
    drop(s);
    dir.close().unwrap();
}
