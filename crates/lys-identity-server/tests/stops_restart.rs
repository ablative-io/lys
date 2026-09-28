//! The emergency stops start from their signed snapshot and read only the
//! leaves after it; what they fold to is the same across a restart; the
//! same stop sent again answers what was kept; the same operation in other
//! words is refused by name.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::error::ServerError;
use lys_identity_server::stop_api::StopView;
use lys_identity_server::stops_state::Stop;
use lys_identity_server::stops_store::StopStore;
use lys_log_store::Start;

type TestResult = Result<(), Box<dyn Error>>;

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("stops.key"),
    )?))
}

fn stop(operation: &str, reason: &str, at: u64) -> Stop {
    Stop {
        operation: operation.to_owned(),
        agent: "agent-00000000000000000000000000000002".to_owned(),
        by: "person-00000000000000000000000000000001".to_owned(),
        reason: reason.to_owned(),
        at,
        certificates_withdrawn: vec!["op-1".to_owned()],
        sessions_asked: vec!["op-9".to_owned()],
        credentials_ended: None,
        credentials_refused: Some("SecretsUnavailable: not configured".to_owned()),
        done: true,
    }
}

#[test]
fn a_stop_asked_binds_its_words_before_it_is_done() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stops");
    let mut store = StopStore::open(&path, key(dir.path())?)?;
    assert_eq!(store.ask(stop("op-1", "leaked its key", 5))?, None);
    assert!(matches!(
        store.ask(stop("op-1", "other words", 6)),
        Err(ServerError::StopReused { .. })
    ));
    assert!(matches!(
        store.keep(stop("op-1", "other words", 6)),
        Err(ServerError::StopReused { .. })
    ));
    assert_eq!(store.ask(stop("op-1", "leaked its key", 7))?, None);
    drop(store);

    let mut store = StopStore::open(&path, key(dir.path())?)?;
    let asked = store.recorded("op-1").cloned().ok_or("op-1 not held")?;
    assert!(!asked.done);
    let shown = StopView::from(asked);
    assert_eq!(shown.state, "asked");
    assert!(!shown.done);
    let done = store.keep(stop("op-1", "leaked its key", 8))?;
    assert!(done.done);
    assert_eq!(StopView::from(done.clone()).state, "suspended");
    assert_eq!(
        store.ask(stop("op-1", "leaked its key", 9))?,
        Some(done.clone())
    );
    assert_eq!(store.keep(stop("op-1", "leaked its key", 9))?, done);
    Ok(())
}

#[test]
fn stops_fold_the_same_across_a_restart_from_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stops");
    let store = StopStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = StopStore::open(&path, key(dir.path())?)?;
    store.keep(stop("op-1", "leaked its key", 5))?;
    store.keep(stop("op-2", "ran outside its machine", 6))?;
    drop(store);

    let store = StopStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.recorded("op-1"),
        Some(&stop("op-1", "leaked its key", 5))
    );
    assert_eq!(
        store.recorded("op-2"),
        Some(&stop("op-2", "ran outside its machine", 6))
    );
    Ok(())
}

#[test]
fn the_same_stop_again_answers_what_was_kept_and_other_words_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = StopStore::open(&dir.path().join("stops"), key(dir.path())?)?;
    let kept = store.keep(stop("op-1", "leaked its key", 5))?;
    let mut later = stop("op-1", "leaked its key", 9);
    later.sessions_asked.clear();
    assert_eq!(
        store.keep(later)?,
        kept,
        "the kept stop answers, not the retry"
    );
    assert!(matches!(
        store.keep(stop("op-1", "other words", 9)),
        Err(ServerError::StopReused { .. })
    ));
    Ok(())
}
