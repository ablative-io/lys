//! How an operation goes on the table: accepted, typed at a boundary,
//! confirmed by what the harness says or by the session's end, and said in
//! the feed.

use super::{OperationOutcome, OperationState, unavailable};
use crate::error::RunnerError;
use crate::protocol::Ended;
use crate::session::{Table, now_ms};
use crate::tracking_store::{Body, Commit};
use std::sync::Weak;

#[path = "acceptance.rs"]
mod acceptance;
#[path = "delivery_completion.rs"]
mod completion;
pub(crate) use acceptance::accept;

#[path = "delivery_enter.rs"]
mod enter;
/// Claude Code and Codex detect paste bursts by byte arrival rate; keep Enter
/// outside their paste window after the terminal text has been flushed.
pub(super) const ENTER_AFTER_TEXT: std::time::Duration = std::time::Duration::from_millis(300);

/// Say `outcome` in the feed, naming in the log a feed that cannot take it:
/// the record of operations still answers it.
pub(super) fn feed(table: &mut Table, outcome: &OperationOutcome) {
    let appended = table.feed.append(
        &outcome.session,
        outcome.at,
        vec![Body::Operation(outcome.clone())],
        Commit::default(),
    );
    if let Err(error) = appended {
        let gap = table.gaps.entry(outcome.session.clone()).or_default();
        gap.lost += 1;
        gap.since.get_or_insert(outcome.at);
        gap.words = format!("an operation's outcome was not fed: {error}");
        crate::error::said(&format!(
            "operation {}: its outcome was not fed: {error}",
            outcome.operation
        ));
    }
}

pub(super) fn stop(
    table: &mut Table,
    id: &str,
    operation: &str,
) -> Result<OperationOutcome, RunnerError> {
    let Some(session) = table.sessions.get_mut(id) else {
        return Err(RunnerError::refused(
            "session_unknown",
            format!("no session {id} is held"),
        ));
    };
    session.ending = true;
    if let Some(live) = &session.live {
        live.kill()?;
    }
    let outcome = table.operations.set(
        operation,
        OperationState::Delivered,
        "the end was sent; it is confirmed when the exit is seen".to_owned(),
    )?;
    feed(table, &outcome);
    Ok(outcome)
}

/// Type the first accepted operation of session `id` that waits for a
/// boundary: `delivering` made durable first, `delivered` after.
pub(crate) fn deliver(table: &mut Table, id: &str) {
    deliver_at(table, id, now_ms());
}

pub(crate) fn deliver_at(table: &mut Table, id: &str, now: u64) {
    if table
        .sessions
        .get(id)
        .is_some_and(|session| session.managed.is_some())
    {
        if let Err(error) = crate::harness_control::events::expire_notices(table, id, now) {
            crate::error::said(&format!(
                "session {id}: managed boundary fallback failed: {error}"
            ));
        }
        return;
    }
    let Some(queue) = table.operations.accepted.get(id) else {
        return;
    };
    let eligible = |operation: &&String| {
        table
            .operations
            .get(operation)
            .is_some_and(|held| held.state == OperationState::Accepted)
            && super::wake::ready(table, operation)
    };
    let next = queue
        .iter()
        .filter(eligible)
        .find(|operation| {
            table
                .operations
                .get(operation)
                .is_some_and(|held| held.request == "wake_up")
        })
        .or_else(|| queue.iter().find(eligible))
        .cloned();
    let Some(operation) = next else {
        return;
    };
    let Some(mut words) = super::boundary::note_at(table, id, &operation, now) else {
        return;
    };
    if let Some(held) = table.operations.get(&operation)
        && let Some((note, _)) = held.words.split_once("; ")
        && held.request == "wake_up"
    {
        words = match words.split_once("; ") {
            Some((boundary, message)) => format!("{note} {boundary}; {message}"),
            None => format!("{note}; {words}"),
        };
    }
    if let Some(queue) = table.operations.accepted.get_mut(id) {
        queue.retain(|queued| queued != &operation);
    }
    let marked = table
        .operations
        .set(&operation, OperationState::Delivering, words);
    if let Ok(outcome) = &marked {
        feed(table, outcome);
    }
    if let Err(error) = marked {
        crate::error::said(&format!(
            "operation {operation} was not typed: its delivery could not be recorded first: {error}"
        ));
        finish(table, &operation, Err(error));
        return;
    }
    let submitted = enqueue(table, id, &operation);
    if let Err(error) = submitted {
        finish(table, &operation, Err(error));
    }
}

pub(super) fn enqueue(table: &mut Table, id: &str, operation: &str) -> Result<(), RunnerError> {
    let text = table.operations.texts.remove(operation).ok_or_else(|| {
        RunnerError::refused(
            "operation_text_missing",
            format!("operation {operation} has no text"),
        )
    })?;
    let session = table
        .sessions
        .get_mut(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", format!("no session {id}")))?;
    let writer = session.live(id)?.writer.clone();
    session.guard.idle = false;
    let owner = Weak::clone(&table.owner);
    let durable = owner
        .upgrade()
        .ok_or_else(|| RunnerError::refused("runner_stopping", "the runner ended before delivery"))?
        .writer
        .clone();
    let operation = operation.to_owned();
    let enter_writer = writer.clone();
    let empty = text.is_empty();
    writer.submit_after(text.into_bytes(), durable, move |result| {
        if let Err(error) = result {
            completion::record(&owner, &operation, Err(error));
            return;
        }
        enter::after_flush(owner, operation, enter_writer, empty);
    })
}

pub(super) fn finish(table: &mut Table, operation: &str, result: Result<(), RunnerError>) {
    if !table
        .operations
        .get(operation)
        .is_some_and(|outcome| outcome.state == OperationState::Delivering)
    {
        if let Err(error) = result {
            crate::error::said(&format!(
                "operation {operation}: input ended after its outcome changed: {error}"
            ));
        }
        return;
    }
    let compacting = table.operations.compacting.remove(operation);
    let (state, words) = match result {
        Ok(()) if compacting => (
            OperationState::Confirmed,
            "the harness said it is compacting".to_owned(),
        ),
        Ok(()) => (
            OperationState::Delivered,
            "typed into the session".to_owned(),
        ),
        Err(error) => (OperationState::Refused, error.to_string()),
    };
    match table.operations.set(operation, state, words) {
        Ok(outcome) => {
            feed(table, &outcome);
            if state == OperationState::Confirmed {
                super::wake::confirmed(table, operation);
            }
            if state == OperationState::Refused {
                super::wake::failed(table, operation);
            }
            super::boundary::resume(table, &outcome.session);
        }
        Err(error) => crate::error::said(&format!(
            "operation {operation}: its delivery was not recorded, and it stays uncertain: {error}"
        )),
    }
}

/// The harness says it is compacting: a delivered compaction is confirmed.
pub(crate) fn compacting(table: &mut Table, id: &str) {
    let delivered: Vec<(String, OperationState)> = table
        .operations
        .active
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|operation| table.operations.get(operation))
        .filter(|held| {
            held.session == id
                && held.request == "compact"
                && matches!(
                    held.state,
                    OperationState::Delivering | OperationState::Delivered
                )
        })
        .map(|held| (held.operation.clone(), held.state))
        .collect();
    for (operation, state) in delivered {
        if state == OperationState::Delivering {
            table.operations.compacting.insert(operation);
            continue;
        }
        match table.operations.set(
            &operation,
            OperationState::Confirmed,
            "the harness said it is compacting".to_owned(),
        ) {
            Ok(outcome) => {
                feed(table, &outcome);
                super::wake::confirmed(table, &operation);
            }
            Err(error) => crate::error::said(&format!("operation {operation}: {error}")),
        }
    }
}

/// Session `id` ended with `ended`: a sent stop is confirmed by it, what
/// was being typed is uncertain, and what waited is refused.
pub(crate) fn ended(table: &mut Table, id: &str, ended: &Ended) {
    let open: Vec<(String, OperationState, String)> = table
        .operations
        .active
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|operation| table.operations.get(operation))
        .filter(|held| held.session == id && held.request != "restart")
        .map(|held| (held.operation.clone(), held.state, held.request.clone()))
        .collect();
    for (operation, state, request) in open {
        let (next, words) = match (state, request.as_str()) {
            (OperationState::Delivered, "stop") => (
                OperationState::Confirmed,
                format!("the session's exit was seen at {} ms", ended.at),
            ),
            (OperationState::Accepted, _) => (
                OperationState::Refused,
                "session_ended: the session ended before its boundary came".to_owned(),
            ),
            (OperationState::Delivering, _)
                if table.operations.get(&operation).is_some_and(enter::waiting) =>
            {
                (
                    OperationState::Refused,
                    "session_unknown: the session ended before Enter was submitted".to_owned(),
                )
            }
            (OperationState::Delivering, _) => (
                OperationState::Uncertain,
                "the session ended while it was typed".to_owned(),
            ),
            _ => continue,
        };
        table.operations.texts.remove(&operation);
        table.operations.compacting.remove(&operation);
        let changed = table
            .operations
            .get(&operation)
            .cloned()
            .ok_or_else(|| unavailable(format!("no operation {operation} is held")))
            .and_then(|mut outcome| {
                outcome.state = next;
                outcome.at = now_ms();
                outcome.words = super::receipt_words(&outcome.words, words);
                if next == OperationState::Confirmed {
                    outcome.ended = Some(ended.clone());
                }
                table.operations.record(outcome.clone())?;
                Ok(outcome)
            });
        match changed {
            Ok(outcome) => {
                feed(table, &outcome);
            }
            Err(error) => crate::error::said(&format!("operation {operation}: {error}")),
        }
    }
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
