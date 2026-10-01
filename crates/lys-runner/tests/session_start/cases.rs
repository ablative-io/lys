#![cfg(test)]
//! A process spawn cannot own the shared session table.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::Sessions;
use crate::Launch;

#[test]
fn the_real_spawn_boundary_holds_no_session_table_lock() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let available = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&available);
    let probed = Arc::clone(&sessions);
    *sessions
        .spawn_probe
        .lock()
        .map_err(|error| error.to_string())? = Some(Box::new(move || {
        observed.store(probed.table.try_lock().is_ok(), Ordering::SeqCst);
    }));
    sessions.start(Launch {
        session: "session".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    sessions.stop_all();
    assert!(
        available.load(Ordering::SeqCst),
        "process spawn held the session table"
    );
    Ok(())
}

#[test]
fn launches_in_progress_count_toward_the_session_capacity() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    {
        let mut table = sessions.lock();
        for slot in 0..16 {
            table.starting.insert(format!("starting_{slot}"));
        }
    }
    let started = sessions.start(Launch {
        session: "overflow".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    });
    sessions.lock().starting.clear();
    sessions.stop_all();
    assert_eq!(
        started.err().map(|error| error.name()).as_deref(),
        Some("runner_sessions_full")
    );
    Ok(())
}

#[test]
fn a_replacement_uses_its_existing_session_slot() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let mut table = sessions.lock();
    for slot in 0..16 {
        table.running.insert(format!("running_{slot}"));
    }
    assert!(table.reserve_capacity("running_0").is_ok());
    assert_eq!(
        table
            .reserve_capacity("new_session")
            .err()
            .map(|error| error.name())
            .as_deref(),
        Some("runner_sessions_full")
    );
    Ok(())
}

#[test]
fn a_proved_restart_holds_capacity_until_its_replacement_finishes() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    {
        let mut table = sessions.lock();
        for slot in 0..15 {
            table.running.insert(format!("running_{slot}"));
        }
        table.restarting.insert("restarting".to_owned());
        assert!(table.reserve_capacity("restarting").is_ok());
        assert_eq!(
            table
                .reserve_capacity("new_session")
                .err()
                .map(|error| error.name())
                .as_deref(),
            Some("runner_sessions_full")
        );
    }
    let reservation = super::Restarting {
        sessions: Arc::clone(&sessions),
        id: "restarting".to_owned(),
    };
    drop(reservation);
    assert!(sessions.lock().reserve_capacity("new_session").is_ok());
    Ok(())
}

#[test]
fn occupied_generation_workers_refuse_before_another_child_is_spawned() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let permit = Arc::clone(&sessions.generations).try_acquire_many_owned(32)?;
    let started = sessions.start(Launch {
        session: "overflow".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    });
    assert_eq!(
        started.err().map(|error| error.name()).as_deref(),
        Some("runner_lifecycle_workers_full")
    );
    assert!(sessions.lock().starting.is_empty());
    assert!(sessions.lock().sessions.is_empty());
    drop(permit);
    Ok(())
}

#[test]
fn a_full_follower_pool_refuses_the_selected_binding() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(Launch {
        session: "followed".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    let permit = Arc::clone(&sessions.followers).try_acquire_many_owned(32)?;
    let result = {
        let mut table = sessions.lock();
        table.feed.append(
            "followed",
            super::now_ms(),
            Vec::new(),
            crate::tracking_store::Commit {
                source: Some(crate::tracking_store::SourceState {
                    path: dir
                        .path()
                        .join("transcript.jsonl")
                        .to_string_lossy()
                        .into_owned(),
                    ..crate::tracking_store::SourceState::default()
                }),
                attempt: None,
            },
        )?;
        sessions.follow(&mut table, "followed")
    };
    drop(permit);
    sessions.stop_all();
    assert_eq!(
        result.err().map(|error| error.name()).as_deref(),
        Some("runner_followers_full")
    );
    Ok(())
}

#[test]
fn a_full_input_worker_pool_refuses_before_input_is_queued() {
    let workers = Arc::new(tokio::sync::Semaphore::new(0));
    let input = crate::input::Input::bounded(Box::new(Vec::<u8>::new()), workers);
    let completed = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&completed);
    let result = input.submit(b"input".to_vec(), move |_| {
        observed.store(true, Ordering::SeqCst);
    });
    assert_eq!(
        result.err().map(|error| error.name()).as_deref(),
        Some("runner_input_workers_full")
    );
    assert!(!completed.load(Ordering::SeqCst));
}
