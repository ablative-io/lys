use std::error::Error;
use std::sync::Arc;

use super::{Held, Report, Reported};
use crate::folded_work::{Work, count, reset};

fn report(operation: &str, session: &str) -> Report {
    Report {
        operation: operation.to_owned(),
        session: session.to_owned(),
        agent: None,
        machine: "machine".to_owned(),
        state: Reported::Running,
        what: String::new(),
        confirmation: String::new(),
        reported_by: "person".to_owned(),
        at: 1,
        launch: None,
    }
}

#[test]
fn operation_lookup_visits_no_runtime_history_after_restore_or_write() -> Result<(), Box<dyn Error>>
{
    let mut held = Held::default();
    for n in 0..512 {
        held.hold(report(&format!("operation-{n}"), "session"))?;
    }
    let bytes = held.encode()?;
    let mut restored = Held::decode(&bytes)?;
    assert_eq!(restored.encode()?, bytes);
    let copied = restored.clone();
    assert!(Arc::ptr_eq(&restored.index, &copied.index));
    restored.hold(report("latest", "session"))?;
    reset();
    assert_eq!(
        restored
            .operation("operation-511")
            .map(|report| report.operation.as_str()),
        Some("operation-511")
    );
    assert_eq!(
        restored
            .operation("latest")
            .map(|report| report.operation.as_str()),
        Some("latest")
    );
    assert!(restored.operation("missing").is_none());
    assert!(copied.operation("latest").is_none());
    assert_eq!(
        count(Work::RuntimeOperation),
        0,
        "operation lookup walked runtime history"
    );
    Ok(())
}

#[test]
fn duplicate_locations_preserve_the_first_session_and_report_order() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(report("first", "one"))?;
    held.hold(report("duplicate", "two"))?;
    held.hold(report("duplicate", "one"))?;
    held.hold(report("duplicate", "one"))?;
    assert_eq!(
        held.operation("duplicate"),
        Some(&held.sessions[0].reports[1])
    );
    let restored = Held::decode(&held.encode()?)?;
    assert_eq!(restored.operation("duplicate"), held.operation("duplicate"));
    Ok(())
}
