//! Session choice follows an actual start report, never a later usage or status timestamp.

use super::started_at;
use crate::runtime_state::{Report, Reported, Tracked};

fn report(state: Reported, at: u64) -> Report {
    Report {
        operation: format!("report-{at}"),
        session: "session".to_owned(),
        agent: Some("agent".to_owned()),
        machine: "machine".to_owned(),
        state,
        what: String::new(),
        confirmation: String::new(),
        reported_by: "person".to_owned(),
        at,
        launch: None,
    }
}

#[test]
fn later_running_reports_do_not_change_the_sessions_actual_start_order() -> Result<(), String> {
    let tracked = Tracked {
        session: "session".to_owned(),
        agent: Some("agent".to_owned()),
        machine: "machine".to_owned(),
        reports: vec![report(Reported::Starting, 1), report(Reported::Running, 20)],
    };
    assert_eq!(started_at(&tracked)?, 1);
    Ok(())
}

#[test]
fn an_unreported_start_is_named_instead_of_using_a_running_observation_as_a_start() {
    let tracked = Tracked {
        session: "session".to_owned(),
        agent: Some("agent".to_owned()),
        machine: "machine".to_owned(),
        reports: vec![report(Reported::Running, 20)],
    };
    assert_eq!(
        started_at(&tracked).err().as_deref(),
        Some("the start time of session session was not reported")
    );
}
