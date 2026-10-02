#![cfg(test)]

//! A put-back returns the data the previous build last ran on: whatever the
//! new build wrote after the copy was kept, such as a directory record of a
//! kind the previous build cannot read, is gone, and what was there is back.

use std::collections::BTreeMap;
use std::error::Error;

use super::super::intent::{Intent, Step};
use super::{keep, restore};
use crate::identity::install::layout::Layout;

type TestResult = Result<(), Box<dyn Error>>;

fn intent() -> Intent {
    Intent {
        from: BTreeMap::new(),
        to: BTreeMap::new(),
        screens: false,
        screens_existed: false,
        files: Vec::new(),
        compose_changed: false,
        steps: Vec::new(),
    }
}

#[test]
fn a_put_back_removes_what_the_new_build_wrote_and_returns_what_was_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let log = layout.data_dir().join("directory-log");
    std::fs::create_dir_all(&log)?;
    std::fs::write(log.join("leaf-9"), b"a record the previous build reads")?;
    let mut intent = intent();
    keep(&layout, &mut intent)?;
    assert!(intent.has(Step::DataKept));

    std::fs::write(log.join("leaf-10"), b"an agent call record, kind 10")?;
    std::fs::write(log.join("leaf-9"), b"rewritten by the new build")?;
    let mut said = Vec::new();
    restore(&layout, &intent, &mut |words| said.push(words.to_owned()))?;

    assert!(
        !log.join("leaf-10").exists(),
        "a record the new build wrote is not left for the previous build"
    );
    assert_eq!(
        std::fs::read(log.join("leaf-9"))?,
        b"a record the previous build reads"
    );
    assert_eq!(said, ["the data the previous build last ran on is back"]);
    Ok(())
}

#[test]
fn nothing_is_put_back_when_this_upgrade_kept_no_data() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    std::fs::create_dir_all(layout.data_dir())?;
    std::fs::write(layout.data_dir().join("leaf-10"), b"kind 10")?;
    let mut said = Vec::new();
    restore(&layout, &intent(), &mut |words| said.push(words.to_owned()))?;
    assert!(layout.data_dir().join("leaf-10").exists());
    assert!(said.is_empty());
    Ok(())
}
