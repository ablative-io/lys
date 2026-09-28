#![cfg(test)]
//! Gates on the sessions a runtime reported, as they are answered: a listing
//! of five hundred sessions takes the machines' lock once, counted rather
//! than timed, and a report's answer takes it once; each session is answered
//! with its machine's name and runtime exactly as before.

use std::error::Error;

use serde_json::{Value, json};

use super::{listing, reported};
use crate::network_store::{Machine, NetworkLock, NetworkStore};
use crate::runtime_state::{Held, Report, Reported};

type TestResult = Result<(), Box<dyn Error>>;

/// How many sessions the listing answers.
const LISTED: u64 = 500;

const MACHINE: &str = "op-00000000000000000000000000000001";
const ELSEWHERE: &str = "op-00000000000000000000000000000002";

fn machine() -> Machine {
    Machine {
        id: MACHINE.to_owned(),
        name: "Laptop 2".to_owned(),
        kind: "laptop".to_owned(),
        runtime: Some("local launcher".to_owned()),
        slots: 2,
        may_run: Vec::new(),
        may_run_roles: Vec::new(),
        may_reach: Vec::new(),
        named_by: "person-a".to_owned(),
        named_at: 1,
        retired: None,
    }
}

fn report(n: u64, machine: &str, state: Reported) -> Report {
    Report {
        operation: format!("op-{n}-{}", state.name()),
        session: format!("s-{n}"),
        agent: Some("agent-a".to_owned()),
        machine: machine.to_owned(),
        state,
        what: if state == Reported::Running {
            "working".to_owned()
        } else {
            String::new()
        },
        confirmation: String::new(),
        reported_by: "agent-a".to_owned(),
        at: n,
        launch: None,
    }
}

#[test]
fn a_listing_of_five_hundred_sessions_takes_the_machines_lock_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = NetworkStore::open(&dir.path().join("network.json"))?;
    store.name(machine())?;
    let network = NetworkLock::new(store);
    let mut held = Held::default();
    for n in 0..LISTED {
        held.hold(report(n, MACHINE, Reported::Starting))?;
    }
    held.hold(report(0, MACHINE, Reported::Running))?;
    held.hold(report(LISTED, ELSEWHERE, Reported::Starting))?;

    let listed = listing(Some(&network), held.sessions(), |_| true);
    assert_eq!(network.taken(), 1, "one lock for the whole listing");
    assert_eq!(listed.sessions.len(), 501);
    let placed = listed
        .sessions
        .iter()
        .filter(|view| view.machine_name.as_deref() == Some("Laptop 2"))
        .count();
    assert_eq!(placed, 500);
    assert_eq!(
        serde_json::to_value(&listed.sessions[0])?,
        json!({
            "session": "s-0", "agent": "agent-a", "machine": MACHINE,
            "machine_name": "Laptop 2", "runtime": "local launcher", "shown": "running",
            "last_reported": "running", "first_report_at": 0, "last_report_at": 0,
            "what": "working", "stopped": null, "stop_asked_at": null,
            "reported_by": "agent-a",
        })
    );
    let unnamed = serde_json::to_value(&listed.sessions[500])?;
    assert_eq!(unnamed["machine_name"], Value::Null);
    assert_eq!(unnamed["runtime"], Value::Null);
    assert_eq!(unnamed["shown"], "unconfirmed");

    let unkept = listing(None, held.sessions(), |_| true);
    assert!(
        unkept
            .sessions
            .iter()
            .all(|view| view.machine_name.is_none())
    );
    assert_eq!(network.taken(), 1, "no machines kept, no lock");

    let first = held.session("s-1").ok_or("s-1 is kept")?;
    let answer = reported(Some(&network), first)?;
    assert_eq!(network.taken(), 2, "one lock for a report's answer");
    assert_eq!(answer.machine_name.as_deref(), Some("Laptop 2"));
    assert_eq!(answer.runtime.as_deref(), Some("local launcher"));
    assert_eq!(answer.shown, "unconfirmed");
    Ok(())
}
