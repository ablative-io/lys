#![cfg(test)]

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::{FileLeafStore, FrontierLog, Start};

use super::{ORIGIN, SeatStore};
use crate::error::ServerError;
use crate::error_agents::AgentsError;
use crate::error_seat::SeatError;
use crate::seats_state::{Added, DOMAIN, Held, Line, Retired, Started, Stopped};

type TestResult = Result<(), Box<dyn Error>>;

const LEGACY_LINES: &str = r#"[
  {"added":{"operation":"add","name":"fixture-seat","agent":"agent",
    "harness":"claude","profile_version":1,"machine":"machine",
    "working_folder":null,"account":null,"by":"person","at":1}},
  {"started":{"operation":"start","name":"fixture-seat","session":"start",
    "harness_session":"known-native-conversation","by":"person","at":2}},
  {"stopped":{"operation":"stop","name":"fixture-seat","session":"start",
    "forced":false,"by":"person","at":3}}
]"#;

fn added(operation: &str) -> Line {
    Line::Added(Added {
        operation: operation.to_owned(),
        name: "fixture-seat".to_owned(),
        agent: "agent".to_owned(),
        harness: "claude".to_owned(),
        profile_version: 1,
        machine: "machine".to_owned(),
        working_folder: None,
        account: None,
        by: "person".to_owned(),
        at: 1,
    })
}

fn started(operation: &str, conversation: Option<&str>) -> Line {
    Line::Started(Started {
        qualification: None,
        transport: "pty".to_owned(),
        operation: operation.to_owned(),
        name: "fixture-seat".to_owned(),
        session: operation.to_owned(),
        harness_session: conversation.map(str::to_owned),
        by: "person".to_owned(),
        at: 2,
    })
}

fn stopped(operation: &str, session: &str) -> Line {
    Line::Stopped(Stopped {
        operation: operation.to_owned(),
        name: "fixture-seat".to_owned(),
        session: session.to_owned(),
        forced: false,
        by: "person".to_owned(),
        at: 3,
    })
}

#[test]
fn a_stopped_seat_remembers_its_conversation_after_tail_and_snapshot_reopens() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("seats");
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    store.keep(added("add"))?;
    assert_eq!(store.remembered_conversation("fixture-seat")?, None);
    store.keep(started("start", Some("native-conversation")))?;
    store.keep(stopped("stop", "start"))?;
    let leaves = store.log.len();
    assert_eq!(
        store.remembered_conversation("fixture-seat")?.as_deref(),
        Some("native-conversation")
    );
    assert_eq!(store.log.len(), leaves);
    drop(store);

    let mut reopened = SeatStore::open(&path, Arc::clone(&key))?;
    assert!(matches!(
        reopened.start(),
        Start::Resumed {
            size: 0,
            replayed: 3,
        }
    ));
    assert_eq!(
        reopened.remembered_conversation("fixture-seat")?.as_deref(),
        Some("native-conversation")
    );
    assert!(!reopened.named("fixture-seat")?.running);
    reopened.write_snapshot();
    assert_eq!(reopened.snapshot_failure(), None);
    drop(reopened);

    let reopened = SeatStore::open(&path, key)?;
    assert!(matches!(
        reopened.start(),
        Start::Resumed {
            size: 3,
            replayed: 0,
        }
    ));
    assert_eq!(
        reopened.remembered_conversation("fixture-seat")?.as_deref(),
        Some("native-conversation")
    );
    assert!(!reopened.named("fixture-seat")?.running);
    Ok(())
}

#[test]
fn a_fresh_conversation_replaces_the_remembered_one_without_rewriting_its_history() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("seats");
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    store.keep(added("add"))?;
    let first = started("first-start", Some("first-native-conversation"));
    store.keep(first.clone())?;
    store.keep(stopped("first-stop", "first-start"))?;
    store.keep(started("fresh-start", Some("fresh-native-conversation")))?;
    store.keep(stopped("fresh-stop", "fresh-start"))?;
    drop(store);

    let reopened = SeatStore::open(&path, key)?;
    assert_eq!(
        reopened.remembered_conversation("fixture-seat")?.as_deref(),
        Some("fresh-native-conversation")
    );
    assert_eq!(reopened.recorded("first-start"), Some(&first));
    Ok(())
}

#[test]
fn an_unknown_latest_conversation_never_falls_back_to_an_older_known_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("seats");
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    store.keep(added("add"))?;
    store.keep(started("first-start", Some("first-native-conversation")))?;
    store.keep(stopped("first-stop", "first-start"))?;
    store.keep(started("latest-start", None))?;
    store.keep(stopped("latest-stop", "latest-start"))?;
    assert_eq!(store.remembered_conversation("fixture-seat")?, None);
    drop(store);

    let reopened = SeatStore::open(&path, key)?;
    assert_eq!(reopened.remembered_conversation("fixture-seat")?, None);
    Ok(())
}

#[test]
fn old_encoded_lines_keep_a_known_conversation_through_migration_and_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("seats");
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let leaves: Vec<serde_json::Value> = serde_json::from_str(LEGACY_LINES)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::create(&path, ORIGIN)?)?;
    for leaf in &leaves {
        log.append(&serde_json::to_vec(leaf)?)?;
    }
    drop(log);

    let store = SeatStore::open(&path, Arc::clone(&key))?;
    assert!(matches!(store.start(), Start::Rebuilt { replayed: 3, .. }));
    assert_eq!(
        store.remembered_conversation("fixture-seat")?.as_deref(),
        Some("known-native-conversation")
    );
    assert_eq!(
        store.named("fixture-seat")?.transport.as_deref(),
        Some("managed")
    );
    assert!(!store.named("fixture-seat")?.running);
    assert_eq!(store.snapshot_failure(), None);
    drop(store);

    let reopened = SeatStore::open(&path, key)?;
    assert!(matches!(
        reopened.start(),
        Start::Resumed {
            size: 3,
            replayed: 0,
        }
    ));
    assert_eq!(
        reopened.remembered_conversation("fixture-seat")?.as_deref(),
        Some("known-native-conversation")
    );
    Ok(())
}

#[test]
fn an_old_signed_snapshot_keeps_a_known_conversation_without_rebuilding_its_leaves() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("seats");
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let leaves: Vec<serde_json::Value> = serde_json::from_str(LEGACY_LINES)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::create(&path, ORIGIN)?)?;
    let mut held = Held::default();
    for leaf in &leaves {
        log.append(&serde_json::to_vec(leaf)?)?;
        held.hold(serde_json::from_value(leaf.clone())?)?;
    }
    let mut snapshot: serde_json::Value = serde_json::from_slice(&held.encode()?)?;
    let old_seat = snapshot["held"]["seats"][0]
        .as_object_mut()
        .ok_or("snapshot seat missing")?;
    old_seat.remove("qualification");
    old_seat.remove("transport");
    old_seat.remove("retired");
    let old_start = snapshot["held"]["operations"]["start"]["started"]
        .as_object_mut()
        .ok_or("snapshot start missing")?;
    old_start.remove("qualification");
    old_start.remove("transport");
    log.write_snapshot(DOMAIN, &serde_json::to_vec(&snapshot)?, &key)?;
    drop(log);

    let store = SeatStore::open(&path, key)?;
    assert!(matches!(
        store.start(),
        Start::Resumed {
            size: 3,
            replayed: 0,
        }
    ));
    assert_eq!(
        store.remembered_conversation("fixture-seat")?.as_deref(),
        Some("known-native-conversation")
    );
    assert_eq!(
        store.named("fixture-seat")?.transport.as_deref(),
        Some("managed")
    );
    assert!(!store.named("fixture-seat")?.running);
    Ok(())
}

#[test]
fn a_reused_seat_name_never_remembers_its_retired_incarnations_conversation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("seats");
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    store.keep(added("first-add"))?;
    store.keep(started("start", Some("retired-native-conversation")))?;
    store.keep(stopped("stop", "start"))?;
    store.keep(Line::Retired(Retired {
        operation: "retire".to_owned(),
        name: "fixture-seat".to_owned(),
        by: "person".to_owned(),
        at: 4,
    }))?;
    assert!(matches!(
        store.remembered_conversation("fixture-seat"),
        Err(ServerError::Agents(AgentsError::Seat(
            SeatError::Unknown { .. }
        )))
    ));
    store.keep(added("second-add"))?;
    assert_eq!(store.remembered_conversation("fixture-seat")?, None);
    drop(store);

    let reopened = SeatStore::open(&path, key)?;
    assert_eq!(reopened.remembered_conversation("fixture-seat")?, None);
    Ok(())
}
