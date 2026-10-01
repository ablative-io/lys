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
    let bytes = vec![b'x'; 16_384];
    LOCKS.set(0);
    WAKES.set(0);
    COUNTING.set(true);
    sessions.pump("output", generation, Box::new(Cursor::new(bytes.clone())));
    COUNTING.set(false);
    let locks = LOCKS.get();
    let wakes = WAKES.get();
    let kept = sessions
        .lock()
        .sessions
        .get("output")
        .ok_or("session missing")?
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
