use std::cell::Cell;
use std::collections::BTreeMap;
use std::error::Error;
use std::io::Cursor;

use super::Sessions;
use crate::Launch;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static LOCKS: Cell<usize> = const { Cell::new(0) };
    static WAKES: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn table_locked() {
    if COUNTING.get() {
        LOCKS.set(LOCKS.get() + 1);
    }
}

pub(crate) fn table_woken() {
    if COUNTING.get() {
        WAKES.set(WAKES.get() + 1);
    }
}

#[test]
fn terminal_output_never_locks_or_wakes_the_global_table() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 32_768)?;
    sessions.start(Launch {
        session: "output".to_owned(),
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
    let generation = sessions
        .lock()
        .sessions
        .get("output")
        .ok_or("session missing")?
        .generation;
    let output = std::sync::Arc::clone(
        &sessions
            .lock()
            .sessions
            .get("output")
            .ok_or("session missing")?
            .output,
    );
    let bytes = vec![b'x'; 16_384];
    LOCKS.set(0);
    WAKES.set(0);
    COUNTING.set(true);
    sessions.pump(
        "output",
        generation,
        &output,
        Box::new(Cursor::new(bytes.clone())),
    );
    COUNTING.set(false);
    let locks = LOCKS.get();
    let wakes = WAKES.get();
    let kept = sessions
        .lock()
        .sessions
        .get("output")
        .ok_or("session missing")?
        .output
        .lock()?
        .scrollback()
        .from(0)?;
    sessions.stop_all();
    assert_eq!(kept, bytes);
    assert_eq!(
        (locks, wakes),
        (0, 0),
        "ordinary output touched the shared table"
    );
    Ok(())
}

#[test]
fn output_wait_and_cancellation_use_the_selected_handle() -> Result<(), Box<dyn Error>> {
    use super::super::output::OutputHandle;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    let selected = Arc::new(OutputHandle::new(4096, None));
    let other = OutputHandle::new(4096, None);
    selected.begin(1, None, false)?;
    other.begin(1, None, false)?;
    let left = Arc::new(AtomicBool::new(false));
    let (entered, checked) = mpsc::channel();
    let waiting = Arc::clone(&selected);
    let caller = Arc::clone(&left);
    let waiter = std::thread::spawn(move || {
        waiting.until("selected", &caller, |_, _| {
            if let Err(error) = entered.send(()) {
                return Some(Err(crate::RunnerError::refused(
                    "test_signal_failed",
                    error.to_string(),
                )));
            }
            None::<Result<(), crate::RunnerError>>
        })
    });
    checked.recv()?;
    other.push(1, b"independent")?;
    left.store(true, Ordering::SeqCst);
    selected.wake()?;
    let result = waiter
        .join()
        .map_err(|panic| format!("output waiter panicked: {panic:?}"))?;
    assert!(matches!(result, Err(ref error) if error.name() == "caller_left"));
    assert_eq!(other.lock()?.scrollback().from(0)?, b"independent");
    Ok(())
}

#[test]
fn words_cross_chunks_once_and_stale_generations_keep_nothing() -> Result<(), Box<dyn Error>> {
    use super::super::output::OutputHandle;
    use crate::rotation::{Limit, Rotation, RotationState};
    let output = OutputHandle::new(4096, None);
    let rotation = RotationState::new(Rotation {
        accounts: vec!["one".to_owned()],
        variable: "ACCOUNT".to_owned(),
        limit: Limit::Words {
            words: vec!["limit reached".to_owned()],
        },
        resume_arguments: Vec::new(),
    })?;
    output.begin(1, Some(&rotation), false)?;
    assert!(!output.push(1, b"limit ")?);
    assert!(output.push(1, b"reached")?);
    assert!(!output.push(1, b"limit reached")?);
    output.begin(2, Some(&rotation), true)?;
    let cursor = output.lock()?.scrollback().end();
    assert!(!output.push(1, b"stale")?);
    output.finish(
        1,
        crate::protocol::Ended {
            how: crate::protocol::EndedHow::Exited,
            at: 1,
            status: Some(0),
            signal: None,
            reason: None,
        },
    )?;
    assert!(output.lock()?.ended().is_none());
    assert_eq!(output.lock()?.scrollback().end(), cursor);
    assert!(!output.push(2, b"limit reached")?);
    Ok(())
}
