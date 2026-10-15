//! A completed paste flush permits its submit key, with no clock involved.

use super::super::{OperationOutcome, OperationState};
use crate::error::RunnerError;
use crate::input::Input;
use crate::protocol::Key;
use crate::session::Sessions;
use std::sync::Weak;
#[cfg(test)]
use std::sync::{OnceLock, mpsc};

const WAITING: &str = "awaiting Enter after text flush";

struct Pending {
    owner: Weak<Sessions>,
    operation: String,
    writer: Input,
}

pub(super) fn waiting(held: &OperationOutcome) -> bool {
    held.state == OperationState::Delivering && held.words.ends_with(WAITING)
}

pub(super) fn after_flush(owner: Weak<Sessions>, operation: String, writer: Input, empty: bool) {
    let pending = Pending {
        owner,
        operation,
        writer,
    };
    if empty {
        send(&pending);
        return;
    }
    if let Err(error) = arm(&pending) {
        super::completion::record(&pending.owner, &pending.operation, Err(error));
        return;
    }
    send(&pending);
}

fn arm(pending: &Pending) -> Result<(), RunnerError> {
    let sessions = pending
        .owner
        .upgrade()
        .ok_or_else(|| RunnerError::refused("runner_stopping", "the runner ended before Enter"))?;
    let mut table = sessions.lock()?;
    if table
        .operations
        .get(&pending.operation)
        .is_some_and(|held| held.state == OperationState::Delivering)
    {
        let outcome = table.operations.set(
            &pending.operation,
            OperationState::Delivering,
            WAITING.to_owned(),
        )?;
        super::feed(&mut table, &outcome);
    }
    drop(table);
    sessions.wake();
    #[cfg(test)]
    pause_if_held(&pending.operation)?;
    Ok(())
}

fn send(pending: &Pending) {
    let result = submit(pending);
    if let Err(error) = result {
        super::completion::record(&pending.owner, &pending.operation, Err(error));
    }
}

fn submit(pending: &Pending) -> Result<(), RunnerError> {
    let sessions = pending
        .owner
        .upgrade()
        .ok_or_else(|| RunnerError::refused("runner_stopping", "the runner ended before Enter"))?;
    let mut table = sessions.lock()?;
    let held = table.operations.get(&pending.operation).ok_or_else(|| {
        RunnerError::refused(
            "operation_unknown",
            "the pending Enter operation disappeared",
        )
    })?;
    if held.state != OperationState::Delivering {
        return Ok(());
    }
    let session = table
        .sessions
        .get(&held.session)
        .ok_or_else(|| RunnerError::refused("session_unknown", "the session ended before Enter"))?;
    if session.ended.is_some() || session.live.is_none() {
        return Err(RunnerError::refused(
            "session_unknown",
            "the session ended before Enter",
        ));
    }
    if session.ending {
        return Err(RunnerError::refused(
            "runner_stopping",
            "the session is stopping before Enter",
        ));
    }
    // Once submitted, an exit makes delivery uncertain again: Enter may have
    // reached the harness even if its completion has not been recorded yet.
    let outcome = table.operations.set(
        &pending.operation,
        OperationState::Delivering,
        "submitting Enter".to_owned(),
    )?;
    super::feed(&mut table, &outcome);
    let owner = Weak::clone(&pending.owner);
    let operation = pending.operation.clone();
    pending
        .writer
        .submit(Key::Enter.bytes().to_vec(), move |result| {
            super::completion::record(&owner, &operation, result);
        })
}

#[cfg(test)]
struct Pause {
    ready: mpsc::Sender<()>,
    resume: mpsc::Receiver<()>,
}

#[cfg(test)]
fn pauses() -> &'static std::sync::Mutex<std::collections::BTreeMap<String, Pause>> {
    static PAUSES: OnceLock<std::sync::Mutex<std::collections::BTreeMap<String, Pause>>> =
        OnceLock::new();
    PAUSES.get_or_init(std::sync::Mutex::default)
}

#[cfg(test)]
pub(super) fn pause_gap(
    operation: &str,
) -> Result<(mpsc::Receiver<()>, mpsc::Sender<()>), RunnerError> {
    let (ready, reached) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    let mut pauses = pauses()
        .lock()
        .map_err(|error| RunnerError::refused("enter_probe_poisoned", error.to_string()))?;
    if pauses.contains_key(operation) {
        return Err(RunnerError::refused(
            "enter_probe_reused",
            "the gap already has a probe",
        ));
    }
    pauses.insert(
        operation.to_owned(),
        Pause {
            ready,
            resume: resumed,
        },
    );
    Ok((reached, resume))
}

#[cfg(test)]
fn pause_if_held(operation: &str) -> Result<(), RunnerError> {
    let pause = pauses()
        .lock()
        .map_err(|error| RunnerError::refused("enter_probe_poisoned", error.to_string()))?
        .remove(operation);
    if let Some(pause) = pause {
        pause
            .ready
            .send(())
            .map_err(|error| RunnerError::refused("enter_probe_gone", error.to_string()))?;
        pause
            .resume
            .recv()
            .map_err(|error| RunnerError::refused("enter_probe_gone", error.to_string()))?;
    }
    Ok(())
}
