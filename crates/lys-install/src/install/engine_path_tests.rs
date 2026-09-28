#![cfg(test)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_finder_path_is_extended_with_every_engine_folder_once() {
    let finder = OsString::from("/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin");
    let home = Path::new("/Users/someone");
    let dirs = search_dirs(Some(&finder), Some(home));
    assert_eq!(
        dirs[..5],
        [
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/usr/local/bin"),
        ]
    );
    let local = dirs
        .iter()
        .filter(|dir| dir.as_path() == Path::new("/usr/local/bin"));
    assert_eq!(local.count(), 1, "{dirs:?}");
    for expected in engine_dirs(Some(home)) {
        assert!(
            dirs.contains(&expected),
            "{expected:?} missing from {dirs:?}"
        );
    }
    assert!(dirs.contains(&home.join(".docker").join("bin")));
}

#[test]
fn with_no_path_given_only_the_engine_folders_are_searched() {
    let dirs = search_dirs(None, None);
    assert_eq!(dirs, engine_dirs(None));
    assert_eq!(dirs.len(), 3);
}

#[test]
fn the_first_folder_holding_the_program_names_it() -> TestResult {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    let third = tempfile::tempdir()?;
    std::fs::write(second.path().join(DOCKER), b"")?;
    std::fs::write(third.path().join(DOCKER), b"")?;
    let dirs = [first.path(), second.path(), third.path()].map(Path::to_path_buf);
    assert_eq!(find(DOCKER, &dirs), second.path().join(DOCKER));
    Ok(())
}

#[test]
fn a_program_found_nowhere_is_left_for_its_start_to_refuse() -> TestResult {
    let empty = tempfile::tempdir()?;
    let dirs = [empty.path().to_path_buf()];
    assert_eq!(find(DOCKER, &dirs), PathBuf::from(DOCKER));
    Ok(())
}
