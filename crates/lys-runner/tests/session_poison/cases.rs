#![cfg(test)]
//! A failed table mutation invalidates every reader and wakes refused waits.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use super::Sessions;

fn poison(sessions: &Arc<Sessions>) {
    let sessions = Arc::clone(sessions);
    let panicked = std::thread::spawn(move || {
        let mut table = match sessions.table.lock() {
            Ok(table) => table,
            Err(error) => panic!("the table was already poisoned: {error}"),
        };
        table.starting.insert("unfinished".to_owned());
        panic!("a table update failed before completion");
    })
    .join();
    assert!(panicked.is_err());
}

#[test]
fn a_poisoned_table_refuses_session_reads_and_changes() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    poison(&sessions);
    let left = AtomicBool::new(false);
    let errors = [
        sessions.status(None).err(),
        sessions.write("unfinished", b"input").err(),
        sessions.resize("unfinished", 80, 24).err(),
        sessions.end("unfinished", &left).err(),
        sessions.wake_session("unfinished").err(),
        sessions.stop_all().err(),
        sessions.until_any(&left, |_| Some(())).err(),
    ];
    for error in errors {
        assert_eq!(
            error.ok_or("a poisoned table was used")?.name(),
            "session_table_poisoned"
        );
    }
    assert!(sessions.table.is_poisoned());
    Ok(())
}

#[test]
fn a_table_poisoned_while_waiting_names_the_failed_table() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let left = Arc::new(AtomicBool::new(false));
    let (ready, waiting) = mpsc::channel();
    let held = Arc::clone(&sessions);
    let left_waiting = Arc::clone(&left);
    let waiter = std::thread::spawn(move || {
        held.until_any(&left_waiting, |_| {
            assert!(ready.send(()).is_ok(), "the wait observer closed");
            None::<()>
        })
    });
    waiting.recv()?;
    poison(&sessions);
    left.store(true, Ordering::SeqCst);
    sessions.wake();
    let result = waiter.join().expect("the waiter panicked");
    assert_eq!(
        result.err().ok_or("poisoned wait succeeded")?.name(),
        "session_table_poisoned"
    );
    Ok(())
}
