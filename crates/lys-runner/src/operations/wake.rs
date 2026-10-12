//! Confirmation releases exactly the wake occurrence that names its compaction.
use super::{OperationOutcome, OperationState, feed};
use crate::session::Table;
use crate::tracking::Harness;

pub(super) fn compaction(held: &OperationOutcome) -> Option<&str> {
    held.words
        .strip_prefix("wake_up compaction=")?
        .split([';', ' '])
        .next()
}

pub(super) fn ready(table: &Table, operation: &str) -> bool {
    let Some(held) = table.operations.get(operation) else {
        return false;
    };
    held.request != "wake_up"
        || compaction(held)
            .and_then(|id| table.operations.get(id))
            .is_some_and(|compact| {
                compact.session == held.session && compact.state == OperationState::Confirmed
            })
}

pub(super) fn confirmed(table: &mut Table, compaction_id: &str) {
    let operation = format!("wake-up-{compaction_id}");
    if !table.operations.get(&operation).is_some_and(|held| {
        held.state == OperationState::Accepted && compaction(held) == Some(compaction_id)
    }) {
        return;
    }
    if let Err(error) = super::boundary::arm(table, &operation) {
        match table
            .operations
            .set(&operation, OperationState::Refused, error.to_string())
        {
            Ok(outcome) => feed(table, &outcome),
            Err(record_error) => crate::error::said(&format!(
                "operation {operation}: {error}; refusal record failed: {record_error}"
            )),
        }
    }
}

/// Confirm a delivered Codex compaction at its next agent turn's end.
pub(crate) fn confirm_on_turn_end(table: &mut Table, id: &str) {
    if table.sessions.get(id).is_none_or(|session| {
        session.managed.is_some()
            || session
                .guard
                .tracking
                .as_ref()
                .is_none_or(|tracking| tracking.harness != Harness::Codex)
    }) {
        return;
    }
    let compactions: Vec<_> = table
        .operations
        .active
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|operation| table.operations.get(operation))
        .filter(|held| held.request == "compact" && held.state == OperationState::Delivered)
        .map(|held| held.operation.clone())
        .collect();
    for operation in compactions {
        match table.operations.set(
            &operation,
            OperationState::Confirmed,
            "agent-turn-complete: the turn after compaction ended".to_owned(),
        ) {
            Ok(outcome) => {
                feed(table, &outcome);
                confirmed(table, &operation);
            }
            Err(error) => crate::error::said(&format!(
                "operation {operation}: confirmation record failed: {error}"
            )),
        }
    }
}

pub(super) fn failed(table: &mut Table, compaction_id: &str) {
    let operation = format!("wake-up-{compaction_id}");
    if !table.operations.get(&operation).is_some_and(|held| {
        held.state == OperationState::Accepted && compaction(held) == Some(compaction_id)
    }) {
        return;
    }
    match table.operations.set(
        &operation,
        OperationState::Refused,
        format!("compaction_unconfirmed: {compaction_id} failed before confirmation"),
    ) {
        Ok(outcome) => feed(table, &outcome),
        Err(error) => crate::error::said(&format!(
            "operation {operation}: compaction failure record failed: {error}"
        )),
    }
}
