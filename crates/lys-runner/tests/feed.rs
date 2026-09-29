//! DIRECTORY-051 R1: the runner's feed keeps what it received from a
//! session's stream and that stream's cursor together or not at all. A unit
//! a stop cut short is cut off on the next start and read again from the
//! saved cursor, and the server pages the feed from an opaque cursor.

use std::error::Error;
use std::io::Write;

use lys_runner::tracking_store::{Body, Boundary, Commit, Feed, SourceState};

fn source(offset: u64) -> SourceState {
    SourceState {
        path: "/work/stream.jsonl".to_owned(),
        generation: 1,
        offset,
        bound: "/work".to_owned(),
        ..SourceState::default()
    }
}

fn turn(name: &str) -> Body {
    Body::Boundary(Boundary {
        boundary: name.to_owned(),
        turn: Some("turn-1".to_owned()),
    })
}

fn unit(offset: u64) -> Commit {
    Commit {
        source: Some(source(offset)),
        attempt: None,
        control: None,
    }
}

#[test]
fn a_committed_unit_and_its_cursor_survive_a_restart() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let mut feed = Feed::open(dir.path())?;
    let first = feed.append("s1", 10, vec![turn("turn_start")], unit(120))?;
    feed.append("s1", 11, vec![turn("turn_end")], unit(240))?;
    drop(feed);
    let feed = Feed::open(dir.path())?;
    assert_eq!(feed.source("s1").map(|kept| kept.offset), Some(240));
    let page = feed.page(None)?;
    let seqs: Vec<u64> = page.entries.iter().map(|entry| entry.seq).collect();
    assert_eq!(seqs.len(), 2, "commit lines are not served: {seqs:?}");
    assert_eq!(seqs[0], first);
    assert_eq!(page.entries[1].body, turn("turn_end"));
    assert!(!feed.after(Some(&page.cursor))?);
    Ok(())
}

#[test]
fn a_unit_a_stop_cut_short_is_cut_off_and_read_again() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let mut feed = Feed::open(dir.path())?;
    feed.append("s1", 10, vec![turn("turn_start")], unit(120))?;
    drop(feed);
    let path = dir.path().join("feed.jsonl");
    let whole = std::fs::metadata(&path)?.len();
    let mut file = std::fs::OpenOptions::new().append(true).open(&path)?;
    writeln!(
        file,
        "{}",
        serde_json::json!({"seq": 2, "at": 11, "session": "s1",
            "body": {"kind": "boundary", "entry": {"boundary": "turn_end", "turn": "turn-1"}}})
    )?;
    drop(file);
    let feed = Feed::open(dir.path())?;
    assert_eq!(std::fs::metadata(&path)?.len(), whole);
    assert_eq!(feed.source("s1").map(|kept| kept.offset), Some(120));
    assert_eq!(feed.page(None)?.entries.len(), 1);
    Ok(())
}

#[test]
fn another_feeds_cursor_is_refused() -> Result<(), Box<dyn Error>> {
    let (one, two) = (tempfile::TempDir::new()?, tempfile::TempDir::new()?);
    let mut first = Feed::open(one.path())?;
    first.append("s1", 10, vec![turn("turn_start")], unit(120))?;
    let cursor = first.page(None)?.cursor;
    let mut second = Feed::open(two.path())?;
    second.append("s2", 10, vec![turn("turn_start")], unit(120))?;
    let refused = second
        .page(Some(&cursor))
        .err()
        .ok_or("another feed's cursor was served")?;
    assert!(refused.to_string().contains("cursor_expired"), "{refused}");
    Ok(())
}
