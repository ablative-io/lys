use super::{deliver, feed};
use crate::error::RunnerError;
use crate::operations::{
    Operation, OperationOutcome, OperationRequest, OperationState, TextDigest, unavailable,
};
use crate::session::{Table, now_ms};

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
    if let OperationRequest::WakeUp { compaction, .. } = &operation.request {
        if operation.operation != format!("wake-up-{compaction}")
            || compaction == &operation.operation
        {
            return Err(RunnerError::refused(
                "wake_up_identity_invalid",
                "wake occurrence must name its compaction exactly",
            ));
        }
        if session.managed.is_some() {
            return Err(RunnerError::refused(
                "control_transport_unsupported",
                "this wake occurrence is for a terminal session",
            ));
        }
        if table
            .operations
            .get(compaction)
            .is_some_and(|held| held.session != operation.session || held.request != "compact")
        {
            return Err(RunnerError::refused(
                "wake_up_compaction_invalid",
                "wake occurrence names another session or request",
            ));
        }
    }
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
        let words = match &operation.request {
            OperationRequest::WakeUp { compaction, .. } => {
                format!("wake_up compaction={compaction}; accepted")
            }
            _ if (session.guard.tracking.is_some() || session.guard.boundary_observed)
                && !session.guard.idle
                && operation.request != OperationRequest::Stop =>
            {
                "accepted awaiting turn boundary".to_owned()
            }
            _ => "accepted".to_owned(),
        };
        (OperationState::Accepted, words)
    };
    let words = match &operation.request {
        OperationRequest::WakeUp { compaction, .. } if !words.starts_with("wake_up ") => {
            format!("wake_up compaction={compaction}; {words}")
        }
        _ => words,
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
    deliver(table, &id);
    table
        .operations
        .get(&operation.operation)
        .cloned()
        .ok_or_else(|| unavailable("accepted operation record disappeared"))
}
