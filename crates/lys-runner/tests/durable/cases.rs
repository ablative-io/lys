#![cfg(test)]
//! A blocked durable append must leave the session table available.

use std::error::Error;
use std::io::Read;
use std::sync::Arc;

use super::Sessions;
use crate::refusals::{REFUSAL_VERSION, RefusalRecord};

#[test]
fn a_blocked_refusal_append_holds_no_session_table_lock() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let path = dir.path().join("feed.jsonl");
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    nix::unistd::mkfifo(
        &path,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )?;
    let record = RefusalRecord {
        version: REFUSAL_VERSION,
        source: "runner".to_owned(),
        attempt: "attempt".to_owned(),
        session: "session".to_owned(),
        agent: "agent".to_owned(),
        at: 1,
        tool: "Read".to_owned(),
        target: "target".to_owned(),
        policy_version: 1,
        rule: None,
        check: "rule_denied".to_owned(),
        grantable: false,
        permission: None,
        grantor: None,
        words: "x".repeat(524_288),
    };
    let caller = Arc::clone(&sessions);
    let joined = std::thread::spawn(move || {
        crate::refusal_log::keep(&caller, "session", "attempt", &record)
    });
    let mut reader = std::fs::File::open(&path)?;
    let mut first = [0; 1];
    reader.read_exact(&mut first)?;
    let available = sessions.table.try_lock().is_ok();
    let mut rest = Vec::new();
    reader.read_to_end(&mut rest)?;
    let verdict = joined
        .join()
        .map_err(|panic| format!("refusal caller panicked: {panic:?}"))?;
    std::fs::remove_file(&path)?;
    assert!(
        available,
        "the refusal's disk append held the session table"
    );
    assert_eq!(verdict.audit, "recorded");
    Ok(())
}
