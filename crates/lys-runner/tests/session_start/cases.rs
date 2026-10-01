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
    sessions.stop_all()?;
    assert!(
        available.load(Ordering::SeqCst),
        "process spawn held the session table"
    );
    Ok(())
}
