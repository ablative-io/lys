#![cfg(test)]

use lys_install::install::layout::Layout;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_record_is_beside_the_data_folder_so_it_outlives_it() {
    let layout = Layout::at("/data/lys/identity".into());
    assert_eq!(
        record_path(&layout),
        Path::new("/data/lys/last-uninstall.json")
    );
}

#[test]
fn keeping_the_data_says_the_same_people_sign_in_again() {
    let kept = record_of(&Ok(()), false);
    assert!(kept.done);
    assert!(!kept.removed_data);
    assert!(kept.words.contains("data folder was kept"));
    assert!(kept.words.contains("signs the same people in"));
    let removed = record_of(&Ok(()), true);
    assert!(removed.removed_data);
    assert!(removed.words.contains("data folder was removed"));
}

#[test]
fn a_refused_uninstall_removes_nothing_it_says_and_names_what_to_do() {
    let refusal = Refusal::new(
        "engine_not_running",
        "Nothing was removed, because the container engine is not running.",
        "Open Docker Desktop, then press Uninstall again.",
        "detail",
    );
    let record = record_of(&Err(refusal), true);
    assert!(!record.done);
    assert!(!record.removed_data);
    assert!(record.words.contains("Nothing was removed"));
    assert!(record.words.contains("press Uninstall again"));
    assert!(!record.words.contains("detail"));
}

/// The record is shown on the next opening, once.
#[test]
fn the_note_is_shown_once() -> TestResult {
    let parent = tempfile::tempdir()?;
    let layout = Layout::at(parent.path().join("identity"));
    assert_eq!(take_note(&layout)?, None);
    write_record(&layout, &record_of(&Ok(()), false))?;
    let note = take_note(&layout)?.ok_or("no note")?;
    assert!(note.contains("data folder was kept"));
    assert_eq!(take_note(&layout)?, None);
    Ok(())
}

#[test]
fn an_unreadable_record_is_refused_by_name() -> TestResult {
    let parent = tempfile::tempdir()?;
    let layout = Layout::at(parent.path().join("identity"));
    std::fs::write(record_path(&layout), "{")?;
    let refused = take_note(&layout).err().map(|refusal| refusal.name);
    assert_eq!(refused, Some("uninstall_failed"));
    Ok(())
}
