use std::sync::Weak;

use crate::error::RunnerError;
use crate::session::Sessions;

pub(super) fn record(owner: &Weak<Sessions>, operation: &str, result: Result<(), RunnerError>) {
    let Some(sessions) = owner.upgrade() else {
        crate::error::said(&format!(
            "operation {operation}: input completed after the runner ended"
        ));
        return;
    };
    let Some(mut table) = sessions.lock_logged() else {
        return;
    };
    super::finish(&mut table, operation, result);
    drop(table);
    if let Err(error) = sessions.writer.barrier() {
        crate::error::said(&format!(
            "operation {operation}: delivery_record_uncertain: {error}"
        ));
    }
    sessions.wake();
}
