//! The installed configuration and runner identity select the local computer's receipt.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use super::{Layout, failed, identity};
use crate::identity::error::IdentityResult;

#[derive(Deserialize)]
struct Config {
    network_file: PathBuf,
    runner_socket: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Network {
    machines: Vec<Computer>,
    #[serde(default)]
    runners: BTreeMap<String, Runner>,
}

#[derive(Deserialize)]
struct Computer {
    id: String,
    retired: Option<Value>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Runner {
    Lys,
    Socket { path: PathBuf },
    Dialled { runner: Option<String> },
}

pub(super) fn read(layout: &Layout) -> IdentityResult<Option<String>> {
    let config_path = layout.service_config();
    let bytes = match fs::read(&config_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failed(&config_path, &error)),
    };
    let config: Config =
        serde_json::from_slice(&bytes).map_err(|error| failed(&config_path, &error))?;
    let bytes =
        fs::read(&config.network_file).map_err(|error| failed(&config.network_file, &error))?;
    let runners = runners(&bytes).map_err(|error| failed(&config.network_file, &error))?;
    let runner_path = layout.data_dir().join("runner/runner.id");
    let runner_id = match fs::read_to_string(&runner_path) {
        Ok(text) => {
            let id = text.trim();
            if id.len() != 32 || lys_runner::protocol::unhex(id).is_none() {
                return Err(failed(&runner_path, "runner_machine_runner_id_invalid"));
            }
            Some(id.to_owned())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(failed(&runner_path, &error)),
    };
    let mut own = None;
    for (machine, runner) in runners {
        let local = match runner {
            Runner::Lys => true,
            Runner::Socket { path } => config.runner_socket.as_ref() == Some(&path),
            Runner::Dialled { runner } => runner_id.is_some() && runner == runner_id,
        };
        if !local {
            continue;
        }
        if !identity(&machine, "op-") {
            return Err(failed(
                &config.network_file,
                "runner_machine_install_identity_invalid",
            ));
        }
        if own.is_some() {
            return Err(failed(
                &config.network_file,
                "runner_machine_install_conflict: several computers name this install's runner",
            ));
        }
        own = Some(machine);
    }
    Ok(own)
}

fn runners(bytes: &[u8]) -> Result<BTreeMap<String, Runner>, String> {
    let Some(records) = bytes.strip_prefix(b"LYS-NETWORK-1\n") else {
        return serde_json::from_slice::<Network>(bytes)
            .map(active_runners)
            .map_err(|error| error.to_string());
    };
    let end = records
        .iter()
        .rposition(|byte| *byte == b'\n')
        .ok_or("runner_machine_network_snapshot_incomplete")?;
    if end + 1 != records.len() {
        return Err("runner_machine_network_change_incomplete".to_owned());
    }
    let mut lines = records[..end].split(|byte| *byte == b'\n');
    let snapshot = lines
        .next()
        .ok_or("runner_machine_network_snapshot_missing")?;
    let mut network: Network =
        serde_json::from_slice(snapshot).map_err(|error| error.to_string())?;
    for line in lines {
        let change: Value = serde_json::from_slice(line).map_err(|error| error.to_string())?;
        match change.get("change").and_then(Value::as_str) {
            Some("runner") => {
                let machine = change
                    .get("machine")
                    .and_then(Value::as_str)
                    .ok_or("runner_machine_network_change_machine_missing")?;
                let runner = change
                    .get("runner")
                    .ok_or("runner_machine_network_change_runner_missing")?;
                if runner.is_null() {
                    network.runners.remove(machine);
                } else {
                    network.runners.insert(
                        machine.to_owned(),
                        serde_json::from_value(runner.clone())
                            .map_err(|error| error.to_string())?,
                    );
                }
            }
            Some("named") => {
                network.machines.push(
                    serde_json::from_value(
                        change
                            .get("machine")
                            .ok_or("runner_machine_network_change_machine_missing")?
                            .clone(),
                    )
                    .map_err(|error| error.to_string())?,
                );
            }
            Some("retired") => {
                let machine = change
                    .get("machine")
                    .and_then(Value::as_str)
                    .ok_or("runner_machine_network_change_machine_missing")?;
                let computer = network
                    .machines
                    .iter_mut()
                    .find(|held| held.id == machine)
                    .ok_or("runner_machine_network_retired_machine_unknown")?;
                computer.retired = Some(
                    change
                        .get("retirement")
                        .ok_or("runner_machine_network_retirement_missing")?
                        .clone(),
                );
            }
            Some("team" | "agent") => {}
            _ => return Err("runner_machine_network_change_unknown".to_owned()),
        }
    }
    Ok(active_runners(network))
}

fn active_runners(mut network: Network) -> BTreeMap<String, Runner> {
    let retired: BTreeSet<_> = network
        .machines
        .into_iter()
        .filter(|machine| machine.retired.is_some())
        .map(|machine| machine.id)
        .collect();
    network
        .runners
        .retain(|machine, _| !retired.contains(machine));
    network.runners
}
