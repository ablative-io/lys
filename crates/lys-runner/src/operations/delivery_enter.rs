//! One shared timer queue separates terminal text from its submit key.

use super::super::{OperationOutcome, OperationState};
use super::ENTER_AFTER_TEXT;
use crate::error::RunnerError;
use crate::input::Input;
use crate::protocol::Key;
use crate::session::Sessions;
use std::sync::{OnceLock, Weak, mpsc};
use std::time::Instant;

const WAITING: &str = "awaiting Enter after text flush";

struct Pending {
    owner: Weak<Sessions>,
    operation: String,
    writer: Input,
    due: Instant,
}

pub(super) fn waiting(held: &OperationOutcome) -> bool {
    held.state == OperationState::Delivering && held.words.ends_with(WAITING)
}

pub(super) fn after_flush(owner: Weak<Sessions>, operation: String, writer: Input, empty: bool) {
    let pending = Pending {
        owner,
        operation,
        writer,
        due: Instant::now() + ENTER_AFTER_TEXT,
    };
    if empty {
        send(&pending);
        return;
    }
    if let Err(error) = arm(&pending) {
        super::completion::record(&pending.owner, &pending.operation, Err(error));
        return;
    }
    let owner = Weak::clone(&pending.owner);
    let operation = pending.operation.clone();
    if let Err(error) = scheduler().and_then(|queue| {
        queue
            .send(pending)
            .map_err(|error| RunnerError::refused("enter_timer_unavailable", error.to_string()))
    }) {
        super::completion::record(&owner, &operation, Err(error));
    }
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

fn scheduler() -> Result<&'static mpsc::Sender<Pending>, RunnerError> {
    static QUEUE: OnceLock<Result<mpsc::Sender<Pending>, String>> = OnceLock::new();
    QUEUE
        .get_or_init(|| {
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("operation-enter".to_owned())
                .spawn(move || run(&receiver))
                .map(|worker| {
                    drop(worker);
                    sender
                })
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| RunnerError::refused("enter_timer_unavailable", error.clone()))
}

fn run(receiver: &mpsc::Receiver<Pending>) {
    let mut pending: Vec<Pending> = Vec::new();
    loop {
        let next = pending
            .iter()
            .enumerate()
            .min_by_key(|(_, held)| held.due)
            .map(|(index, _)| index);
        let received = if let Some(index) = next {
            let delay = pending[index].due.saturating_duration_since(Instant::now());
            if delay.is_zero() {
                send(&pending.swap_remove(index));
                continue;
            }
            match receiver.recv_timeout(delay) {
                Ok(held) => Some(held),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    send(&pending.swap_remove(index));
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => None,
            }
        } else {
            match receiver.recv() {
                Ok(held) => Some(held),
                Err(mpsc::RecvError) => {
                    crate::error::said("runner_stopping: the Enter scheduler queue disconnected");
                    None
                }
            }
        };
        match received {
            Some(held) => pending.push(held),
            None => {
                for held in pending {
                    super::completion::record(
                        &held.owner,
                        &held.operation,
                        Err(RunnerError::refused(
                            "runner_stopping",
                            "the Enter scheduler ended",
                        )),
                    );
                }
                return;
            }
        }
    }
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
