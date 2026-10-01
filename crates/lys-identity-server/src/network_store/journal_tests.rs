use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::{AgentsRecorded, Kept, Machine, NetworkStore, Retirement, TeamRecorded};
use crate::error::ServerError;
use crate::runner_client::RunnerRecord;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

thread_local! {
    static WRITE_BYTES: Cell<usize> = const { Cell::new(0) };
    static REPLAYED: Cell<u64> = const { Cell::new(0) };
    static TAIL_RECOVERIES: Cell<u64> = const { Cell::new(0) };
}

pub(super) fn written(bytes: usize) {
    WRITE_BYTES.with(|count| count.set(count.get() + bytes));
}

pub(super) fn replayed() {
    REPLAYED.with(|count| count.set(count.get() + 1));
}

pub(super) fn recovered() {
    TAIL_RECOVERIES.with(|count| count.set(count.get() + 1));
}

fn machine(position: usize) -> Machine {
    Machine {
        id: format!("machine-{position:04}"),
        name: "Build box".to_owned(),
        kind: "server".to_owned(),
        runtime: Some("norn".to_owned()),
        slots: 4,
        may_run: Vec::new(),
        may_run_roles: Vec::new(),
        may_reach: vec!["git.example.test".to_owned()],
        named_by: "person".to_owned(),
        named_at: 5,
        retired: None,
        team: None,
        creation_team: None,
    }
}

fn ownership(operation: &str, team: Option<&str>) -> TeamRecorded {
    TeamRecorded {
        operation: operation.to_owned(),
        machine: "machine-0000".to_owned(),
        team: team.map(str::to_owned),
        by: "person".to_owned(),
        at: 8,
    }
}

fn fixture(kept: &Kept) -> TestResult<(tempfile::TempDir, PathBuf)> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    fs::write(&path, serde_json::to_vec_pretty(kept)?)?;
    Ok((directory, path))
}

fn bytes_for_change(machines: usize) -> TestResult<usize> {
    let kept = Kept {
        machines: (0..machines).map(machine).collect(),
        ..Kept::default()
    };
    let (directory, path) = fixture(&kept)?;
    let mut store = NetworkStore::open(&path)?;
    store.assign_team(ownership("initial-team", None))?;
    WRITE_BYTES.with(|count| count.set(0));
    store.assign_team(ownership("team-change", Some("team-a")))?;
    let bytes = WRITE_BYTES.with(Cell::get);
    drop(store);
    directory.close()?;
    Ok(bytes)
}

#[test]
fn a_change_writes_only_one_record_as_retained_state_grows() -> TestResult {
    let small = bytes_for_change(1)?;
    let large = bytes_for_change(512)?;
    assert_eq!(
        small, large,
        "a change must not serialize retained machines"
    );
    assert!(large <= 1024, "one ownership receipt wrote {large} bytes");
    Ok(())
}

#[test]
fn a_legacy_install_keeps_machines_runners_and_receipts_after_its_first_change() -> TestResult {
    let assigned = ownership("first-team", Some("team-a"));
    let allowance = AgentsRecorded {
        operation: "first-agent".to_owned(),
        machine: "machine-0000".to_owned(),
        agent: "agent-a".to_owned(),
        allow: true,
        by: "person".to_owned(),
        at: 7,
        original_may_run: Some(Vec::new()),
    };
    let mut first = machine(0);
    first.team = assigned.team.clone();
    first.may_run.push(allowance.agent.clone());
    let kept = Kept {
        machines: vec![first.clone(), machine(1)],
        runners: BTreeMap::from([(first.id.clone(), RunnerRecord::Lys)]),
        team_changes: BTreeMap::from([(assigned.operation.clone(), assigned.clone())]),
        agent_changes: BTreeMap::from([(allowance.operation.clone(), allowance.clone())]),
    };
    let (directory, path) = fixture(&kept)?;
    let original = fs::read(&path)?;
    let mut store = NetworkStore::open(&path)?;
    assert_eq!(fs::read(&path)?, original);
    assert_eq!(store.machine(&first.id), Some(&first));
    assert_eq!(store.runner(&first.id), Some(&RunnerRecord::Lys));
    assert_eq!(store.team_recorded(&assigned.operation), Some(&assigned));
    assert_eq!(store.agent_recorded(&allowance.operation), Some(&allowance));
    let cleared = ownership("clear-team", None);
    store.assign_team(cleared.clone())?;
    store.retire(
        "machine-0001",
        Retirement {
            by: "person".to_owned(),
            at: 9,
        },
    )?;
    drop(store);
    let mut reopened = NetworkStore::open(&path)?;
    assert_eq!(
        reopened.machine(&first.id).ok_or("machine missing")?.team,
        None
    );
    assert_eq!(reopened.runner(&first.id), Some(&RunnerRecord::Lys));
    assert_eq!(reopened.team_recorded(&assigned.operation), Some(&assigned));
    assert_eq!(reopened.team_recorded(&cleared.operation), Some(&cleared));
    assert_eq!(
        reopened.agent_recorded(&allowance.operation),
        Some(&allowance)
    );
    assert_eq!(reopened.assign_team(cleared.clone())?, cleared);
    assert_eq!(reopened.change_agent(allowance.clone())?, allowance);
    let original_name = machine(0);
    reopened.name(original_name)?;
    assert!(
        reopened
            .machine("machine-0001")
            .ok_or("retired machine missing")?
            .retired
            .is_some()
    );
    drop(reopened);
    directory.close()?;
    Ok(())
}

#[test]
fn every_change_reopens_with_its_receipt_and_machine_state() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    let mut store = NetworkStore::open(&path)?;
    let named = machine(0);
    store.name(named.clone())?;
    store.name_runner(&named.id, Some(RunnerRecord::Lys))?;
    let assigned = ownership("assign-team", Some("team-a"));
    store.assign_team(assigned.clone())?;
    let allowed = AgentsRecorded {
        operation: "allow-agent".to_owned(),
        machine: named.id.clone(),
        agent: "agent-a".to_owned(),
        allow: true,
        by: "person".to_owned(),
        at: 9,
        original_may_run: None,
    };
    let allowed = store.change_agent(allowed)?;
    let removed = AgentsRecorded {
        operation: "remove-agent".to_owned(),
        allow: false,
        ..allowed.clone()
    };
    let removed = store.change_agent(removed)?;
    assert_eq!(removed.original_may_run, None);
    store.name_runner(&named.id, None)?;
    let retired = Retirement {
        by: "person".to_owned(),
        at: 10,
    };
    store.retire(&named.id, retired.clone())?;
    drop(store);
    let bytes = fs::read(&path)?;
    let mut store = NetworkStore::open(&path)?;
    assert_eq!(fs::read(&path)?, bytes);
    assert_eq!(
        store.machine(&named.id).ok_or("machine missing")?.retired,
        Some(retired)
    );
    assert!(
        store
            .machine(&named.id)
            .ok_or("machine missing")?
            .may_run
            .is_empty()
    );
    assert_eq!(
        store.machine(&named.id).ok_or("machine missing")?.team,
        assigned.team
    );
    assert_eq!(store.runner(&named.id), None);
    assert_eq!(store.team_recorded(&assigned.operation), Some(&assigned));
    assert_eq!(store.agent_recorded(&allowed.operation), Some(&allowed));
    assert_eq!(store.agent_recorded(&removed.operation), Some(&removed));
    store.name(named)?;
    assert_eq!(
        fs::read(&path)?,
        bytes,
        "replaying creation must not append"
    );
    drop(store);
    directory.close()?;
    Ok(())
}

#[test]
fn a_torn_tail_is_synced_back_to_the_complete_prefix_before_appending() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    let mut store = NetworkStore::open(&path)?;
    store.name(machine(0))?;
    drop(store);
    let original = fs::read(&path)?;
    for suffix in [b"{\"change\":\"team\"".as_slice(), b"{}".as_slice()] {
        let mut incomplete = original.clone();
        incomplete.extend(suffix);
        fs::write(&path, &incomplete)?;
        TAIL_RECOVERIES.with(|count| count.set(0));
        let mut store = NetworkStore::open(&path)?;
        assert_eq!(
            TAIL_RECOVERIES.with(Cell::get),
            1,
            "recovery is counted only after set_len and sync succeed"
        );
        assert_eq!(
            fs::read(&path)?,
            original,
            "only the incomplete final record is removed"
        );
        let change = ownership("after-recovery", Some("team-a"));
        store.assign_team(change.clone())?;
        drop(store);
        let store = NetworkStore::open(&path)?;
        assert_eq!(store.team_recorded(&change.operation), Some(&change));
    }
    directory.close()?;
    Ok(())
}

#[test]
fn a_complete_malformed_record_or_incomplete_snapshot_remains_a_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    let mut store = NetworkStore::open(&path)?;
    store.name(machine(0))?;
    drop(store);
    let mut malformed = fs::read(&path)?;
    malformed.extend(b"{}\n");
    for bytes in [malformed, b"LYS-NETWORK-1\n{\"machines\":[".to_vec()] {
        fs::write(&path, &bytes)?;
        assert!(matches!(
            NetworkStore::open(&path),
            Err(ServerError::NetworkUnavailable { .. })
        ));
        assert_eq!(
            fs::read(&path)?,
            bytes,
            "a complete malformed record or incomplete snapshot is not recovery data"
        );
    }
    directory.close()?;
    Ok(())
}

fn journal_fixture(path: &std::path::Path, appends: u64) -> TestResult {
    let kept = Kept {
        machines: vec![machine(0)],
        ..Kept::default()
    };
    let mut bytes = b"LYS-NETWORK-1\n".to_vec();
    serde_json::to_writer(&mut bytes, &kept)?;
    bytes.push(b'\n');
    for position in 0..appends {
        let change = super::journal::Change::Team {
            recorded: ownership(&format!("seeded-{position}"), None),
        };
        serde_json::to_writer(&mut bytes, &change)?;
        bytes.push(b'\n');
    }
    fs::write(path, bytes)?;
    Ok(())
}

#[test]
fn a_start_after_the_snapshot_boundary_replays_only_the_bounded_tail() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    let interval = lys_identity::SNAPSHOT_EVERY.get();
    journal_fixture(&path, interval - 1)?;
    let mut store = NetworkStore::open(&path)?;
    store.assign_team(ownership("at-boundary", Some("team-a")))?;
    assert_eq!(store.since_snapshot, 0);
    for position in 0..5 {
        store.assign_team(ownership(&format!("after-boundary-{position}"), None))?;
    }
    drop(store);
    REPLAYED.with(|count| count.set(0));
    let store = NetworkStore::open(&path)?;
    assert_eq!(
        REPLAYED.with(Cell::get),
        5,
        "count actual decoded and applied change records"
    );
    assert!(REPLAYED.with(Cell::get) <= interval);
    assert_eq!(store.team_records().count(), usize::try_from(interval + 5)?);
    assert!(store.team_recorded("seeded-0").is_some());
    assert!(store.team_recorded("at-boundary").is_some());
    assert!(store.team_recorded("after-boundary-4").is_some());
    drop(store);
    directory.close()?;
    Ok(())
}

#[test]
fn a_start_at_the_snapshot_boundary_checkpoints_before_the_next_append() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    let interval = lys_identity::SNAPSHOT_EVERY.get();
    journal_fixture(&path, interval)?;
    let mut store = NetworkStore::open(&path)?;
    assert_eq!(store.since_snapshot, 0);
    store.assign_team(ownership("after-interrupted-checkpoint", None))?;
    drop(store);
    REPLAYED.with(|count| count.set(0));
    let store = NetworkStore::open(&path)?;
    assert_eq!(REPLAYED.with(Cell::get), 1);
    assert_eq!(store.team_records().count(), usize::try_from(interval + 1)?);
    drop(store);
    directory.close()?;
    Ok(())
}

#[test]
fn a_failed_checkpoint_refuses_the_next_append_until_reconciled() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("network.json");
    let interval = lys_identity::SNAPSHOT_EVERY.get();
    journal_fixture(&path, interval - 1)?;
    let mut store = NetworkStore::open(&path)?;
    let writing = path.with_extension("writing");
    fs::create_dir(&writing)?;
    let boundary = ownership("at-failed-checkpoint", Some("team-a"));
    assert!(matches!(
        store.assign_team(boundary.clone()),
        Err(ServerError::NetworkUnavailable { .. })
    ));
    let committed = fs::read(&path)?;
    assert!(matches!(
        NetworkStore::open(&path),
        Err(ServerError::NetworkUnavailable { .. })
    ));
    let next = ownership("after-failed-checkpoint", None);
    assert!(matches!(
        store.assign_team(next.clone()),
        Err(ServerError::NetworkUnavailable { .. })
    ));
    assert_eq!(
        fs::read(&path)?,
        committed,
        "no append is admitted after the bounded tail is full"
    );
    fs::remove_dir(&writing)?;
    assert_eq!(store.assign_team(boundary.clone())?, boundary);
    assert_eq!(store.since_snapshot, 0);
    assert_eq!(store.assign_team(next.clone())?, next);
    drop(store);
    REPLAYED.with(|count| count.set(0));
    let store = NetworkStore::open(&path)?;
    assert_eq!(REPLAYED.with(Cell::get), 1);
    assert_eq!(store.team_records().count(), usize::try_from(interval + 1)?);
    drop(store);
    directory.close()?;
    Ok(())
}

#[test]
fn a_failed_migration_reloads_the_legacy_snapshot_before_the_next_change() -> TestResult {
    let kept = Kept {
        machines: vec![machine(0)],
        ..Kept::default()
    };
    let (directory, path) = fixture(&kept)?;
    let original = fs::read(&path)?;
    let mut store = NetworkStore::open(&path)?;
    let writing = path.with_extension("writing");
    fs::create_dir(&writing)?;
    let change = ownership("assign-team", Some("team-a"));
    assert!(matches!(
        store.assign_team(change.clone()),
        Err(ServerError::NetworkUnavailable { .. })
    ));
    assert_eq!(fs::read(&path)?, original);
    assert_eq!(
        store.machine("machine-0000").ok_or("machine missing")?.team,
        None
    );
    assert_eq!(store.team_recorded(&change.operation), None);
    fs::remove_dir(&writing)?;
    assert_eq!(store.assign_team(change.clone())?, change);
    drop(store);
    let store = NetworkStore::open(&path)?;
    assert_eq!(store.team_recorded(&change.operation), Some(&change));
    drop(store);
    directory.close()?;
    Ok(())
}
