//! Only a harness boundary permits delivery into a busy session.
use crate::session::Table;

#[cfg(test)]
#[path = "boundary_tests.rs"]
mod tests;

pub(super) fn note(table: &Table, id: &str) -> Option<String> {
    let session = table.sessions.get(id)?;
    let busy = {
        (session.managed.is_some()
            || session.guard.tracking.is_some()
            || session.guard.boundary_observed)
            && !session.guard.idle
    };
    (!busy && !session.ending && session.ended.is_none()).then(|| "being typed".to_owned())
}
