use super::{
    AppliedContext, Boundary, BoundaryPlan, Controller, Kind, OperationState, RunnerError, Update,
};
use crate::harness_control::{ContextDecision, ControlStatus, Dispatch, receipt};

impl Controller {
    pub(in crate::harness_control) fn preparation(
        &self,
        dispatch: &Dispatch,
    ) -> Result<(crate::operations::Prepared, crate::operations::TextDigest), RunnerError> {
        let current = self
            .current
            .as_ref()
            .filter(|pending| pending.id == dispatch.operation)
            .or_else(|| {
                self.early_notices
                    .iter()
                    .map(|written| &written.pending)
                    .find(|pending| pending.id == dispatch.operation)
            })
            .ok_or_else(|| {
                RunnerError::refused(
                    "control_preparation_missing",
                    "the dispatch has no reserved input",
                )
            })?;
        Ok((
            crate::operations::Prepared {
                binding: self.binding.clone(),
                uuid: current.uuid.clone(),
                frame: dispatch.frame.clone(),
                reference: current.reminder.clone(),
            },
            crate::operations::TextDigest::of(&current.text),
        ))
    }
}

impl crate::session::Sessions {
    /// Read current controller state without taking output or account history.
    ///
    /// # Errors
    /// Refuses an unknown session or an unavailable session table.
    pub fn control_status(
        &self,
        session: &str,
    ) -> Result<Option<ControlStatus>, crate::error::RunnerError> {
        let table = self.lock()?;
        let held = table.sessions.get(session).ok_or_else(|| {
            crate::error::RunnerError::refused(
                "session_unknown",
                "the runner holds no session with this identity",
            )
        })?;
        Ok(held
            .live
            .as_ref()
            .and_then(|live| live.control.as_ref())
            .map(|control| control.controller.control_status()))
    }
}

impl Controller {
    pub(in crate::harness_control) fn apply_boundary_plan(
        &mut self,
        plan: BoundaryPlan<'_>,
    ) -> Update {
        let (reply, boundary, decisions, next) = match plan {
            BoundaryPlan::Unavailable(reason) => return self.refuse_held(reason),
            BoundaryPlan::Current {
                reply,
                boundary,
                decisions,
                next,
            } => (reply, boundary, decisions, next),
        };
        let mut update = self.apply_reminders(boundary, &decisions);
        self.authority.answered = true;
        self.authority.last = Some(AppliedContext::of(&reply.context));
        match &reply.context {
            ContextDecision::Released => {
                if let Some(crossing) = self.authority.crossing.take()
                    && let Some(position) = self
                        .pending
                        .iter()
                        .position(|pending| pending.kind == Kind::Compact && pending.id == crossing)
                {
                    self.pending.remove(position);
                    self.pending_ids.remove(&crossing);
                    update.receipts.push(receipt(
                        &crossing,
                        OperationState::Refused,
                        "context_policy_released: compaction was not written",
                    ));
                }
                self.authority.normal = true;
                self.authority.compact = None;
            }
            ContextDecision::Compact { crossing } => {
                self.authority.normal = false;
                self.authority.compact = Some(crossing.clone());
            }
            ContextDecision::Held { crossing, .. } => {
                self.authority.crossing.clone_from(crossing);
                self.authority.normal = false;
                self.authority.compact = None;
            }
            ContextDecision::Unavailable { .. } => {}
        }
        if let Some(next) = next {
            let (pending, frame) = *next;
            self.pending.pop_front();
            self.pending_ids.remove(&pending.id);
            self.boundary = Boundary::Reserved;
            self.authority.waiting = None;
            self.authority.answered = false;
            self.compacted = None;
            self.admitted = false;
            self.turn = None;
            update
                .events
                .push(self.event("control_prepared", None, Some(pending.id.clone())));
            update.dispatches.push(Dispatch {
                operation: pending.id.clone(),
                frame,
            });
            self.current = Some(pending);
        }
        update
    }
}
