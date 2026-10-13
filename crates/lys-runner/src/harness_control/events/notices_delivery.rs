use super::{apply, runtime};
use crate::error::RunnerError;
use crate::session::Table;

pub(crate) fn expire(table: &mut Table, id: &str, now: u64) -> Result<(), RunnerError> {
    let generation = table
        .sessions
        .get(id)
        .ok_or_else(|| {
            RunnerError::refused("session_unknown", "managed notice session disappeared")
        })?
        .generation;
    let notices: Vec<_> = runtime(table, id, generation)?
        .controller
        .pending
        .iter()
        .filter(|pending| pending.kind == super::Kind::Notice)
        .map(|pending| pending.id.clone())
        .collect();
    for operation in notices {
        if crate::operations::managed_notice_note(table, id, &operation, now)? {
            let update = runtime(table, id, generation)?
                .controller
                .expire_notice(&operation)?;
            if let Err(error) = apply(table, id, generation, update) {
                let lost = runtime(table, id, generation)?.controller.disconnected();
                apply(table, id, generation, lost)?;
                return Err(error);
            }
        }
    }
    Ok(())
}
