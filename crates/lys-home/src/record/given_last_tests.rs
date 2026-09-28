#![cfg(test)]
//! Gates on the given entry appended last: none while nothing was given,
//! the greatest timestamp across every session whatever order the sessions
//! list in, and a session that cannot be read skipped and named while the
//! rest are still read.

use std::error::Error;

use crate::harness::claude_code::given::ConfigSource;
use crate::record::entries::{CUSTOM_GIVEN, Entry, EntryBase, EntryBody};
use crate::record::given::{GivenRecord, last_given};
use crate::record::given_tests::resolution;
use crate::record::reader_tests::message;
use crate::record::{Home, Session};

type Gate = Result<(), Box<dyn Error>>;

fn record(names: &[&str]) -> GivenRecord {
    GivenRecord::claude_code(
        resolution(ConfigSource::Template),
        names.iter().map(|name| (*name).to_owned()).collect(),
    )
}

fn give(session: &mut Session, id: &str, at: &str, given: &GivenRecord) -> Gate {
    session.append_entry(&Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: Some("m1".to_owned()),
            timestamp: at.to_owned(),
        },
        body: EntryBody::Custom {
            custom_type: CUSTOM_GIVEN.to_owned(),
            data: Some(given.data()?),
        },
    })?;
    Ok(())
}

fn session(home: &Home, id: &str) -> Result<Session, Box<dyn Error>> {
    let mut session = home.create_session(id, "/w", None)?;
    session.append_entry(&message("m1", None, "fixture"))?;
    Ok(session)
}

#[test]
fn a_home_given_nothing_has_no_last_given() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    drop(session(&home, "only")?);
    let seen = last_given(&home)?;
    assert_eq!(seen.last, None);
    assert!(seen.skipped.is_empty());
    Ok(())
}

#[test]
fn the_last_given_is_the_latest_of_every_session() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let latest = record(&["HOME", "PATH"]);
    {
        let mut early = session(&home, "a-listed-first")?;
        give(&mut early, "g1", "2026-01-01T00:00:00.000Z", &record(&[]))?;
        give(&mut early, "g3", "2026-01-03T00:00:00.000Z", &latest)?;
        let mut late = session(&home, "z-listed-last")?;
        give(
            &mut late,
            "g2",
            "2026-01-02T00:00:00.000Z",
            &record(&["HOME"]),
        )?;
    }
    let seen = last_given(&home)?;
    assert!(seen.skipped.is_empty());
    let last = seen.last.ok_or("nothing given")?;
    assert_eq!(last.session, "a-listed-first");
    assert_eq!(last.entry, "g3");
    assert_eq!(last.given_at, "2026-01-03T00:00:00.000Z");
    assert_eq!(last.record, latest);
    Ok(())
}

#[test]
fn a_session_that_cannot_be_read_is_skipped_and_named() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    {
        let mut kept = session(&home, "kept")?;
        give(&mut kept, "g1", "2026-01-01T00:00:00.000Z", &record(&[]))?;
    }
    std::fs::write(home.session_path("broken")?, "not json\n")?;
    let seen = last_given(&home)?;
    assert_eq!(seen.last.ok_or("nothing given")?.entry, "g1");
    assert_eq!(seen.skipped.len(), 1);
    assert_eq!(seen.skipped[0].session, "broken");
    Ok(())
}
