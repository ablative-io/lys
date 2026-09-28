//! The runtime reports start from their signed snapshot and read only the
//! leaves after it; what they fold to is the same across a restart; a
//! session is stopped only with its runtime's confirmation, and one whose
//! runtime reported nothing since it started stays unconfirmed.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::error::ServerError;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;
use lys_log_store::Start;

type TestResult = Result<(), Box<dyn Error>>;

const AGENT: &str = "agent-a";
const MACHINE: &str = "op-00000000000000000000000000000001";

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("runtime.key"),
    )?))
}

fn report(operation: &str, session: &str, agent: Option<&str>, state: Reported) -> Report {
    Report {
        operation: operation.to_owned(),
        session: session.to_owned(),
        agent: agent.map(str::to_owned),
        machine: MACHINE.to_owned(),
        state,
        what: String::new(),
        confirmation: if state == Reported::Stopped {
            "exited 0".to_owned()
        } else {
            String::new()
        },
        reported_by: agent.unwrap_or("person-a").to_owned(),
        launch: None,
        at: 5,
    }
}

#[test]
fn reports_fold_the_same_across_a_restart_from_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("runtime");
    let store = RuntimeStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = RuntimeStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 0
        }
    );
    store.report(report("op-1", "s-run", Some(AGENT), Reported::Starting))?;
    store.report(report("op-2", "s-run", Some(AGENT), Reported::Running))?;
    store.report(report("op-3", "s-stop", Some(AGENT), Reported::Starting))?;
    store.report(report("op-4", "s-stop", Some(AGENT), Reported::Stopped))?;
    store.report(report("op-5", "s-quiet", Some(AGENT), Reported::Starting))?;
    store.report(report("op-6", "s-found", None, Reported::Running))?;
    let before = store.sessions().to_vec();
    drop(store);

    let store = RuntimeStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 6
        },
        "a start reads the snapshot and only the leaves after it"
    );
    assert_eq!(store.sessions(), before.as_slice());
    assert_eq!(store.snapshot_failure(), None);
    let shown = |session: &str| {
        store
            .session(session)
            .map(lys_identity_server::runtime_state::Tracked::shown)
    };
    assert_eq!(shown("s-run"), Some("running"));
    assert_eq!(shown("s-stop"), Some("stopped"));
    assert_eq!(shown("s-quiet"), Some("unconfirmed"));
    assert_eq!(shown("s-found"), Some("running"));
    assert_eq!(
        store
            .session("s-found")
            .map(|tracked| tracked.agent.clone()),
        Some(None),
        "a found session is never given an identity"
    );
    Ok(())
}

#[test]
fn a_snapshot_from_another_key_is_refused_and_every_leaf_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("runtime");
    let mut store = RuntimeStore::open(&path, key(dir.path())?)?;
    store.report(report("op-1", "s-1", Some(AGENT), Reported::Starting))?;
    drop(store);

    let other = tempfile::tempdir()?;
    let store = RuntimeStore::open(&path, key(other.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 1, .. }),
        "{}",
        store.start()
    );
    assert_eq!(
        store
            .session("s-1")
            .map(lys_identity_server::runtime_state::Tracked::shown),
        Some("unconfirmed")
    );
    Ok(())
}

#[test]
fn unconfirmed_stays_unconfirmed_and_a_stop_ends_the_session() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = RuntimeStore::open(&dir.path().join("runtime"), key(dir.path())?)?;
    let started = store.report(report("op-1", "s-1", Some(AGENT), Reported::Starting))?;
    assert_eq!(started.shown(), "unconfirmed");
    let again = store.report(report("op-1", "s-1", Some(AGENT), Reported::Starting))?;
    assert_eq!(again.reports.len(), 1, "the same report is kept once");
    assert_eq!(again.shown(), "unconfirmed");

    let restarted = store.report(report("op-2", "s-1", Some(AGENT), Reported::Starting));
    assert!(matches!(
        restarted,
        Err(ServerError::RuntimeSessionStarted { .. })
    ));
    let other_words = store.report(report("op-1", "s-1", Some(AGENT), Reported::Running));
    assert!(matches!(
        other_words,
        Err(ServerError::RuntimeReportReused { .. })
    ));
    let unbegun = store.report(report("op-3", "s-2", Some(AGENT), Reported::Running));
    assert!(matches!(unbegun, Err(ServerError::RuntimeSessionUnknown)));
    let found_starting = store.report(report("op-4", "s-3", None, Reported::Starting));
    assert!(matches!(
        found_starting,
        Err(ServerError::RuntimeSessionUnknown)
    ));
    let by_another = store.report(report("op-5", "s-1", Some("agent-b"), Reported::Running));
    assert!(matches!(
        by_another,
        Err(ServerError::RuntimeSessionUnknown)
    ));
    assert_eq!(
        store
            .session("s-1")
            .map(lys_identity_server::runtime_state::Tracked::shown),
        Some("unconfirmed"),
        "nothing refused moves the session"
    );

    let stopped = store.report(report("op-6", "s-1", Some(AGENT), Reported::Stopped))?;
    assert_eq!(stopped.shown(), "stopped");
    let after = store.report(report("op-7", "s-1", Some(AGENT), Reported::Running));
    assert!(matches!(
        after,
        Err(ServerError::RuntimeSessionStopped { .. })
    ));
    Ok(())
}
