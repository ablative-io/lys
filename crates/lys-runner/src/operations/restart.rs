//! Restart outcomes use the same durable operation identity and record.

use super::{OperationOutcome, OperationState, feed};
use crate::error::RunnerError;
use crate::protocol::Ended;
use crate::session::{Table, now_ms};

pub(crate) fn begin_restart(
    table: &mut Table,
    id: &str,
    operation: &str,
) -> Result<(OperationOutcome, bool), RunnerError> {
    if let Some(held) = table.operations.get(operation) {
        if held.session != id || held.request != "restart" {
            return Err(RunnerError::refused(
                "operation_reused",
                format!("operation {operation} already names another act"),
            ));
        }
        return Ok((held.clone(), false));
    }
    if table
        .operations
        .active
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|operation| table.operations.get(operation))
        .any(|held| {
            held.session == id
                && held.request == "restart"
                && held.state == OperationState::Delivering
        })
    {
        return Err(RunnerError::refused(
            "session_restarting",
            format!("session {id} is already restarting"),
        ));
    }
    let outcome = OperationOutcome {
        operation: operation.to_owned(),
        session: id.to_owned(),
        request: "restart".to_owned(),
        state: OperationState::Delivering,
        at: now_ms(),
        words: "waiting for the session's exit before restarting its held launch".to_owned(),
        text: None,
        ended: None,
    };
    table.operations.record(outcome.clone())?;
    feed(table, &outcome);
    Ok((outcome, true))
}

pub(crate) fn finish_restart(
    table: &mut Table,
    operation: &str,
    result: Result<Ended, RunnerError>,
) -> Result<OperationOutcome, RunnerError> {
    let mut held = table
        .operations
        .get(operation)
        .cloned()
        .ok_or_else(|| super::unavailable(format!("no restart {operation} is held")))?;
    match result {
        Ok(ended) => {
            held.state = OperationState::Confirmed;
            "the session exited and its held launch restarted with unchanged credentials"
                .clone_into(&mut held.words);
            held.ended = Some(ended);
        }
        Err(error) => {
            held.state = if matches!(error, RunnerError::State { .. }) {
                OperationState::Uncertain
            } else {
                OperationState::Refused
            };
            held.words = error.to_string();
        }
    }
    held.at = now_ms();
    let outcome = held.clone();
    table.operations.record(outcome.clone())?;
    feed(table, &outcome);
    Ok(outcome)
}
