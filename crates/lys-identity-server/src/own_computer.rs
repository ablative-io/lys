//! The computer Lys runs on, named once in the network store (AGENTS-017 R4).
//!
//! A start places an agent on a computer, and this install's own computer is a
//! device once a machine names its runner as `lys` (AGENTS-016 R1). Nothing
//! named it: a person had to add it on Network first. Now it is named for the
//! configured administrator when setup finishes and whenever the administrator
//! records their roots (`POST /grants/agent-roots`), which every install and
//! upgrade's first administrator act reaches; a fresh install has no
//! administrator to name it for until setup.
//!
//! Naming is idempotent and never takes what a person named: a network store
//! that already names a live machine with the `lys` runner is left alone, a
//! machine of this id a person retired is left retired, and
//! the machine's id is an operation derived from this install's runner-server
//! public key, so naming it again finds the same machine and writes nothing.
//! The machine's name is the name this computer gives itself, as the Network
//! screen offers it when a person adds a computer.

use lys_identity::{OperationId, PersonId};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::network_api::with_network;
use crate::network_store::Machine;
use crate::routes::AppState;
use crate::runner_client::RunnerRecord;
use crate::session::now;

/// What the machine is called when this computer names itself nothing.
const UNNAMED: &str = "The computer Lys runs on";

/// The operation that names this install's computer: the same for every
/// install and upgrade that keeps the runner-server key.
fn operation(runner_key: &[u8; 32]) -> String {
    let hash = Sha256::digest([b"lys/own-computer/v1/".as_slice(), runner_key].concat());
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    OperationId::from_bytes(id).to_string()
}

/// Name the computer Lys runs on for `administrator`, once, with its runner
/// `lys`; the machine's id. None when this service keeps no network.
///
/// # Errors
/// The network store's own refusals when it cannot be read or written.
pub(crate) fn name(
    state: &AppState,
    administrator: PersonId,
) -> Result<Option<String>, ServerError> {
    if state.network.is_none() {
        return Ok(None);
    }
    let id = operation(&state.runners.public_key());
    let name = crate::network_api::host_name::read().unwrap_or_else(|| UNNAMED.to_owned());
    with_network(state, |store| {
        if let Some(named) = store.lys_runner_machine() {
            return Ok(Some(named.id.clone()));
        }
        match store.machine(&id) {
            // A person retired it: it is theirs to name again, never the install's.
            Some(kept) if kept.retired.is_some() => return Ok(None),
            Some(_) => {}
            None => store.name(Machine {
                id: id.clone(),
                name,
                kind: "Computer".to_owned(),
                runtime: Some("lys-runner".to_owned()),
                slots: 0,
                may_run: Vec::new(),
                may_run_roles: Vec::new(),
                may_reach: Vec::new(),
                named_by: administrator.to_string(),
                named_at: now(),
                retired: None,
                team: None,
                creation_team: None,
            })?,
        }
        store.name_runner(&id, Some(RunnerRecord::Lys))?;
        Ok(Some(id))
    })
}
