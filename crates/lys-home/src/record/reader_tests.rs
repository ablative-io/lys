//! Gates on the read-only reader (HOME-004 R2): it reads a held session,
//! writes nothing beside the file, leaves the lock alone, and lists every
//! session of a home and nothing else. The fixture home here is the one the
//! lantern, epilogue and recall gates build on.

use std::collections::BTreeMap;
use std::error::Error;

use serde_json::json;
use tempfile::TempDir;

use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::index::Index;
use crate::record::reader::SessionReader;
use crate::record::{Home, Session};

/// The session the fixture home holds.
pub(crate) const FIXTURE: &str = "fixture-lantern";
/// A line of message text the fixture carries, which no report may print.
pub(crate) const TRANSCRIPT_LINE: &str = "FIXTURE-TRANSCRIPT-LINE-q7";
/// The fixture's five entry ids, in file order; the head is the last.
pub(crate) const ENTRIES: [&str; 5] = ["e1", "e2", "e3", "e4", "e5"];

type Gate = Result<(), Box<dyn Error>>;

/// A message entry with the given id, text and parent.
pub(crate) fn message(id: &str, parent: Option<&str>, text: &str) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: parent.map(str::to_owned),
            timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
        },
        body: EntryBody::Message {
            message: json!({"role": "user", "content": [{"type": "text", "text": text}], "timestamp": 0}),
        },
    }
}

/// Append five message entries to a fresh session, the third carrying
/// [`TRANSCRIPT_LINE`], so the head is `e5`.
pub(crate) fn five_messages(session: &mut Session) -> Result<(), Box<dyn Error>> {
    let mut prev: Option<&str> = None;
    for (n, id) in ENTRIES.iter().enumerate() {
        let text = if n == 2 { TRANSCRIPT_LINE } else { "fixture" };
        session.append_entry(&message(id, prev, text))?;
        prev = Some(id);
    }
    Ok(())
}

/// A fresh home holding [`FIXTURE`] with its five entries, closed.
pub(crate) fn fixture_home() -> Result<(TempDir, Home), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session(FIXTURE, "/fixture", None)?;
    five_messages(&mut session)?;
    drop(session);
    Ok((dir, home))
}

/// Every file under the home's `sessions/` by name, with its bytes.
pub(crate) fn snapshot(home: &Home) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(home.root().join("sessions"))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        files.insert(name, std::fs::read(entry.path())?);
    }
    Ok(files)
}

fn session_file(home: &Home) -> Result<std::path::PathBuf, Box<dyn Error>> {
    Ok(home.session_path(FIXTURE)?)
}

#[test]
fn a_held_session_is_read_in_file_order() -> Gate {
    let (_dir, home) = fixture_home()?;
    let owner = home.open_session(FIXTURE)?;
    let reader = home.read_session(FIXTURE)?;
    let ids: Vec<&str> = reader.ids().collect();
    assert_eq!(ids, ENTRIES);
    assert_eq!(reader.len(), 5);
    assert_eq!(reader.header().id, FIXTURE);
    assert_eq!(reader.entry("e3")?.parent_id(), Some("e2"));
    drop(owner);
    Ok(())
}

#[test]
fn without_an_index_the_reader_scans_in_memory_and_writes_nothing() -> Gate {
    let (_dir, home) = fixture_home()?;
    let file = session_file(&home)?;
    {
        let mut owner = home.open_session(FIXTURE)?;
        owner.append(EntryBody::Custom {
            custom_type: "lys.test_reader".to_owned(),
            data: Some(json!({"n": 1})),
        })?;
    }
    let owned: Vec<Entry> = home
        .open_session(FIXTURE)?
        .customs_everywhere("lys.test_reader")?;
    std::fs::remove_file(Index::index_path(&file))?;
    let before = snapshot(&home)?;
    let reader = SessionReader::open(&file)?;
    assert!(reader.index_was_scanned());
    let read = reader.customs_everywhere("lys.test_reader")?;
    assert_eq!(read, owned);
    assert_eq!(read.len(), 1);
    assert_eq!(snapshot(&home)?, before);
    assert!(!Index::index_path(&file).exists());
    Ok(())
}

#[test]
fn an_absent_lock_stays_absent_across_a_read() -> Gate {
    let (_dir, home) = fixture_home()?;
    let file = session_file(&home)?;
    let lock = Index::lock_path(&file);
    std::fs::remove_file(&lock)?;
    let before = snapshot(&home)?;
    let reader = SessionReader::open(&file)?;
    assert_eq!(reader.ids().count(), 5);
    assert!(!lock.exists());
    assert_eq!(snapshot(&home)?, before);
    Ok(())
}

#[test]
fn the_home_lists_its_sessions_and_nothing_else() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    for id in ["fixture-b", "fixture-a", "x.index"] {
        let mut session = home.create_session(id, "/fixture", None)?;
        session.append_entry(&message("e1", None, "fixture"))?;
    }
    std::fs::write(home.root().join("sessions").join("notes.txt"), "notes\n")?;
    assert_eq!(home.session_ids()?, ["fixture-a", "fixture-b", "x.index"]);
    Ok(())
}

#[test]
fn a_missing_sessions_directory_lists_nothing() -> Gate {
    let dir = tempfile::tempdir()?;
    let ids = crate::record::reader::session_ids(&dir.path().join("nowhere"))?;
    assert!(ids.is_empty());
    Ok(())
}

#[test]
fn the_owner_opens_and_appends_after_a_read() -> Gate {
    let (_dir, home) = fixture_home()?;
    let reader = home.read_session(FIXTURE)?;
    assert!(reader.contains("e5"));
    let mut owner = home.open_session(FIXTURE)?;
    assert!(!owner.index_was_rebuilt());
    assert_eq!(owner.head()?, Some("e5"));
    let id = owner.append(EntryBody::Custom {
        custom_type: "lys.test_reader".to_owned(),
        data: None,
    })?;
    assert_eq!(owner.head()?, Some(id.as_str()));
    assert!(!reader.contains(&id));
    Ok(())
}
