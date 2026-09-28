#![cfg(test)]
//! A sessions listing reads the machines once, whatever it lists, and shows
//! each session as it was shown when each session read them for itself.

use std::collections::HashMap;

use serde_json::json;

use super::{Placements, listing};
use crate::network_store::Placement;
use crate::runtime_state::{Held, Report, Reported};

const MACHINE: &str = "op-00000000000000000000000000000001";

/// The report under operation `n` on session `session`, received at `n`.
fn report(n: u64, session: u64, state: Reported) -> Report {
    Report {
        operation: format!("op-{n:032x}"),
        session: format!("op-{:032x}", 1_000_000 + session),
        agent: Some("agent-a".to_owned()),
        machine: MACHINE.to_owned(),
        state,
        what: if state == Reported::Running {
            "up".to_owned()
        } else {
            String::new()
        },
        confirmation: if state == Reported::Stopped {
            "exited 0".to_owned()
        } else {
            String::new()
        },
        reported_by: "agent-a".to_owned(),
        at: n,
        launch: None,
    }
}

fn placements() -> Placements {
    let placement = Placement {
        name: "the build box".to_owned(),
        runtime: Some("pi".to_owned()),
    };
    HashMap::from([(MACHINE.to_owned(), placement)])
}

#[test]
fn a_listing_of_five_hundred_sessions_reads_the_machines_once() -> Result<(), String> {
    let mut held = Held::default();
    for n in 0..500 {
        held.hold(report(n, n, Reported::Starting))?;
    }
    let mut reads = 0;
    let listed = listing(held.sessions.iter(), || {
        reads += 1;
        placements()
    });
    assert_eq!(reads, 1, "the machines are read once per listing");
    assert_eq!(listed.sessions.len(), 500);
    for shown in &listed.sessions {
        assert_eq!(shown.machine_name.as_deref(), Some("the build box"));
        assert_eq!(shown.runtime.as_deref(), Some("pi"));
    }
    Ok(())
}

#[test]
fn a_session_is_shown_as_it_was_shown_before() -> Result<(), String> {
    let mut held = Held::default();
    held.hold(report(5, 7, Reported::Starting))?;
    held.hold(report(6, 7, Reported::Running))?;
    held.hold(report(9, 7, Reported::Stopped))?;
    let shown = listing(held.sessions.iter(), placements);
    let shown = serde_json::to_value(&shown).map_err(|error| error.to_string())?;
    assert_eq!(
        shown,
        json!({ "sessions": [{
            "session": format!("op-{:032x}", 1_000_007),
            "agent": "agent-a",
            "machine": MACHINE,
            "machine_name": "the build box",
            "runtime": "pi",
            "shown": "stopped",
            "last_reported": "stopped",
            "first_report_at": 5,
            "last_report_at": 9,
            "what": "up",
            "stopped": { "at": 9, "confirmation": "exited 0" },
            "stop_asked_at": null,
            "reported_by": "agent-a",
        }]})
    );
    let unplaced = listing(held.sessions.iter(), HashMap::new);
    let unplaced = serde_json::to_value(&unplaced).map_err(|error| error.to_string())?;
    assert_eq!(unplaced["sessions"][0]["machine_name"], json!(null));
    Ok(())
}
