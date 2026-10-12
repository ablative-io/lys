//! Local runner admission keeps the recorded install key in the existing receipt format.

use lys_identity::{MachineId, OperationId, PersonId};
use std::str::FromStr;

use super::{JoinRecord, JoinStanding, JoinStore, unavailable};
use crate::error::ServerError;
use crate::network_store::Machine;

pub(crate) fn joined_local(
    store: &mut JoinStore,
    machine: &Machine,
    key: &str,
) -> Result<Option<MachineId>, ServerError> {
    OperationId::from_str(&machine.id)?;
    PersonId::from_str(&machine.named_by)?;
    if let Some(record) = store.kept.codes.iter().rev().find(|record| {
        record.machine == machine.id && matches!(record.standing, JoinStanding::Used { .. })
    }) {
        let JoinStanding::Used {
            key: held,
            identity,
            ..
        } = &record.standing
        else {
            return Err(unavailable("runner_machine_receipt_invalid"));
        };
        if !held.eq_ignore_ascii_case(key) {
            return Err(unavailable(
                "runner_machine_key_conflict: the installed key differs from the computer's recorded machine key",
            ));
        }
        if identity.is_some() {
            return Ok(None);
        }
    }
    let identity = MachineId::generate()?;
    let mut next = store.kept.clone();
    next.codes.push(JoinRecord {
        operation: OperationId::generate()?.to_string(),
        machine: machine.id.clone(),
        issued_by: machine.named_by.clone(),
        issued_at: machine.named_at,
        standing: JoinStanding::Used {
            at: machine.named_at,
            key: key.to_owned(),
            identity: Some(identity.to_string()),
        },
    });
    store.write(next)?;
    Ok(Some(identity))
}
