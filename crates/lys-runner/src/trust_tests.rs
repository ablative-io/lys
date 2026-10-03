#![cfg(test)]
//! The trust row: written once for a fresh folder, left alone when it is
//! there, and never at the cost of anything else in the file.

use std::error::Error;

use serde_json::{Value, json};

use crate::trust::{FILE, ROW, record};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_fresh_folder_gets_one_row_and_a_second_record_leaves_the_file_byte_identical() -> TestResult {
    let dir = tempfile::tempdir()?;
    let home = dir.path().join("home");
    let first = record(&home, "/work/one")?;
    assert!(first.written);
    assert_eq!(first.directory, "/work/one");
    let bytes = std::fs::read(home.join(FILE))?;
    let root: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(root["projects"]["/work/one"][ROW], Value::Bool(true));
    assert!(
        std::fs::read_dir(&home)?.count() == 1,
        "the sibling file is renamed away"
    );
    let second = record(&home, "/work/one")?;
    assert!(!second.written);
    assert_eq!(std::fs::read(home.join(FILE))?, bytes);
    Ok(())
}

#[test]
fn the_rest_of_the_file_and_other_rows_are_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let home = dir.path().to_path_buf();
    let before = json!({
        "numStartups": 4,
        "projects": {
            "/work/other": {ROW: true, "allowedTools": ["Read"]},
            "/work/one": {"allowedTools": ["Bash"]}
        }
    });
    std::fs::write(home.join(FILE), serde_json::to_vec_pretty(&before)?)?;
    let trust = record(&home, "/work/one")?;
    assert!(trust.written);
    let after: Value = serde_json::from_slice(&std::fs::read(home.join(FILE))?)?;
    assert_eq!(after["numStartups"], json!(4));
    assert_eq!(
        after["projects"]["/work/other"],
        before["projects"]["/work/other"]
    );
    assert_eq!(
        after["projects"]["/work/one"]["allowedTools"],
        json!(["Bash"])
    );
    assert_eq!(after["projects"]["/work/one"][ROW], Value::Bool(true));
    Ok(())
}

#[test]
fn a_file_that_is_not_an_object_is_refused_by_name_and_left_as_it_is() -> TestResult {
    let dir = tempfile::tempdir()?;
    let home = dir.path().to_path_buf();
    std::fs::write(home.join(FILE), b"[1, 2]")?;
    let err = record(&home, "/work/one").unwrap_err();
    assert_eq!(err.name(), "trust_file_invalid", "{err}");
    assert_eq!(std::fs::read(home.join(FILE))?, b"[1, 2]");
    Ok(())
}
