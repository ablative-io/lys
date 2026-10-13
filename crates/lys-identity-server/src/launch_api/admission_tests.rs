use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, TryLockError};

use lys_core::Ed25519Identity;
use serde_json::json;

use super::keep_with_runner;
use crate::error::ServerError;
use crate::network_store::{Machine, NetworkStore};
use crate::runner_client::RunnerRecord;
use crate::runtime_state::{Report, Reported};
use crate::runtime_store::RuntimeStore;

fn bytes(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    let mut kept = BTreeMap::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            kept.extend(bytes(&entry.path())?);
        } else {
            kept.insert(entry.path(), std::fs::read(entry.path())?);
        }
    }
    Ok(kept)
}

fn remove_at_append(network: &Mutex<NetworkStore>, machine: &str) -> Result<bool, ServerError> {
    match network.try_lock() {
        Ok(mut store) => {
            store.name_runner(machine, None)?;
            Ok(true)
        }
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Poisoned(error)) => Err(ServerError::NetworkUnavailable {
            reason: format!("the network lock is poisoned: {error}"),
        }),
    }
}

#[test]
fn runner_removal_is_ordered_before_or_after_the_starting_append() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let runtime_path = dir.path().join("runtime");
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut runtime = RuntimeStore::open(&runtime_path, key)?;
    let mut network = NetworkStore::open(&dir.path().join("network.json"))?;
    let machine = "op-00000000000000000000000000000001";
    network.name(Machine {
        id: machine.to_owned(),
        name: "Computer".to_owned(),
        kind: "computer".to_owned(),
        runtime: Some("sh".to_owned()),
        slots: 1,
        may_run: vec!["agent".to_owned()],
        may_run_roles: Vec::new(),
        may_reach: Vec::new(),
        named_by: "person".to_owned(),
        named_at: 1,
        retired: None,
        team: None,
        creation_team: None,
    })?;
    network.name_runner(machine, Some(RunnerRecord::Lys))?;
    network.name_runner(machine, None)?;
    let network = Mutex::new(network);
    let report = Report {
        operation: "start".to_owned(),
        session: "start".to_owned(),
        agent: Some("agent".to_owned()),
        machine: machine.to_owned(),
        state: Reported::Starting,
        what: "start admitted".to_owned(),
        confirmation: String::new(),
        reported_by: "person".to_owned(),
        at: 1,
        launch: Some(json!({"session": "start"})),
    };
    let before = bytes(&runtime_path)?;
    let mut called = false;
    let refused = keep_with_runner(&network, machine, || {
        called = true;
        runtime.report(report.clone())
    });
    assert!(
        matches!(refused, Err(ServerError::MachineWithoutRunner)),
        "{refused:?}"
    );
    assert!(!called, "removal before admission must prevent the append");
    assert!(runtime.sessions().is_empty());
    assert_eq!(bytes(&runtime_path)?, before);

    network
        .lock()
        .map_err(|error| error.to_string())?
        .name_runner(machine, Some(RunnerRecord::Lys))?;
    let mut removed = None;
    let held = keep_with_runner(&network, machine, || {
        removed = Some(remove_at_append(&network, machine)?);
        runtime.report(report.clone())
    })?;
    assert_eq!(
        removed,
        Some(false),
        "runner removal completed between admission and the Starting append"
    );
    assert_eq!(held.first(), Some(&report));
    assert_eq!(held.reports.len(), 1);
    assert_eq!(runtime.sessions().len(), 1);
    assert_eq!(
        network
            .lock()
            .map_err(|error| error.to_string())?
            .runner(machine),
        Some(&RunnerRecord::Lys)
    );
    let appended = bytes(&runtime_path)?;
    assert_ne!(appended, before);
    assert!(
        remove_at_append(&network, machine)?,
        "the append must release the network lock"
    );
    assert!(
        network
            .lock()
            .map_err(|error| error.to_string())?
            .runner(machine)
            .is_none()
    );
    assert_eq!(runtime.session("start"), Some(&held));
    assert_eq!(bytes(&runtime_path)?, appended);

    network
        .lock()
        .map_err(|error| error.to_string())?
        .name_runner(machine, Some(RunnerRecord::Lys))?;
    refused_append(&network, &runtime, &runtime_path, &appended, &held)?;
    Ok(())
}

fn refused_append(
    network: &Mutex<NetworkStore>,
    runtime: &RuntimeStore,
    runtime_path: &Path,
    appended: &BTreeMap<PathBuf, Vec<u8>>,
    held: &crate::runtime_state::Tracked,
) -> Result<(), Box<dyn Error>> {
    let machine = "op-00000000000000000000000000000001";
    let failed = keep_with_runner(network, machine, || {
        Err(ServerError::RuntimeUnavailable {
            reason: "append refused".to_owned(),
        })
    });
    assert!(
        matches!(failed, Err(ServerError::RuntimeUnavailable { ref reason }) if reason == "append refused")
    );
    assert_eq!(&bytes(runtime_path)?, appended);
    assert!(
        remove_at_append(network, machine)?,
        "a refused append must release the network lock"
    );
    assert_eq!(runtime.session("start"), Some(held));
    Ok(())
}
