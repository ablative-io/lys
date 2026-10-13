//! One reader projects managed evidence into the existing feed and receipts.

#[path = "events/notices_delivery.rs"]
mod notices;
mod reader;
mod writes;
pub(crate) use notices::expire as expire_notices;

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak, mpsc};

use super::{
    Binding, Controller, Kind, Pending, Transport, Update, claude, codex, process, unsupported,
};
use crate::error::RunnerError;
use crate::input::Input;
use crate::operations::{OperationRequest, OperationState};
use crate::session::{Sessions, Table, now_ms};
use crate::tracking_store::{Body, Commit};

pub(crate) struct Runtime {
    pub(crate) controller: Controller,
    pipe: Input,
    admissions: BTreeMap<String, mpsc::Sender<Result<(), RunnerError>>>,
    pub(super) left: Arc<AtomicBool>,
    pub(super) approval_active: bool,
    owner: Weak<Sessions>,
}

pub(crate) fn live(
    binding: Binding,
    transport: Transport,
    writer: Box<dyn Write + Send>,
    owner: Weak<Sessions>,
) -> Result<(Runtime, Input), RunnerError> {
    let bridge = Human {
        owner: Weak::clone(&owner),
        session: binding.session.clone(),
        generation: binding.generation,
    };
    let mut controller = Controller::new(binding, transport)?;
    controller.require_boundary_authority()?;
    let runtime = Runtime {
        controller,
        pipe: Input::new(writer),
        admissions: BTreeMap::new(),
        left: Arc::new(AtomicBool::new(false)),
        approval_active: false,
        owner,
    };
    Ok((runtime, Input::new(Box::new(bridge))))
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.left.store(true, Ordering::SeqCst);
        if let Some(owner) = self.owner.upgrade() {
            owner.wake();
        }
    }
}

struct Human {
    owner: Weak<Sessions>,
    session: String,
    generation: u64,
}

impl Write for Human {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let sessions = self
            .owner
            .upgrade()
            .ok_or_else(|| std::io::Error::other("runner_stopping"))?;
        sessions
            .managed_input(&self.session, self.generation, bytes)
            .map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn runtime<'a>(
    table: &'a mut Table,
    id: &str,
    generation: u64,
) -> Result<&'a mut Runtime, RunnerError> {
    let session = table
        .sessions
        .get_mut(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", "managed session is not held"))?;
    if session.generation != generation || session.ended.is_some() {
        return Err(RunnerError::refused(
            "control_generation_changed",
            "managed process generation ended or changed",
        ));
    }
    session
        .live
        .as_mut()
        .and_then(|live| live.control.as_mut())
        .ok_or_else(|| {
            RunnerError::refused(
                "control_transport_unsupported",
                "session has no live managed pipe",
            )
        })
}

pub(crate) fn accepts(
    table: &mut Table,
    id: &str,
    request: &OperationRequest,
) -> Result<(), RunnerError> {
    if *request == OperationRequest::Stop {
        return Ok(());
    }
    let session = table
        .sessions
        .get(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", "session is not held"))?;
    if session.managed.is_none() {
        return Ok(());
    }
    let generation = session.generation;
    if session.ending {
        return Err(RunnerError::refused(
            "session_ending",
            "managed session is ending",
        ));
    }
    if !runtime(table, id, generation)?.controller.ready() {
        return Err(RunnerError::refused(
            "control_source_unbound",
            "managed conversation has not been proved",
        ));
    }
    Ok(())
}

pub(crate) fn enqueue_operation(
    table: &mut Table,
    id: &str,
    operation: &str,
    request: &OperationRequest,
) -> Result<(), RunnerError> {
    let generation = table
        .sessions
        .get(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", "session is not held"))?
        .generation;
    let kind = if matches!(
        request,
        OperationRequest::Compact { .. } | OperationRequest::ContextCompact { .. }
    ) {
        Kind::Compact
    } else if matches!(request, OperationRequest::Notice { .. }) {
        Kind::Notice
    } else {
        Kind::Reminder
    };
    let text = crate::operations::managed_take(table, id, operation)?;
    let pending = match request {
        OperationRequest::GoalReminder { reference, .. } => {
            Pending::for_goal(operation.to_owned(), text, reference.clone())
        }
        _ => Pending::new(operation.to_owned(), kind, text),
    };
    let control = &mut runtime(table, id, generation)?.controller;
    let update = if let OperationRequest::ContextCompact { crossing, .. } = request {
        if crossing != operation {
            return Err(RunnerError::refused(
                "control_crossing_changed",
                "the crossing does not name its operation",
            ));
        }
        control.context_compact(pending)?
    } else {
        control.enqueue(pending)?
    };
    match apply(table, id, generation, update) {
        Ok(()) => Ok(()),
        Err(error) => {
            let lost = runtime(table, id, generation)?.controller.disconnected();
            apply(table, id, generation, lost)?;
            Err(error)
        }
    }
}

pub(super) fn apply(
    table: &mut Table,
    id: &str,
    generation: u64,
    update: Update,
) -> Result<(), RunnerError> {
    for (operation, binding, uuid, turn) in update.admissions {
        if table.operations.get(&operation).is_some() {
            table
                .operations
                .observed(&operation, &binding, &uuid, turn)?;
            writes::admitted(table, &operation)?;
        }
    }
    if !update.events.is_empty() {
        table.feed.append(
            id,
            now_ms(),
            update.events.into_iter().map(Body::Managed).collect(),
            Commit::default(),
        )?;
    }
    for receipt in update.receipts {
        if table.operations.get(&receipt.operation).is_some() {
            crate::operations::managed_state(
                table,
                &receipt.operation,
                receipt.state,
                receipt.reason,
            )?;
        } else if let Some(sender) = runtime(table, id, generation)?
            .admissions
            .remove(&receipt.operation)
        {
            let result = if receipt.state == OperationState::Confirmed {
                Ok(())
            } else {
                Err(RunnerError::refused(
                    "control_input_uncertain",
                    receipt.reason,
                ))
            };
            sender.send(result).map_err(|error| {
                RunnerError::refused(
                    "control_input_caller_left",
                    format!("input admission was not received: {error}"),
                )
            })?;
        }
    }
    for dispatch in update.dispatches {
        if table.sessions.get(id).is_some_and(|session| session.ending) {
            if table.operations.get(&dispatch.operation).is_some() {
                crate::operations::managed_state(
                    table,
                    &dispatch.operation,
                    OperationState::Refused,
                    "session_ending: prepared input was not written".to_owned(),
                )?;
            } else if let Some(sender) = runtime(table, id, generation)?
                .admissions
                .remove(&dispatch.operation)
            {
                sender
                    .send(Err(RunnerError::refused(
                        "session_ending",
                        "prepared input was not written",
                    )))
                    .map_err(|error| {
                        RunnerError::refused(
                            "control_input_caller_left",
                            format!("cancelled input was not received: {error}"),
                        )
                    })?;
            }
            let mut lost = runtime(table, id, generation)?.controller.disconnected();
            lost.receipts
                .retain(|receipt| receipt.operation != dispatch.operation);
            apply(table, id, generation, lost)?;
            continue;
        }
        if !dispatch.operation.is_empty() && table.operations.get(&dispatch.operation).is_some() {
            let (prepared, text) = runtime(table, id, generation)?
                .controller
                .preparation(&dispatch)?;
            table
                .operations
                .prepare(&dispatch.operation, prepared, text)?;
            table.operations.arm(&dispatch.operation)?;
            if let Some(outcome) = table
                .operations
                .get(&dispatch.operation)
                .filter(|held| held.request == "notice")
                .cloned()
            {
                crate::operations::managed_feed(table, &outcome);
            }
        }
        let mut bytes = serde_json::to_vec(&dispatch.frame)
            .map_err(|error| RunnerError::refused("control_frame_invalid", error.to_string()))?;
        bytes.push(b'\n');
        let pipe = runtime(table, id, generation)?.pipe.clone();
        let owner = Weak::clone(&table.owner);
        let sessions = owner.upgrade().ok_or_else(|| {
            RunnerError::refused("runner_stopping", "runner ended before managed delivery")
        })?;
        let durable = sessions.writer.clone();
        let operation = dispatch.operation;
        let session_id = id.to_owned();
        pipe.submit_after(bytes, durable, move |result| {
            let Some(sessions) = owner.upgrade() else {
                crate::error::said("control_transport_lost: runner ended during a pipe write");
                return;
            };
            if let Err(error) =
                sessions.managed_written(&session_id, generation, &operation, result)
            {
                crate::error::said(&format!("control_delivery_uncertain: {error}"));
            }
        })?;
    }
    let idle = runtime(table, id, generation)?.controller.idle();
    if let Some(session) = table.sessions.get_mut(id) {
        session.guard.idle = idle;
        if let (Some(settings), Some(control)) = (
            &mut session.managed,
            session.live.as_ref().and_then(|live| live.control.as_ref()),
        ) {
            if settings.conversation != control.controller.binding.conversation {
                settings
                    .conversation
                    .clone_from(&control.controller.binding.conversation);
            }
        }
    }
    Ok(())
}

impl Sessions {
    fn managed_input(&self, id: &str, generation: u64, bytes: &[u8]) -> Result<(), RunnerError> {
        let text = std::str::from_utf8(bytes).map_err(|error| {
            RunnerError::refused(
                "control_input_invalid",
                format!("managed input must be UTF-8 text: {error}"),
            )
        })?;
        let text = text.strip_suffix('\r').unwrap_or(text);
        if text
            .chars()
            .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
        {
            return Err(RunnerError::refused(
                "control_input_invalid",
                "terminal control bytes are not a managed message",
            ));
        }
        let operation = format!(
            "human-{}",
            crate::protocol::hex(&rand::random::<[u8; 16]>())
        );
        let (sender, admission) = mpsc::channel();
        let mut table = self.lock()?;
        if table.sessions.get(id).is_some_and(|session| session.ending) {
            return Err(RunnerError::refused(
                "session_ending",
                "managed session is ending",
            ));
        }
        let held = runtime(&mut table, id, generation)?;
        let update = held.controller.enqueue(Pending::new(
            operation.clone(),
            Kind::Human,
            text.to_owned(),
        ))?;
        held.admissions.insert(operation, sender);
        let applied = apply(&mut table, id, generation, update);
        drop(table);
        if let Err(error) = applied {
            self.managed_lost(id, generation, &error.to_string())?;
            return Err(error);
        }
        self.writer.barrier()?;
        self.wake();
        admission.recv().map_err(|error| {
            RunnerError::refused(
                "control_input_uncertain",
                format!("managed admission channel ended: {error}"),
            )
        })?
    }

    pub(crate) fn managed_lost(
        &self,
        id: &str,
        generation: u64,
        reason: &str,
    ) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let held = runtime(&mut table, id, generation)?;
        let leader = held.controller.binding.leader.clone();
        held.left.store(true, Ordering::SeqCst);
        let update = held.controller.disconnected();
        let admissions = std::mem::take(&mut held.admissions);
        let applied = apply(&mut table, id, generation, update);
        if let Some(session) = table.sessions.get_mut(id) {
            session.ending = true;
        }
        drop(table);
        for sender in admissions.into_values() {
            if sender
                .send(Err(RunnerError::refused("control_transport_lost", reason)))
                .is_err()
            {
                crate::error::said("control_input_caller_left: transport loss was not received");
            }
        }
        let ended = lost_cleanup(crate::pty::end_left_group(&leader));
        if let Ok(Some(diagnostic)) = &ended {
            crate::error::said(&format!("session {id}: {diagnostic}"));
        }
        let durable = self.writer.barrier();
        self.wake();
        if let Err(error) = &applied {
            crate::error::said(&format!("control_loss_record_failed: {error}"));
        }
        if let Err(error) = &durable {
            crate::error::said(&format!("control_loss_fence_failed: {error}"));
        }
        ended?;
        applied?;
        durable?;
        Ok(())
    }

    pub(crate) fn managed_bootstrap(&self, id: &str, generation: u64) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let held = runtime(&mut table, id, generation)?;
        let update = held.controller.bootstrap();
        apply(&mut table, id, generation, update)
    }
}

#[cfg(test)]
#[path = "events/tests.rs"]
mod tests;

fn lost_cleanup(
    left: Result<crate::pty::Left, RunnerError>,
) -> Result<Option<String>, RunnerError> {
    match left {
        Ok(crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None }) => Ok(None),
        Ok(crate::pty::Left::Ended {
            reason: Some(reason),
        }) => Ok(Some(format!("control_group_cleanup_proved: {reason}"))),
        Ok(left) => Err(RunnerError::refused(
            "control_process_end_unconfirmed",
            format!("lost managed process group could not be ended: {left:?}"),
        )),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "events/cleanup_tests.rs"]
mod cleanup_tests;
