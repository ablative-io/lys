#![cfg(test)]

//! A put-back returns the data the previous build last ran on and removes
//! none of what the new build wrote: the window's data is moved whole to
//! `data.rolled-back/<rfc3339>/`, counted store by store, and every store it
//! shrank is recorded beside the receipt (DIRECTORY-107 R1). The server's
//! join records and this computer's saved join are kept and put back as one
//! set (R3).

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use lys_log_store::{FileLeafStore, FrontierLog};

use super::super::intent::{Intent, Step};
use super::{keep, keep_for_organisations, restore, restore_at};
use crate::identity::install::layout::Layout;

type TestResult = Result<(), Box<dyn Error>>;

const ORIGIN: &str = "example.test/lys/directory";

fn intent() -> Intent {
    Intent {
        from: BTreeMap::new(),
        to: BTreeMap::new(),
        screens: false,
        screens_existed: false,
        files: Vec::new(),
        compose_changed: false,
        steps: Vec::new(),
        organisation_copy: None,
        clients_placed: crate::identity::install::layout::PlacedClients::default(),
    }
}

/// Every file under `dir` with its bytes, by its path under `dir`.
fn tree(dir: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(path.strip_prefix(dir)?.to_path_buf(), std::fs::read(&path)?);
            }
        }
    }
    Ok(files)
}

/// Append `count` records to the log at `dir`, creating it first when absent.
fn append(dir: &Path, count: u64) -> TestResult {
    if !dir.exists() {
        FileLeafStore::create(dir, ORIGIN)?;
    }
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(dir)?)?;
    for record in 0..count {
        log.append(format!("record {record}").as_bytes())?;
    }
    Ok(())
}

fn records(dir: &Path) -> Result<u64, Box<dyn Error>> {
    Ok(lys_log_store::LeafStore::extent(
        &FileLeafStore::open_read_only(dir)?,
    ))
}

/// The one folder under `data.rolled-back/`.
fn rolled_back(layout: &Layout) -> Result<PathBuf, Box<dyn Error>> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(layout.root.join("data.rolled-back"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()?;
    folders.retain(|path| path.is_dir() && path.extension().is_none_or(|ext| ext != "join-set"));
    match folders.as_slice() {
        [one] => Ok(one.clone()),
        other => Err(format!("expected one rolled-back folder, found {other:?}").into()),
    }
}

#[test]
fn a_put_back_moves_the_windows_data_aside_whole_and_removes_none_of_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let log = layout.data_dir().join("directory-log");
    append(&log, 2)?;
    std::fs::write(layout.data_dir().join("roles.json"), b"kept roles")?;
    let mut intent = intent();
    keep(&layout, &mut intent)?;

    append(&log, 2)?;
    std::fs::write(layout.data_dir().join("roles.json"), b"window roles")?;
    std::fs::write(
        layout.data_dir().join("leaf-10"),
        b"an agent call record, kind 10",
    )?;
    let window = tree(&layout.data_dir())?;
    let mut said = Vec::new();
    restore(&layout, &intent, &mut |words| said.push(words.to_owned()))?;

    let aside = rolled_back(&layout)?;
    let name = aside
        .file_name()
        .ok_or("no name")?
        .to_string_lossy()
        .into_owned();
    assert!(
        chrono::DateTime::parse_from_rfc3339(&name).is_ok(),
        "the folder is named by the instant the put-back began: {name}"
    );
    assert_eq!(tree(&aside)?, window, "the window's data, byte for byte");
    assert_eq!(
        records(&log)?,
        2,
        "the previous build reads every pre-window record"
    );
    assert_eq!(
        records(&aside.join("directory-log"))?,
        4,
        "every window record is found aside"
    );
    assert_eq!(
        std::fs::read(layout.data_dir().join("roles.json"))?,
        b"kept roles"
    );
    assert!(!layout.data_dir().join("leaf-10").exists());
    assert!(layout.data_previous_dir().is_dir(), "the kept copy stays");
    let folder = format!("{}", aside.display());
    assert!(said.iter().any(|line| line.contains(&folder)), "{said:?}");
    assert!(
        said.iter().any(|line| line == "  directory-log: 4 records"),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|line| line.starts_with("  roles.json: not counted here")),
        "a store it cannot count is named with the reason: {said:?}"
    );
    assert!(
        said.iter()
            .any(|line| line.starts_with("  leaf-10: not counted here")),
        "{said:?}"
    );
    Ok(())
}

#[test]
fn a_put_back_records_each_store_it_shrank_beside_the_receipt() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let log = layout.data_dir().join("grant-log");
    append(&log, 3)?;
    append(&layout.data_dir().join("seats"), 1)?;
    let mut intent = intent();
    keep(&layout, &mut intent)?;
    append(&log, 2)?;
    restore(&layout, &intent, &mut |_| {})?;

    let entries = lys_identity::stores_receipt::read_acceptances(
        &layout.install_dir().join("stores.accepted.json"),
    )?;
    assert_eq!(entries.len(), 1, "only the store that shrank: {entries:?}");
    let entry = &entries[0];
    assert_eq!(entry.kind, lys_identity::stores_receipt::Kind::RolledBack);
    assert_eq!(entry.store, "grant-log");
    assert_eq!(
        (entry.held.as_str(), entry.found.as_str()),
        ("5 records", "3 records")
    );
    assert!(chrono::DateTime::parse_from_rfc3339(&entry.at).is_ok());
    Ok(())
}

#[test]
fn a_taken_rolled_back_name_is_refused_by_name_and_moves_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    std::fs::create_dir_all(layout.data_dir())?;
    std::fs::write(layout.data_dir().join("roles.json"), b"kept")?;
    let mut intent = intent();
    keep(&layout, &mut intent)?;
    std::fs::write(layout.data_dir().join("roles.json"), b"window")?;
    let at = "2026-10-19T01:02:03.000004Z";
    let taken = layout.root.join("data.rolled-back").join(at);
    std::fs::create_dir_all(&taken)?;
    let refused = restore_at(&layout, &intent, at, &mut |_| {})
        .err()
        .ok_or("a put-back moved data onto a taken name")?;
    assert_eq!(refused.kind().name(), "rolled_back_name_taken");
    assert!(refused.to_string().contains(at), "{refused}");
    assert_eq!(
        std::fs::read(layout.data_dir().join("roles.json"))?,
        b"window"
    );
    assert_eq!(std::fs::read_dir(&taken)?.count(), 0, "nothing was moved");
    Ok(())
}

#[test]
fn the_join_set_is_kept_and_put_back_together_and_the_windows_copies_are_kept_aside() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let run = layout.run_dir();
    std::fs::create_dir_all(&run)?;
    std::fs::create_dir_all(layout.data_dir().join("runner").join("owners"))?;
    std::fs::create_dir_all(layout.service_key().parent().ok_or("no keys folder")?)?;
    std::fs::write(layout.service_key(), [9; 32])?;
    std::fs::write(layout.runner_machine_key(), [7; 32])?;
    std::fs::write(layout.data_dir().join("network.json"), b"kept machines")?;
    std::fs::write(layout.data_dir().join("network.joins.json"), b"kept codes")?;
    std::fs::write(run.join("runner-join.json"), b"kept join")?;
    std::fs::write(run.join("runner-server.pub"), b"kept server key")?;
    let mut intent = intent();
    keep(&layout, &mut intent)?;

    std::fs::write(layout.data_dir().join("network.json"), b"window machines")?;
    std::fs::write(layout.data_dir().join("network.joins.json"), b"window code")?;
    std::fs::write(run.join("runner-join.json"), b"window join")?;
    std::fs::write(run.join("runner-server.pub"), b"window server key")?;
    restore(&layout, &intent, &mut |_| {})?;

    for (path, kept) in [
        (
            layout.data_dir().join("network.json"),
            &b"kept machines"[..],
        ),
        (layout.data_dir().join("network.joins.json"), b"kept codes"),
        (run.join("runner-join.json"), b"kept join"),
        (run.join("runner-server.pub"), b"kept server key"),
        (layout.service_key(), &[9; 32]),
        (layout.runner_machine_key(), &[7; 32]),
    ] {
        assert_eq!(
            std::fs::read(&path)?,
            kept,
            "{} is from the kept moment",
            path.display()
        );
    }
    assert!(layout.data_dir().join("runner").join("owners").is_dir());
    let aside = rolled_back(&layout)?;
    assert_eq!(
        std::fs::read(aside.join("network.json"))?,
        b"window machines"
    );
    assert_eq!(
        std::fs::read(aside.join("network.joins.json"))?,
        b"window code"
    );
    // The instant's fraction holds a dot, so the name is extended, never
    // given a new extension.
    let mut joins_name = aside.file_name().ok_or("no instant")?.to_os_string();
    joins_name.push(".join-set");
    let joins = aside.with_file_name(joins_name);
    assert_eq!(
        std::fs::read(joins.join("run").join("runner-join.json"))?,
        b"window join"
    );
    assert_eq!(
        std::fs::read(joins.join("run").join("runner-server.pub"))?,
        b"window server key"
    );
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
    assert!(!layout.root.join("data.rolled-back").exists());
    Ok(())
}

#[test]
fn the_record_names_the_kept_data_so_a_release_that_cannot_put_it_back_refuses_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    std::fs::create_dir_all(layout.data_dir())?;
    let mut intent = intent();
    intent.write(&layout)?;
    keep(&layout, &mut intent)?;
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(layout.upgrade_intent())?)?;
    assert_eq!(
        record["steps"],
        serde_json::json!(["data_kept"]),
        "a release that knows no such step refuses the record before it touches anything"
    );
    assert!(layout.data_previous_dir().is_dir());
    assert!(intent.has(Step::DataKept));
    Ok(())
}

#[test]
fn the_stores_before_organisations_are_kept_once_and_named_on_the_record() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    append(&layout.data_dir().join("directory-log"), 3)?;
    append(&layout.data_dir().join("grant-log"), 2)?;
    std::fs::write(layout.data_dir().join("sessions.json"), b"not a store")?;
    let mut intent = intent();
    intent.write(&layout)?;
    keep_for_organisations(&layout, &mut intent)?;
    let copy = intent
        .organisation_copy
        .clone()
        .ok_or("the record names no organisation copy")?;
    let name = copy
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the copy has no name")?;
    assert!(
        chrono::DateTime::parse_from_rfc3339(name).is_ok(),
        "the copy is named by the instant it was made: {name}"
    );
    assert_eq!(
        copy.parent(),
        Some(layout.root.join("data.previous-organisation").as_path())
    );
    assert_eq!(records(&copy.join("directory-log"))?, 3);
    assert_eq!(records(&copy.join("grant-log"))?, 2);
    assert!(
        !copy.join("sessions.json").exists(),
        "only the two stores are copied"
    );
    let read = Intent::read(&layout)?.ok_or("the record is gone")?;
    assert_eq!(read.organisation_copy, Some(copy.clone()));
    // A later upgrade finds the copy and leaves it: the record then names none.
    append(&layout.data_dir().join("directory-log"), 4)?;
    // The record above shadows the fixture's maker.
    let mut later = self::intent();
    keep_for_organisations(&layout, &mut later)?;
    assert_eq!(later.organisation_copy, None);
    assert_eq!(records(&copy.join("directory-log"))?, 3);
    let folders = std::fs::read_dir(layout.root.join("data.previous-organisation"))?.count();
    assert_eq!(folders, 1);
    Ok(())
}

#[test]
fn an_install_with_no_store_yet_keeps_nothing_for_organisations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    std::fs::create_dir_all(layout.data_dir())?;
    let mut intent = intent();
    keep_for_organisations(&layout, &mut intent)?;
    assert_eq!(intent.organisation_copy, None);
    assert!(!layout.root.join("data.previous-organisation").exists());
    Ok(())
}
