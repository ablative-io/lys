#![cfg(test)]

//! A log cursor reads forward from its process's offset, once per byte.

use std::error::Error;
use std::io::Write;

use super::LogCursor;

type TestResult = Result<(), Box<dyn Error>>;

fn append(path: &std::path::Path, text: &str) -> TestResult {
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

#[test]
fn what_came_before_the_offset_is_never_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let log = dir.path().join("service.log");
    std::fs::write(&log, "an earlier run: listening on 1\n")?;
    let offset = std::fs::metadata(&log)?.len();
    let mut cursor = LogCursor::at(&log, offset);
    assert!(
        !cursor.says("listening on"),
        "the earlier run's line was read"
    );
    append(&log, "starting\n")?;
    assert!(!cursor.says("listening on"));
    append(&log, "listen")?;
    assert!(!cursor.says("listening on"));
    append(&log, "ing on 2\n")?;
    assert!(cursor.says("listening on"), "a line split across two reads");
    Ok(())
}

#[test]
fn a_later_read_starts_where_the_last_one_ended() -> TestResult {
    let dir = tempfile::tempdir()?;
    let log = dir.path().join("service.log");
    std::fs::write(&log, "0123456789ab\n")?;
    let mut cursor = LogCursor::at(&log, 0);
    assert!(!cursor.says("ready"));
    std::fs::write(&log, "ready ready!\n")?;
    append(&log, "more\n")?;
    assert!(
        !cursor.says("ready"),
        "bytes before the last read's end were read again"
    );
    append(&log, "ready\n")?;
    assert!(cursor.says("ready"));
    Ok(())
}
