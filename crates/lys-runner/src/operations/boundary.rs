//! A boundary signal cancels its fallback; expiry is named on the receipt.
use super::{BOUNDARY_WAIT, OperationState, unavailable};
use crate::error::RunnerError;
use crate::session::{Table, now_ms};
use std::sync::mpsc;
use std::time::Duration;

pub(super) fn wait_start(table: &Table, operation: &str) -> Option<u64> {
    let held = table.operations.get(operation)?;
    if held.request == "wake_up" {
        let compaction = super::wake::compaction(held)?;
        return table
            .operations
            .get(compaction)
            .filter(|compact| compact.state == OperationState::Confirmed)
            .map(|compact| compact.at);
    }
    Some(held.at)
}

pub(super) fn arm(table: &mut Table, operation: &str) -> Result<(), RunnerError> {
    if table.operations.boundary_waiters.contains_key(operation) {
        return Ok(());
    }
    let Some(start) = wait_start(table, operation) else {
        return Ok(());
    };
    let held = table
        .operations
        .get(operation)
        .ok_or_else(|| unavailable("boundary operation disappeared"))?;
    if held.state != OperationState::Accepted
        || table
            .sessions
            .get(&held.session)
            .is_none_or(|session| session.managed.is_none() && session.guard.tracking.is_none())
    {
        return Ok(());
    }
    let id = held.session.clone();
    let delay = BOUNDARY_WAIT.saturating_sub(Duration::from_millis(now_ms().saturating_sub(start)));
    let waited = Duration::from_millis(now_ms().saturating_sub(start));
    let armed = std::time::Instant::now();
    let owner = std::sync::Weak::clone(&table.owner);
    let operation_id = operation.to_owned();
    let (cancel, cancelled) = mpsc::channel();
    table
        .operations
        .boundary_waiters
        .insert(operation.to_owned(), cancel);
    let spawned = std::thread::Builder::new()
        .name("operation-boundary".to_owned())
        .spawn(move || {
            match cancelled.recv_timeout(delay) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let Some(sessions) = owner.upgrade() else {
                return;
            };
            let Some(mut table) = sessions.lock_logged() else {
                return;
            };
            table.operations.boundary_waiters.remove(&operation_id);
            table
                .operations
                .boundary_expired
                .insert(operation_id.clone(), (waited + armed.elapsed()).as_secs());
            if table
                .operations
                .get(&operation_id)
                .is_some_and(|held| held.state == OperationState::Accepted)
            {
                super::deliver(&mut table, &id);
            }
            drop(table);
            if let Err(error) = sessions.writer.barrier() {
                crate::error::said(&format!(
                    "session {id}: boundary fallback record failed: {error}"
                ));
            }
            sessions.wake();
        });
    match spawned {
        Ok(thread) => {
            drop(thread);
            Ok(())
        }
        Err(error) => {
            table.operations.boundary_waiters.remove(operation);
            Err(RunnerError::refused(
                "boundary_wait_unavailable",
                error.to_string(),
            ))
        }
    }
}

pub(super) fn note_at(table: &Table, id: &str, operation: &str, now: u64) -> Option<String> {
    let busy = table.sessions.get(id).is_some_and(|session| {
        (session.managed.is_some() || session.guard.tracking.is_some()) && !session.guard.idle
    });
    if !busy {
        return Some("being typed".to_owned());
    }
    let elapsed = match table.operations.boundary_expired.get(operation) {
        Some(seconds) => Duration::from_secs(*seconds),
        None => Duration::from_millis(now.saturating_sub(wait_start(table, operation)?)),
    };
    if elapsed < BOUNDARY_WAIT {
        return None;
    }
    Some(format!(
        "boundary_unsignalled wait_seconds={} boundary_wait={}s; being typed",
        elapsed.as_secs(),
        BOUNDARY_WAIT.as_secs()
    ))
}

pub(super) fn resume(table: &mut Table, session: &str) {
    let accepted = table
        .operations
        .accepted
        .get(session)
        .cloned()
        .unwrap_or_default();
    for operation in accepted {
        if let Err(error) = arm(table, &operation) {
            match table
                .operations
                .set(&operation, OperationState::Refused, error.to_string())
            {
                Ok(outcome) => super::feed(table, &outcome),
                Err(record_error) => crate::error::said(&format!(
                    "operation {operation}: {error}; refusal record failed: {record_error}"
                )),
            }
        }
    }
}
