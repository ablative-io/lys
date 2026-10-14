//! Seat transitions enter storage whole and repeated operations append nothing.

use std::error::Error;
use std::sync::Arc;

use identity_contract::harness::{Fault, Harness};
use lys_core::Ed25519Identity;
use lys_log_store::{Start, flush_count};

use super::SeatStore;
use crate::seats_state::{Added, Line, Moved, Started, Stopped};

type TestResult = Result<(), Box<dyn Error>>;
const NAME: &str = "fixture-seat";

fn added(operation: &str, name: &str) -> Line {
    Line::Added(Added {
        operation: operation.to_owned(),
        name: name.to_owned(),
        agent: "agent".to_owned(),
        harness: "claude-code".to_owned(),
        profile_version: 1,
        machine: "machine".to_owned(),
        working_folder: Some("/project".to_owned()),
        account: None,
        by: "person".to_owned(),
        at: 1,
    })
}

fn started(operation: &str, session: &str, native: Option<&str>) -> Line {
    Line::Started(Started {
        qualification: None,
        transport: "pty".to_owned(),
        operation: operation.to_owned(),
        name: NAME.to_owned(),
        session: session.to_owned(),
        harness_session: native.map(str::to_owned),
        by: "person".to_owned(),
        at: 2,
    })
}

fn stopped(operation: &str, session: &str) -> Line {
    Line::Stopped(Stopped {
        operation: operation.to_owned(),
        name: NAME.to_owned(),
        session: session.to_owned(),
        forced: false,
        by: "person".to_owned(),
        at: 3,
    })
}

fn moved(operation: &str, name: &str) -> Line {
    Line::Moved(Moved {
        operation: operation.to_owned(),
        name: name.to_owned(),
        profile_version: 2,
        harness: "codex".to_owned(),
        by: "person".to_owned(),
        at: 4,
    })
}

fn seed<S: lys_log_store::LeafStore>(store: &mut SeatStore<S>, operation: &str) -> TestResult {
    store.keep(added("add", NAME))?;
    store.keep(started(operation, operation, None))?;
    Ok(())
}

#[test]
fn late_native_identity_and_stop_land_with_one_flush_and_survive_tail_and_snapshot_reopens()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("seats");
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    seed(&mut store, "start")?;
    let lines = vec![
        started("late-native", "start", Some("native-conversation")),
        stopped("stop", "start"),
    ];
    let before = flush_count();
    assert_eq!(store.keep_all(lines.clone())?, lines);
    assert_eq!(flush_count() - before, 1);
    assert_eq!(store.log.len(), 4);
    let seat = store.named(NAME)?;
    assert!(!seat.running);
    assert_eq!(seat.session.as_deref(), Some("start"));
    assert_eq!(seat.harness_session.as_deref(), Some("native-conversation"));
    assert_eq!(seat.revision, 4);
    drop(store);

    let mut reopened = SeatStore::open(&path, Arc::clone(&key))?;
    assert!(matches!(
        reopened.start(),
        Start::Resumed {
            size: 0,
            replayed: 4
        }
    ));
    assert_eq!(reopened.named(NAME)?, seat);
    for line in &lines {
        assert_eq!(reopened.recorded(line.operation()), Some(line));
    }
    reopened.write_snapshot();
    assert_eq!(reopened.snapshot_failure(), None);
    drop(reopened);
    let reopened = SeatStore::open(&path, key)?;
    assert!(matches!(
        reopened.start(),
        Start::Resumed {
            size: 4,
            replayed: 0
        }
    ));
    assert_eq!(reopened.named(NAME)?, seat);
    Ok(())
}

#[test]
fn a_fresh_profile_transition_folds_stop_move_and_new_start_before_its_one_flush() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("seats");
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    seed(&mut store, "old")?;
    let lines = vec![
        stopped("stop-old", "old"),
        moved("move-profile", NAME),
        started("start-fresh", "fresh", Some("fresh-native")),
    ];
    let before = flush_count();
    assert_eq!(store.keep_all(lines.clone())?, lines);
    assert_eq!(flush_count() - before, 1);
    let seat = store.named(NAME)?;
    assert!(seat.running);
    assert_eq!(seat.added.profile_version, 2);
    assert_eq!(seat.added.harness, "codex");
    assert_eq!(seat.session.as_deref(), Some("fresh"));
    assert_eq!(seat.harness_session.as_deref(), Some("fresh-native"));
    assert_eq!(seat.revision, 5);
    let before = flush_count();
    assert_eq!(store.keep_all(lines.clone())?, lines);
    assert_eq!(flush_count(), before);
    assert_eq!(store.log.len(), 5);
    drop(store);
    let mut reopened = SeatStore::open(&path, key)?;
    assert_eq!(reopened.named(NAME)?, seat);
    let before = flush_count();
    assert_eq!(reopened.keep_all(lines.clone())?, lines);
    assert_eq!(flush_count(), before);
    Ok(())
}

#[test]
fn duplicate_and_previously_retained_operations_answer_the_original_lines_without_extra_leaves()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::open(&directory.path().join("seats"), key)?;
    seed(&mut store, "start")?;
    let late = started("late", "start", Some("native"));
    let mut duplicate = late.clone();
    let Line::Started(ref mut asked) = duplicate else {
        return Err("the duplicate fixture is not a start".into());
    };
    asked.by = "another-person".to_owned();
    asked.at = 99;
    let stop = stopped("stop", "start");
    let before = flush_count();
    assert_eq!(
        store.keep_all(vec![
            added("add", NAME),
            late.clone(),
            duplicate,
            stop.clone()
        ])?,
        vec![added("add", NAME), late.clone(), late, stop]
    );
    assert_eq!(flush_count() - before, 1);
    assert_eq!(store.log.len(), 4);
    let before = flush_count();
    assert!(store.keep_all(Vec::new())?.is_empty());
    assert_eq!(flush_count(), before);
    Ok(())
}

#[test]
fn named_rejections_in_a_later_batch_line_leave_the_original_projection_and_log_unchanged()
-> TestResult {
    let directory = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::open(&directory.path().join("seats"), key)?;
    seed(&mut store, "start")?;
    let late = started("late", "start", Some("native"));
    let before = store.held.clone();
    for (bad, name) in [
        (added("add", "other-seat"), "seat_operation_reused"),
        (added("taken", NAME), "seat_name_taken"),
        (moved("unknown", "missing-seat"), "seat_unknown"),
        (moved("running", NAME), "seat_running"),
    ] {
        let flushes = flush_count();
        let error = store
            .keep_all(vec![late.clone(), bad])
            .err()
            .ok_or("an invalid later line committed its earlier staged line")?;
        assert_eq!(error.name(), name);
        assert_eq!(flush_count(), flushes);
        assert_eq!(store.held, before);
        assert_eq!(store.log.len(), 2);
        assert!(store.recorded("late").is_none());
    }
    Ok(())
}

#[test]
fn an_inconsistent_session_is_refused_during_staging_before_any_batch_leaf_is_written() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("seats");
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::open(&path, Arc::clone(&key))?;
    seed(&mut store, "start")?;
    let seat = store.named(NAME)?;
    let before = flush_count();
    let error = store
        .keep_all(vec![
            started("late", "start", Some("native")),
            stopped("wrong-stop", "another-session"),
        ])
        .err()
        .ok_or("an inconsistent session was written into the seat log")?;
    assert_eq!(error.name(), "seats_unavailable");
    assert!(error.to_string().contains("does not hold session"));
    assert_eq!(flush_count(), before);
    assert_eq!(store.log.len(), 2);
    assert_eq!(store.named(NAME)?, seat);
    drop(store);
    assert_eq!(SeatStore::open(&path, key)?.named(NAME)?, seat);
    Ok(())
}

#[test]
fn an_uncertain_batch_is_read_back_whole_and_a_refused_write_leaves_no_transition() -> TestResult {
    let harness = Harness::new(47)?;
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::over(harness.leaves(), Arc::clone(&key))?;
    store.keep(added("add", NAME))?;
    for (number, fault) in [
        Fault::BeforeLeaf,
        Fault::LeafStoredWriteFailed,
        Fault::AfterLeaf,
    ]
    .into_iter()
    .enumerate()
    {
        let session = format!("start-{number}");
        store.keep(started(&session, &session, None))?;
        let seat = store.named(NAME)?;
        let index = store.log.len();
        let late = format!("late-{number}");
        let stop = format!("stop-{number}");
        let lines = vec![
            started(&late, &session, Some("native")),
            stopped(&stop, &session),
        ];
        harness.fail(fault)?;
        let answer = store.keep_all(lines.clone());
        if fault == Fault::BeforeLeaf {
            let error = answer.err().ok_or("a refused batch was acknowledged")?;
            assert_eq!(error.name(), "seats_unavailable");
            assert_eq!(store.log.len(), index);
            assert_eq!(store.named(NAME)?, seat);
            assert!(store.recorded(&late).is_none());
            assert!(store.recorded(&stop).is_none());
            assert_eq!(store.keep_all(lines.clone())?, lines);
        } else {
            assert_eq!(answer?, lines);
        }
        assert_eq!(store.log.len(), index + 2);
        let held = store.named(NAME)?;
        assert!(!held.running);
        assert_eq!(held.harness_session.as_deref(), Some("native"));
        let before = flush_count();
        assert_eq!(store.keep_all(lines.clone())?, lines);
        assert_eq!(flush_count(), before);
    }
    drop(store);
    let reopened = SeatStore::over(harness.leaves(), key)?;
    assert!(!reopened.named(NAME)?.running);
    assert_eq!(
        reopened.remembered_conversation(NAME)?.as_deref(),
        Some("native")
    );
    Ok(())
}

#[test]
fn an_unreadable_uncertain_batch_blocks_later_writes_until_its_whole_transition_is_settled()
-> TestResult {
    let harness = Harness::new(48)?;
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = SeatStore::over(harness.leaves(), key)?;
    seed(&mut store, "start")?;
    let lines = vec![
        started("late", "start", Some("native")),
        stopped("stop", "start"),
    ];
    harness.fail(Fault::AfterLeafUnreadable)?;
    let error = store
        .keep_all(lines.clone())
        .err()
        .ok_or("an unreadable append was acknowledged without read-back")?;
    assert_eq!(error.name(), "seats_unavailable");
    assert!(store.uncertain);
    let before = flush_count();
    assert!(
        store
            .keep_all(vec![started("later", "later", None)])
            .is_err()
    );
    assert_eq!(flush_count(), before);
    harness.fail(Fault::None)?;
    store.settle()?;
    assert!(!store.uncertain);
    assert!(!store.named(NAME)?.running);
    assert_eq!(store.log.len(), 4);
    assert!(store.recorded("later").is_none());
    let before = flush_count();
    assert_eq!(store.keep_all(lines.clone())?, lines);
    assert_eq!(flush_count(), before);
    Ok(())
}
