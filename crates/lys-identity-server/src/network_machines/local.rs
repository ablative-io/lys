//! The managed install's public machine key supplies its local runner receipt.

use std::fs;

use serde::Deserialize;

use crate::error::ServerError;
use crate::network_api::with_network;
use crate::network_join::{joined_local, with_joins};
use crate::routes::AppState;
use crate::runner_client::{RunnerRecord, dial_key};

#[derive(Deserialize)]
struct Build {
    runner_machine: Recorded,
}

#[derive(Deserialize)]
struct Recorded {
    key: String,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::NetworkUnavailable {
        reason: reason.into(),
    }
}

pub(super) fn ensure(state: &AppState) -> Result<(), ServerError> {
    let Some(data) = state
        .estate_plan_file
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "data"))
    else {
        return Ok(());
    };
    let Some(root) = data.parent() else {
        return Err(unavailable("runner_machine_install_root_missing"));
    };
    if state.network.is_none() {
        return Ok(());
    }
    let locals = with_network(state, |store| {
        Ok(store
            .machines()
            .iter()
            .filter(|machine| {
                machine.retired.is_none() && store.runner(&machine.id) == Some(&RunnerRecord::Lys)
            })
            .cloned()
            .collect::<Vec<_>>())
    })?;
    if locals.is_empty() {
        return Ok(());
    }
    let path = root.join("install/build.json");
    let bytes = fs::read(&path).map_err(|error| {
        unavailable(format!("reading the runner machine build record: {error}"))
    })?;
    let build: Build = serde_json::from_slice(&bytes)
        .map_err(|error| unavailable(format!("runner_machine_record_invalid: {error}")))?;
    let key = dial_key(&build.runner_machine.key).ok_or_else(|| {
        unavailable("runner_machine_record_invalid: the recorded key is not an Ed25519 public key")
    })?;
    let key = lys_runner::protocol::hex(&key);
    for machine in locals {
        let created = with_joins(state, |joins| joined_local(joins, &machine, &key))?;
        if let Some(identity) = created {
            (state.say)(&format!(
                "local runner machine identity {identity} recorded for {}",
                machine.id
            ));
        }
    }
    Ok(())
}
