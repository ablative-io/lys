//! A joined computer is selected by its key, never a host-name guess or list order.
//!
//! A device is known by one of two keys: the runner-machine key it joined
//! with, matched against a dialled runner's record; or, on the computer that
//! holds the install and has joined no other Lys, the install's runner key in
//! run/runner-server.pub, matched against the key the view of the install's
//! own runner carries (AGENTS-016 R1). A computer joined to a Lys keeps that
//! server's key in the same file, so the file is read only where no saved
//! join stands beside it.
//!
//! Advice names only what the screens show: the Computers page and the
//! Connect group of a computer, the labels surface/identity's
//! features/network/labels.ts defines and this crate's tests read.

use std::path::Path;

use lys_core::Ed25519Identity;
use serde::Deserialize;
use serde_json::Value;
use zeroize::Zeroizing;

use crate::commands::error::CliResult;
use crate::commands::launch_context::surface;
use crate::commands::seat_client::{Server, refused, segment};
use crate::identity::install::layout::Layout;
use crate::identity::private_files;

#[derive(Deserialize)]
struct Network {
    machines: Vec<Machine>,
}

#[derive(Deserialize)]
struct Machine {
    id: String,
    state: String,
}

/// The Computers page's name in the rail (labels.ts `COMPUTERS`).
pub(crate) const COMPUTERS: &str = "Computers";
/// The start of a computer's Connect group's name (labels.ts `CONNECT`).
pub(crate) const CONNECT: &str = "Connect ";

/// Where a person connects this computer, in the screens' own words, with
/// the Computers page's address when it is known.
pub(crate) fn advice(page: Option<&str>) -> String {
    let at = page.map_or_else(String::new, |page| format!(" at {page}"));
    format!("open {COMPUTERS}{at} and use {CONNECT}<this computer's name>")
}

/// The keys this device is known by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Device {
    /// The public half of the runner-machine key it joined with, as hex.
    pub(crate) machine: Option<String>,
    /// The install's runner key, on the computer that holds the install.
    pub(crate) server: Option<String>,
}

#[derive(Deserialize)]
struct RunnerView {
    machine: String,
    runner: Option<Record>,
    /// For the install's own runner, the install's runner key.
    #[serde(default)]
    runner_key: Option<String>,
    answers: Option<bool>,
    refusal: Option<String>,
    reason: Option<String>,
    /// When Lys lost its computer records, while that loss is accepted
    /// (DIRECTORY-107 R4).
    #[serde(default)]
    lost_at: Option<String>,
}

#[derive(Deserialize)]
struct Record {
    kind: String,
    key: Option<String>,
}

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> CliResult<T> {
    serde_json::from_value(value).map_err(|error| {
        refused(
            "machine_answer_unreadable",
            format!(
                "Lys did not answer in its recorded machine shape; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })
}

fn key_at(path: &Path) -> CliResult<Option<String>> {
    let Some(bytes) = private_files::read(path)? else {
        return Ok(None);
    };
    let seed = Zeroizing::new(<[u8; 32]>::try_from(bytes.as_slice()).map_err(|error| {
        refused(
            "runner_key_invalid",
            format!(
                "this device's private runner key is not 32 bytes: {error}; {}",
                advice(None)
            ),
        )
    })?);
    Ok(Some(lys_runner::protocol::hex(
        &Ed25519Identity::from_seed(&seed).public_key_bytes(),
    )))
}

/// Read the public half of an existing key, including an older join layout.
/// Nothing generates or moves a key while selecting a launch device.
///
/// # Errors
/// Names unreadable keys, conflicting layouts and devices that have not joined.
pub fn public_key(root: &Path) -> CliResult<String> {
    machine_key(root)?.ok_or_else(|| {
        refused(
            "machine_not_joined",
            format!("this device has not joined; {}", advice(None)),
        )
    })
}

/// The keys this device is known by: its runner-machine key, and the
/// install's runner key when it holds the install and has joined no other
/// Lys. Nothing is generated or moved.
///
/// # Errors
/// Names unreadable keys, conflicting layouts and a device known by neither.
pub(crate) fn device(root: &Path) -> CliResult<Device> {
    let layout = Layout {
        root: root.to_owned(),
    };
    let machine = machine_key(root)?;
    let server = if layout.run_dir().join("runner-join.json").exists() {
        None
    } else {
        std::fs::read_to_string(layout.run_dir().join("runner-server.pub"))
            .map(|key| Some(key.trim().to_owned()))
            .or_else(|error| match error.kind() {
                std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(refused(
                    "runner_server_key_unreadable",
                    format!("this device's run/runner-server.pub cannot be read: {error}"),
                )),
            })?
            .filter(|key| !key.is_empty())
    };
    if machine.is_none() && server.is_none() {
        return Err(refused(
            "machine_not_joined",
            format!("this device has not joined; {}", advice(None)),
        ));
    }
    Ok(Device { machine, server })
}

fn machine_key(root: &Path) -> CliResult<Option<String>> {
    let layout = Layout {
        root: root.to_owned(),
    };
    let current = key_at(&layout.runner_machine_key())?;
    let older = key_at(
        &layout
            .data_dir()
            .join(crate::commands::runner_join::MACHINE_KEY),
    )?;
    match (current, older) {
        (Some(current), Some(older)) if current != older => Err(refused(
            "runner_machine_key_conflict",
            "the installed and old join keys name different devices; resolve the key conflict on the Computers page before launching",
        )),
        (Some(key), _) | (None, Some(key)) => Ok(Some(key)),
        (None, None) => Ok(None),
    }
}

/// Select this device's joined machine and require its runner to answer now.
///
/// # Errors
/// Names absent, ambiguous, retired or offline machine identities and server refusals.
pub fn find(server: &Server, root: &Path, base: &str) -> CliResult<String> {
    find_with(server, root, base, true)
}

/// Identify the joined machine for a list even while its runner is disconnected.
///
/// # Errors
/// Names absent, ambiguous or retired machine identities and server refusals.
pub fn listed(server: &Server, root: &Path, base: &str) -> CliResult<String> {
    find_with(server, root, base, false)
}

fn find_with(server: &Server, root: &Path, base: &str, online: bool) -> CliResult<String> {
    let device = device(root)?;
    select(&device, |path| server.get(path), online).map_err(|error| {
        let name = match &error {
            crate::commands::error::CliError::Runner(error) => error.name(),
            _ => "machine_unavailable".to_owned(),
        };
        refused(
            &name,
            format!("{error}; {}", advice(Some(&surface(base, "network")))),
        )
    })
}

#[cfg(test)]
fn select_with(key: &str, get: impl FnMut(&str) -> CliResult<Value>) -> CliResult<String> {
    let device = Device {
        machine: Some(key.to_owned()),
        server: None,
    };
    select(&device, get, true)
}

fn select(
    device: &Device,
    mut get: impl FnMut(&str) -> CliResult<Value>,
    online: bool,
) -> CliResult<String> {
    let network: Network = decode(get("/network")?)?;
    let mut matched = Vec::new();
    let mut retired = false;
    let mut lost_at = None;
    for machine in network.machines {
        segment("machine_id_invalid", "the machine id", &machine.id)?;
        if !matches!(machine.state.as_str(), "in_use" | "retired") {
            return Err(refused(
                "machine_answer_unreadable",
                "the machine answer has no recognized lifecycle state",
            ));
        }
        let record: RunnerView = decode(get(&format!("/network/machines/{}/runner", machine.id))?)?;
        if record.machine != machine.id {
            return Err(refused(
                "machine_answer_unreadable",
                "the runner answer names another machine",
            ));
        }
        if record.lost_at.is_some() {
            lost_at.clone_from(&record.lost_at);
        }
        let this = record
            .runner
            .as_ref()
            .is_some_and(|runner| match runner.kind.as_str() {
                "dialled" => device.machine.is_some() && runner.key == device.machine,
                "lys" => device.server.is_some() && record.runner_key == device.server,
                _ => false,
            });
        if this {
            if machine.state == "retired" {
                retired = true;
            } else {
                matched.push((machine, record));
            }
        }
    }
    if matched.len() > 1 {
        return Err(refused(
            "machine_identity_ambiguous",
            "this device's key names multiple server machines; retire the duplicate machine on the Computers page",
        ));
    }
    let (machine, record) = matched.pop().ok_or_else(|| {
        if retired {
            refused(
                "MachineRetired",
                "every joined machine for this device's key is retired",
            )
        } else {
            let words = "Lys's visible runner records do not identify a joined machine for this device's key";
            match &lost_at {
                Some(at) => refused(
                    "machine_not_joined",
                    format!("{words}; Lys lost its computer records at {at}; connect this computer again"),
                ),
                None => refused("machine_not_joined", words),
            }
        }
    })?;
    if online && record.answers != Some(true) {
        let name = record.refusal.as_deref().ok_or_else(|| {
            refused(
                "machine_answer_unreadable",
                "the joined runner did not answer or name its refusal",
            )
        })?;
        let reason = record.reason.ok_or_else(|| {
            refused(
                "machine_answer_unreadable",
                "the runner's refusal has no reason",
            )
        })?;
        return Err(refused(name, reason));
    }
    Ok(machine.id)
}

#[cfg(test)]
#[path = "launch_device_tests.rs"]
mod tests;
