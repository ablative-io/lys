//! Expired notices keep the active turn and its correlations intact.
use super::{
    Boundary, Controller, Dispatch, Kind, Observation, Pending, Transport, Update, receipt,
};
use crate::error::RunnerError;
use crate::operations::OperationState;
use serde_json::Value;

#[derive(Debug)]
pub(super) struct Written {
    pub(super) pending: Pending,
    pub(super) admitted: bool,
    turn: Option<String>,
}

impl Controller {
    pub(super) fn expire_notice(&mut self, operation: &str) -> Result<Update, RunnerError> {
        if !self.ready || self.closed {
            return Err(RunnerError::refused(
                "control_transport_lost",
                "the notice channel ended",
            ));
        }
        let position = self
            .pending
            .iter()
            .position(|pending| pending.id == operation && pending.kind == Kind::Notice)
            .ok_or_else(|| {
                RunnerError::refused(
                    "operation_unknown",
                    "no queued notice names the expired operation",
                )
            })?;
        let pending = self.pending.remove(position).ok_or_else(|| {
            RunnerError::refused("operation_unknown", "the queued notice disappeared")
        })?;
        self.pending_ids.remove(operation);
        let frame = match self.transport {
            Transport::Claude => super::claude::request(&self.binding.conversation, &pending),
            Transport::Codex => super::codex::request(&self.binding.conversation, &pending),
            Transport::Pty => return Err(super::unsupported()),
        };
        let mut update = Update::default();
        update
            .events
            .push(self.event("control_prepared", None, Some(pending.id.clone())));
        update.dispatches.push(Dispatch {
            operation: pending.id.clone(),
            frame,
        });
        self.early_notices.push_back(Written {
            pending,
            admitted: false,
            turn: None,
        });
        Ok(update)
    }

    pub(super) fn observe_early_notice(
        &mut self,
        value: &Value,
    ) -> Result<Option<Update>, RunnerError> {
        let position = self
            .early_notices
            .iter()
            .position(|written| match self.transport {
                Transport::Claude => {
                    value.get("type").and_then(Value::as_str) == Some("user")
                        && value.get("uuid").and_then(Value::as_str) == Some(&written.pending.uuid)
                }
                Transport::Codex => {
                    value.get("id").and_then(Value::as_str) == Some(&written.pending.id)
                }
                Transport::Pty => false,
            });
        let Some(position) = position else {
            return Ok(None);
        };
        let written = self.early_notices.get_mut(position).ok_or_else(|| {
            RunnerError::refused("operation_unknown", "written notice disappeared")
        })?;
        let observation = match self.transport {
            Transport::Claude => super::claude::observe(
                &self.binding.conversation,
                &self.binding.harness_version,
                value,
                Some(&written.pending),
            )?,
            Transport::Codex => {
                super::codex::observe(&self.binding.conversation, value, Some(&written.pending))?
            }
            Transport::Pty => return Err(super::unsupported()),
        };
        let mut update = Update::default();
        match observation {
            Observation::Admitted { uuid, turn } if !written.admitted => {
                written.admitted = true;
                written.turn.clone_from(&turn);
                update.admissions.push((
                    written.pending.id.clone(),
                    self.binding.clone(),
                    uuid,
                    turn,
                ));
                update.receipts.push(receipt(
                    &written.pending.id,
                    OperationState::Confirmed,
                    "harness_admitted",
                ));
            }
            Observation::Refused => {
                let written = self.early_notices.remove(position).ok_or_else(|| {
                    RunnerError::refused("operation_unknown", "refused notice disappeared")
                })?;
                update.receipts.push(receipt(
                    &written.pending.id,
                    OperationState::Refused,
                    "harness_refused",
                ));
            }
            _ => {}
        }
        Ok(Some(update))
    }

    pub(super) fn promote_notice(&mut self) -> bool {
        let Some(written) = self.early_notices.pop_front() else {
            return false;
        };
        self.boundary = written
            .turn
            .as_ref()
            .map_or(Boundary::Reserved, |turn| Boundary::Active(turn.clone()));
        self.turn = written.turn;
        self.admitted = written.admitted;
        self.current = Some(written.pending);
        true
    }
}
