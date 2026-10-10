//! AGENTS-004 R6: every survival barrier red for its named reason, from
//! `seat_survival_red_matrix.json`, with the owner's record untouched.

use std::error::Error;
use std::fs;
use std::io::BufReader;
use std::path::Path;

use lys_runner::error::RunnerError;
use lys_runner::peer::{self, Leader, StartIdentity};
use lys_runner::seat_owner::counts::Meter;
use lys_runner::seat_owner::recovery::{self, Unreachable};
use lys_runner::seat_owner::sessions::OwnedSeat;
use lys_runner::seat_owner::spawn::{self, OwnerPlan};
use lys_runner::seat_owner::store::{
    Cursors, Custody, Holder, Intent, Lease, OwnerRecord, OwnerStore,
};
use serde::Deserialize;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    format: String,
    brief: String,
    rule: String,
    barriers: Vec<Barrier>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Barrier {
    barrier: String,
    #[serde(rename = "where")]
    site: String,
    refusal: String,
    untouched: String,
}

fn matrix() -> Result<Matrix, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("seat_survival_red_matrix.json");
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn expected<'a>(matrix: &'a Matrix, barrier: &str) -> Result<&'a Barrier, Box<dyn Error>> {
    matrix
        .barriers
        .iter()
        .find(|row| row.barrier == barrier)
        .ok_or_else(|| format!("the matrix has no barrier {barrier}").into())
}

fn names(row: &Barrier, name: &str) -> bool {
    row.refusal.split('|').any(|allowed| allowed == name)
}

fn refusal<T: std::fmt::Debug>(result: Result<T, RunnerError>) -> String {
    match result {
        Ok(value) => format!("accepted: {value:?}"),
        Err(error) => error.name().to_owned(),
    }
}

fn hex_id(n: u64) -> String {
    format!("{n:032x}")
}

fn record(session: &str, start: Leader) -> OwnerRecord {
    OwnerRecord {
        seat: format!("seat-{session}"),
        session: session.to_owned(),
        conversation: format!("conversation-{session}"),
        harness: None,
        lease: Lease {
            generation: 1,
            holder: Holder::Owner { start },
            taken_at: 1_760_060_000_000,
            build: "0.0.0-test".to_owned(),
        },
        custody: Custody::Owned,
        cursors: Cursors::default(),
        credential_references: Vec::new(),
        established_at: 1_760_060_000_000,
    }
}

#[test]
fn seat_survival_red_matrix_reads() -> TestResult {
    let matrix = matrix()?;
    assert_eq!(matrix.format, "lys-runner-seat-survival-red-matrix/v1");
    assert_eq!(matrix.brief, "AGENTS-004 R6");
    assert!(matrix.rule.contains("refused by name"));
    assert_eq!(matrix.barriers.len(), 5);
    for row in &matrix.barriers {
        assert!(!row.site.is_empty() && !row.untouched.is_empty(), "{row:?}");
    }
    Ok(())
}

#[test]
fn seat_survival_plan_unreadable_is_red_by_name() -> TestResult {
    let row = &matrix()?;
    let row = expected(row, "plan_unreadable")?;
    let dir = tempfile::tempdir()?;
    // No plan at all.
    assert!(names(row, &refusal(OwnerPlan::read(dir.path()))));
    // A torn plan.
    fs::write(
        dir.path().join("launch.json"),
        b"{\"binding\": {\"seat\": \"a\"",
    )?;
    assert!(names(row, &refusal(OwnerPlan::read(dir.path()))));
    // A plan of the wrong shape, read as nothing, never as a launch.
    fs::write(dir.path().join("launch.json"), b"{\"launch\": {}}")?;
    assert!(names(row, &refusal(OwnerPlan::read(dir.path()))));
    let entries: Vec<_> = fs::read_dir(dir.path())?.collect();
    assert_eq!(entries.len(), 1, "{}", row.untouched);
    Ok(())
}

#[test]
fn seat_survival_ready_pipe_closed_is_red_by_name() -> TestResult {
    let row = &matrix()?;
    let row = expected(row, "ready_pipe_closed")?;
    let meter = Meter::default();
    // The owner's stdout closed before any ready line.
    let mut closed = BufReader::new(std::io::empty());
    assert!(names(row, &refusal(spawn::wait_ready(&mut closed, &meter))));
    // The owner wrote something that is not a ready line, then closed.
    let mut noise = BufReader::new(&b"owner panicked: no plan\n"[..]);
    assert!(names(row, &refusal(spawn::wait_ready(&mut noise, &meter))));
    let work = meter.snapshot();
    assert_eq!(work.wakes, 0, "{}", row.untouched);
    assert_eq!(work.journal_appends, 0, "{}", row.untouched);
    Ok(())
}

#[test]
fn seat_survival_kernel_proof_mismatch_is_red_by_name() -> TestResult {
    let row = &matrix()?;
    let row = expected(row, "kernel_proof_mismatch")?;
    let dir = tempfile::tempdir()?;
    let pid = std::process::id();
    let seat: OwnedSeat = serde_json::from_value(serde_json::json!({
        "binding": { "seat": "seat-k", "session": "k", "conversation": "c-k", "generation": 1 },
        "endpoint": {
            "dir": dir.path().join("owners").join("k"),
            "socket": dir.path().join("owners").join("k").join("owner.sock"),
            "runner": "owner-k",
            "owner": { "pid": pid, "start": "macos:0.0" }
        },
        "established_at": 1_760_060_000_000u64
    }))?;
    match recovery::prove(&seat) {
        Some(Unreachable::PidReused { found }) => {
            assert_eq!(found, peer::start_identity(pid)?.0);
            assert!(names(row, "pid_reused"));
        }
        other => panic!("a forged start identity proved {other:?}"),
    }
    // Written as an endpoint record, the replacement runner keeps and names
    // it, restarts nothing.
    fs::create_dir_all(&seat.endpoint.dir)?;
    recovery::record_endpoint(&seat)?;
    let (found, counts) = recovery::recover(dir.path())?;
    assert_eq!(found.len(), 1);
    assert!(!found[0].live(), "{}", row.untouched);
    assert_eq!(counts.proofs, 1);
    assert!(
        seat.endpoint.dir.join("endpoint.json").exists(),
        "{}",
        row.untouched
    );
    Ok(())
}

#[test]
fn seat_survival_append_refused_is_red_by_name() -> TestResult {
    let row = &matrix()?;
    let row = expected(row, "append_refused")?;
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    let start = Leader {
        pid: 4_001,
        start: StartIdentity("boot-4001".to_owned()),
    };
    store.record(
        &hex_id(1),
        &Intent::Establish {
            record: record("a", start),
        },
    )?;
    let before = store.owner("a").cloned();
    let journal = dir.path().join("seat-owners.journal");
    fs::remove_file(&journal)?;
    fs::create_dir(&journal)?;
    let name = refusal(store.record(
        &hex_id(2),
        &Intent::Custody {
            session: "a".to_owned(),
            custody: Custody::Stopping {
                intent: hex_id(2),
                since: 2,
            },
        },
    ));
    assert!(names(row, &name), "{name}");
    assert_eq!(store.owner("a").cloned(), before, "{}", row.untouched);
    Ok(())
}

#[test]
fn seat_survival_checkpoint_refused_is_red_by_name() -> TestResult {
    let row = &matrix()?;
    let row = expected(row, "checkpoint_refused")?;
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    let start = Leader {
        pid: 4_002,
        start: StartIdentity("boot-4002".to_owned()),
    };
    store.record(
        &hex_id(3),
        &Intent::Establish {
            record: record("b", start),
        },
    )?;
    store.checkpoint()?;
    let projection = dir.path().join("seat-owners.json");
    let before = fs::read(&projection)?;
    store.record(
        &hex_id(4),
        &Intent::Retire {
            session: "b".to_owned(),
        },
    )?;
    fs::create_dir(dir.path().join("seat-owners.writing"))?;
    assert!(names(row, &refusal(store.checkpoint())));
    assert_eq!(fs::read(&projection)?, before, "{}", row.untouched);
    Ok(())
}
