//! Latest machine times are shared and track only each session's final report.

use std::error::Error;
use std::sync::Arc;

use super::{Held, Report, Reported};

fn report(operation: &str, session: &str, machine: &str, at: u64) -> Report {
    Report {
        operation: operation.to_owned(),
        session: session.to_owned(),
        agent: None,
        machine: machine.to_owned(),
        state: Reported::Running,
        what: String::new(),
        confirmation: String::new(),
        reported_by: "person".to_owned(),
        at,
        launch: None,
    }
}

#[test]
fn latest_machine_reports_share_the_index_across_reads_and_restore() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    for n in 0..512 {
        held.hold(report(
            &format!("operation-{n}"),
            &format!("session-{n}"),
            "machine",
            n,
        ))?;
    }
    let first = held.last_reports();
    assert_eq!(first.get("machine"), Some(&511));
    assert!(Arc::ptr_eq(&first, &held.last_reports()));
    let mut restored = Held::decode(&held.encode()?)?;
    let before = restored.last_reports();
    assert_eq!(*before, *first);
    assert!(Arc::ptr_eq(&before, &restored.last_reports()));
    restored.hold(report("new-final", "session-511", "other-machine", 0))?;
    assert_eq!(restored.last_reports().get("machine"), Some(&510));
    assert!(!restored.last_reports().contains_key("other-machine"));
    assert_eq!(before.get("machine"), Some(&511));
    restored.hold(report("new-session", "new-session", "other-machine", 600))?;
    assert_eq!(restored.last_reports().get("other-machine"), Some(&600));
    let reloaded = Held::decode(&restored.encode()?)?;
    assert_eq!(*reloaded.last_reports(), *restored.last_reports());
    Ok(())
}

#[test]
fn equal_report_times_and_non_monotonic_updates_keep_the_correct_maximum()
-> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(report("one", "one", "machine", 10))?;
    held.hold(report("two", "two", "machine", 10))?;
    held.hold(report("one-lower", "one", "machine", 1))?;
    assert_eq!(held.last_reports().get("machine"), Some(&10));
    held.hold(report("two-lower", "two", "machine", 2))?;
    assert_eq!(held.last_reports().get("machine"), Some(&2));
    held.hold(report("one-higher", "one", "machine", 30))?;
    assert_eq!(held.last_reports().get("machine"), Some(&30));
    Ok(())
}
