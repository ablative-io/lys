//! How an operation goes on the table: accepted, typed at a boundary,
//! confirmed by what the harness says or by the session's end, and said in
//! the feed.

use super::{
    Operation, OperationOutcome, OperationRequest, OperationState, TextDigest, unavailable,
};
use crate::error::RunnerError;
use crate::protocol::{Ended, Key};
use crate::session::{Table, now_ms};
use crate::tracking_store::{Body, Commit};
use std::collections::VecDeque;
use std::sync::Weak;

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

/// Accept `operation` on the table, answering how it stands.
pub(crate) fn accept(
    table: &mut Table,
    operation: Operation,
) -> Result<OperationOutcome, RunnerError> {
    table.operations.prune(now_ms());
    if let Some(held) = table.operations.get(&operation.operation) {
        let same = held.session == operation.session
            && held.request == operation.request.name()
            && match table.operations.original_request(&operation.operation) {
                Some(original) => original == &operation.request.identity()?,
                None => {
                    table.operations.original_text(&operation.operation)
                        == operation.request.text().map(TextDigest::of).as_ref()
                }
            };
        if !same {
            return Err(RunnerError::refused(
                "operation_reused",
                format!(
                    "operation {} already names another act",
                    operation.operation
                ),
            ));
        }
        return Ok(held.clone());
    }
    table.operations.repeated(&operation.operation)?;
    let session = table.sessions.get(&operation.session).ok_or_else(|| {
        RunnerError::refused(
            "session_unknown",
            format!("no session {} is held", operation.session),
        )
    })?;
    if session.ended.is_none() {
        crate::harness_control::events::accepts(table, &operation.session, &operation.request)?;
    }
    let session = table
        .sessions
        .get(&operation.session)
        .ok_or_else(|| RunnerError::refused("session_unknown", "session is not held"))?;
    let (state, words) = if session.ended.is_some() {
        (
            OperationState::Refused,
            "session_ended: the session has ended and does nothing more".to_owned(),
        )
    } else {
        (OperationState::Accepted, "accepted".to_owned())
    };
    let outcome = OperationOutcome {
        operation: operation.operation.clone(),
        session: operation.session.clone(),
        request: operation.request.name().to_owned(),
        state,
        at: now_ms(),
        words,
        text: operation.request.text().map(TextDigest::of),
        ended: None,
    };
    table
        .operations
        .record_request(outcome.clone(), operation.request.identity()?)?;
    feed(table, &outcome);
    if state == OperationState::Refused {
        return Ok(outcome);
    }
    if let Some(text) = operation.request.text() {
        table
            .operations
            .texts
            .insert(operation.operation.clone(), text.to_owned());
    }
    let id = operation.session;
    if table
        .sessions
        .get(&id)
        .is_some_and(|session| session.managed.is_some())
        && operation.request != OperationRequest::Stop
    {
        if let Err(error) = crate::harness_control::events::enqueue_operation(
            table,
            &id,
            &operation.operation,
            &operation.request,
        ) {
            let refused = table.operations.set(
                &operation.operation,
                OperationState::Refused,
                error.to_string(),
            )?;
            feed(table, &refused);
            return Ok(refused);
        }
        return table
            .operations
            .get(&operation.operation)
            .cloned()
            .ok_or_else(|| unavailable("managed operation record disappeared"));
    }
    if operation.request == OperationRequest::Stop {
        return Ok(outcome);
    }
    let waits = table
        .sessions
        .get(&id)
        .is_some_and(|session| session.guard.tracking.is_some() && !session.guard.idle);
    if !waits {
        deliver(table, &id);
    }
    Ok(table
        .operations
        .get(&operation.operation)
        .cloned()
        .unwrap_or(outcome))
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
    let next = table
        .operations
        .accepted
        .get_mut(id)
        .and_then(VecDeque::pop_front);
    let Some(operation) = next else {
        return;
    };
    let marked = table.operations.set(
        &operation,
        OperationState::Delivering,
        "being typed".to_owned(),
    );
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
    let mut bytes = text.into_bytes();
    bytes.extend_from_slice(Key::Enter.bytes());
    let owner = Weak::clone(&table.owner);
    let durable = owner
        .upgrade()
        .ok_or_else(|| RunnerError::refused("runner_stopping", "the runner ended before delivery"))?
        .writer
        .clone();
    let operation = operation.to_owned();
    writer.submit_after(bytes, durable, move |result| {
        let Some(sessions) = owner.upgrade() else {
            crate::error::said(&format!(
                "operation {operation}: input completed after the runner ended"
            ));
            return;
        };
        let Some(mut table) = sessions.lock_logged() else {
            return;
        };
        finish(&mut table, &operation, result);
        drop(table);
        if let Err(error) = sessions.writer.barrier() {
            crate::error::said(&format!(
                "operation {operation}: delivery_record_uncertain: {error}"
            ));
        }
        sessions.wake();
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
        Ok(outcome) => feed(table, &outcome),
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
            Ok(outcome) => feed(table, &outcome),
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
                outcome.words = words;
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
