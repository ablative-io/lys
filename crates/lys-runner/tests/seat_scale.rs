//! AGENTS-004 R7: three times the roster, recovered in one record visit per
//! owner, from `seat_scale_fixture.json`.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use lys_runner::peer;
use lys_runner::seat_owner::recovery::{self, Unreachable};
use lys_runner::seat_owner::sessions::OwnedSeat;
use serde::Deserialize;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    format: String,
    brief: String,
    rule: String,
    ceilings: Ceilings,
    sessions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ceilings {
    record_visits_per_owner: u64,
    proofs_per_owner: u64,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("seat_scale_fixture.json");
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn seat(state: &Path, session: &str, pid: u32, start: &str) -> Result<OwnedSeat, Box<dyn Error>> {
    let dir = state.join("owners").join(session);
    Ok(serde_json::from_value(serde_json::json!({
        "binding": { "seat": format!("seat-{session}"), "session": session, "conversation": format!("c-{session}"), "generation": 1 },
        "endpoint": { "dir": dir, "socket": dir.join("owner.sock"), "runner": format!("owner-{session}"), "owner": { "pid": pid, "start": start } },
        "established_at": 1_760_060_000_000u64
    }))?)
}

/// A pid no process has: a child that has already been waited on.
fn exited_pid() -> Result<u32, Box<dyn Error>> {
    let mut child = Command::new("true").spawn()?;
    let pid = child.id();
    child.wait()?;
    Ok(pid)
}

#[test]
fn seat_scale_fixture_reads() -> TestResult {
    let fixture = fixture()?;
    assert_eq!(fixture.format, "lys-runner-seat-scale-fixture/v1");
    assert_eq!(fixture.brief, "AGENTS-004 R7");
    assert!(fixture.rule.contains("48 owners"));
    assert_eq!(fixture.sessions.len(), 48);
    let mut unique = fixture.sessions;
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 48, "every session is its own owner");
    Ok(())
}

#[test]
fn seat_scale_recovers_three_rosters_in_one_visit_each() -> TestResult {
    let fixture = fixture()?;
    let dir = tempfile::tempdir()?;
    let live_pid = std::process::id();
    let live_start = peer::start_identity(live_pid)?.0;
    let dead_pid = exited_pid()?;
    let mut live = 0_usize;
    for (n, session) in fixture.sessions.iter().enumerate() {
        // Two thirds live, one third with an owner that has exited.
        let seat = if n % 3 == 2 {
            seat(dir.path(), session, dead_pid, "macos:1.1")?
        } else {
            live += 1;
            seat(dir.path(), session, live_pid, &live_start)?
        };
        fs::create_dir_all(&seat.endpoint.dir)?;
        recovery::record_endpoint(&seat)?;
    }
    let owners = u64::try_from(fixture.sessions.len())?;
    let (found, counts) = recovery::recover(dir.path())?;
    assert_eq!(found.len(), fixture.sessions.len());
    assert_eq!(found.iter().filter(|owner| owner.live()).count(), live);
    assert!(
        found
            .iter()
            .filter(|owner| !owner.live())
            .all(|owner| matches!(owner.unreachable, Some(Unreachable::Exited))),
        "a waited-on pid is exited, never reused or unproved: {found:?}"
    );
    assert!(
        counts.record_visits <= owners * fixture.ceilings.record_visits_per_owner,
        "{} visits for {owners} owners",
        counts.record_visits
    );
    assert!(
        counts.proofs <= owners * fixture.ceilings.proofs_per_owner,
        "{} proofs for {owners} owners",
        counts.proofs
    );
    let mut sessions: Vec<&str> = found
        .iter()
        .map(|owner| owner.seat.binding.session.as_str())
        .collect();
    let mut expected: Vec<&str> = fixture.sessions.iter().map(String::as_str).collect();
    sessions.sort_unstable();
    expected.sort_unstable();
    assert_eq!(sessions, expected);
    Ok(())
}
