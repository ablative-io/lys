#![cfg(test)]
//! Gates on the runtime reports' indexes: one report on a store keeping ten
//! thousand sessions visits the one session it is about, counted rather than
//! timed; a first report visits none, and the same report sent again visits
//! the one it was kept on.

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;

use super::RuntimeStore;
use crate::error::ServerError;
use crate::runtime_state::{Held, Report, Reported};

type TestResult = Result<(), Box<dyn Error>>;

/// How many sessions the store keeps before the report.
const KEPT: usize = 10_000;

const MACHINE: &str = "op-00000000000000000000000000000001";

fn report(operation: &str, session: &str, state: Reported) -> Report {
    Report {
        operation: operation.to_owned(),
        session: session.to_owned(),
        agent: Some("agent-a".to_owned()),
        machine: MACHINE.to_owned(),
        state,
        what: String::new(),
        confirmation: String::new(),
        reported_by: "agent-a".to_owned(),
        at: 5,
        launch: None,
    }
}

#[test]
fn one_report_among_ten_thousand_sessions_visits_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("runtime.key"))?;
    let mut store = RuntimeStore::open(&dir.path().join("runtime"), Arc::new(key))?;
    let mut held = Held::default();
    for n in 0..KEPT {
        held.hold(report(
            &format!("op-start-{n}"),
            &format!("s-{n}"),
            Reported::Starting,
        ))?;
    }
    store.held = held;
    assert_eq!(store.sessions().len(), KEPT);

    let last = format!("s-{}", KEPT - 1);
    let before = store.visited();
    let running = store.report(report("op-run", &last, Reported::Running))?;
    assert_eq!(running.shown(), "running");
    assert_eq!(running.reports.len(), 2);
    assert_eq!(store.visited() - before, 1, "one report visits one session");

    let before = store.visited();
    let again = store.report(report("op-run", &last, Reported::Running))?;
    assert_eq!(again.reports.len(), 2, "the same report is kept once");
    assert_eq!(
        store.visited() - before,
        1,
        "a report sent again visits one"
    );

    let before = store.visited();
    let reused = store.report(report("op-run", "s-0", Reported::Running));
    assert!(matches!(
        reused,
        Err(ServerError::RuntimeReportReused { .. })
    ));
    assert_eq!(store.visited() - before, 1, "a reused operation visits one");

    let before = store.visited();
    let begun = store.report(report("op-new", "s-new", Reported::Starting))?;
    assert_eq!(begun.shown(), "unconfirmed");
    assert_eq!(store.visited() - before, 0, "a first report visits none");
    assert_eq!(store.sessions().len(), KEPT + 1);
    Ok(())
}
