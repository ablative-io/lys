//! The service judges policy; a channel reserves only an exact owned boundary.

use serde::{Deserialize, Serialize};

use super::events::{apply, runtime};
use super::reminders::BoundaryPlan;
use super::{Boundary, Controller, Dispatch, Kind, ReminderDecision, Update};
use crate::error::RunnerError;
use crate::operations::{OperationRequest, OperationState};
use crate::session::now_ms;

/// The service's context decision, without a limit or measured value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContextDecision {
    /// The current authority path ended, so held words receive a named refusal.
    Unavailable {
        /// The authority or feed refusal.
        reason: String,
    },
    /// Current authority permits normal input.
    Released,
    /// One existing crossing may compact at this boundary.
    Compact {
        /// The crossing already kept by the service.
        crossing: String,
    },
    /// Normal input remains held by the named policy reason.
    Held {
        /// The crossing, when one has already occurred.
        crossing: Option<String>,
        /// Why the next turn cannot proceed.
        reason: String,
    },
}

/// One service reply for a boundary published on the existing managed feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryReply {
    /// The process generation that owns this boundary.
    pub generation: u64,
    /// The boundary's stable correlation identity.
    pub boundary: Option<String>,
    /// The service's current policy decision.
    pub context: ContextDecision,
    /// Current goal decisions for this session's pending occurrences.
    pub reminders: Vec<ReminderDecision>,
}

/// Only the bound reader or dispatcher can establish the current phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlPhase {
    /// No authoritative boundary has been observed.
    Unknown,
    /// A proved turn boundary is available.
    Idle,
    /// The dispatcher reserved the next turn.
    Reserved,
    /// A matching harness turn is active.
    Active,
    /// The managed channel ended.
    Closed,
}

/// Public reason codes never carry the service's detailed refusal text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextReason {
    /// No current measurement exists.
    MeasurementUnavailable,
    /// The reported instant is ahead of the observation.
    MeasurementFuture,
    /// No fresh measurement follows the completed control act.
    PostMeasurementMissing,
    /// The process stop owner has not observed exit.
    StopPending,
    /// Possible prior delivery prevents another compaction.
    DeliveryUncertain,
    /// Current context still exceeds its limit.
    AboveLimit,
    /// Current authority cannot answer.
    AuthorityUnavailable,
    /// The applied hold supplied no recognised public reason code.
    ReasonUnrecognised,
}

/// The last applied decision, with identifiers and closed reason codes only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppliedContext {
    /// Normal input was permitted by this decision.
    Released,
    /// One crossing was permitted to compact.
    Compact {
        /// The authorised crossing identity.
        crossing: String,
    },
    /// Normal input remains held.
    Held {
        /// The crossing whose input remains held, when one exists.
        crossing: Option<String>,
        /// The public reason for the applied hold.
        reason: ContextReason,
    },
    /// Current authority ended and queued words were refused.
    Unavailable {
        /// The public reason authority could not answer.
        reason: ContextReason,
    },
}

impl AppliedContext {
    fn of(decision: &ContextDecision) -> Self {
        match decision {
            ContextDecision::Released => Self::Released,
            ContextDecision::Compact { crossing } => Self::Compact {
                crossing: crossing.clone(),
            },
            ContextDecision::Unavailable { .. } => Self::Unavailable {
                reason: ContextReason::AuthorityUnavailable,
            },
            ContextDecision::Held { crossing, reason } => {
                let reason = match reason
                    .split_once(':')
                    .map_or(reason.as_str(), |(code, _)| code)
                {
                    "context_measurement_unavailable" => ContextReason::MeasurementUnavailable,
                    "context_measurement_future" => ContextReason::MeasurementFuture,
                    "context_post_measurement_missing" => ContextReason::PostMeasurementMissing,
                    "context_stop_pending" => ContextReason::StopPending,
                    "context_delivery_uncertain" => ContextReason::DeliveryUncertain,
                    "context_above_limit" => ContextReason::AboveLimit,
                    _ => {
                        crate::error::said(
                            "control_reason_unrecognised: the applied hold has no recognised public reason code",
                        );
                        ContextReason::ReasonUnrecognised
                    }
                };
                Self::Held {
                    crossing: crossing.clone(),
                    reason,
                }
            }
        }
    }
}

/// Current control identifiers, read directly from the owned live controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlStatus {
    /// The current process generation.
    pub generation: u64,
    /// The current proved channel phase; silence never changes it.
    pub phase: ControlPhase,
    /// The reserved or active operation, without its payload.
    pub active: Option<String>,
    /// The last applied decision; absent before any service decision.
    pub context: Option<AppliedContext>,
    /// The boundary held for a current service decision.
    pub boundary: Option<String>,
    /// The crossing that holds normal input.
    pub crossing: Option<String>,
    /// Queued goal occurrences, carrying no saved words.
    pub queued: Vec<super::reminders::QueuedReminder>,
}

#[derive(Debug, Default)]
pub(super) struct Authority {
    pub(super) required: bool,
    sequence: u64,
    pub(super) waiting: Option<String>,
    pub(super) crossing: Option<String>,
    compact: Option<String>,
    pub(super) normal: bool,
    pub(super) answered: bool,
    last: Option<AppliedContext>,
}

impl Controller {
    /// Current identifiers for service recovery, without transcript or goal text.
    #[must_use]
    pub fn control_status(&self) -> ControlStatus {
        ControlStatus {
            generation: self.binding.generation,
            phase: if self.closed {
                ControlPhase::Closed
            } else {
                match self.boundary {
                    Boundary::Unknown => ControlPhase::Unknown,
                    Boundary::Idle => ControlPhase::Idle,
                    Boundary::Reserved => ControlPhase::Reserved,
                    Boundary::Active(_) => ControlPhase::Active,
                }
            },
            active: self.current.as_ref().map(|pending| pending.id.clone()),
            context: self.authority.last.clone(),
            boundary: self.authority.waiting.clone(),
            crossing: self.authority.crossing.clone(),
            queued: self
                .pending
                .iter()
                .filter_map(|pending| {
                    pending
                        .reminder
                        .as_ref()
                        .map(|reference| super::reminders::QueuedReminder {
                            operation: pending.id.clone(),
                            reference: reference.clone(),
                        })
                })
                .collect(),
        }
    }

    pub(super) fn refuse_held(&mut self, reason: &str) -> Update {
        self.authority.last = Some(AppliedContext::Unavailable {
            reason: ContextReason::AuthorityUnavailable,
        });
        let mut update = Update::default();
        for pending in self.pending.drain(..) {
            update.receipts.push(super::receipt(
                &pending.id,
                crate::operations::OperationState::Refused,
                reason,
            ));
        }
        self.pending_ids.clear();
        self.authority.normal = false;
        self.authority.waiting = None;
        self.authority.compact = None;
        self.authority.answered = false;
        update
            .events
            .push(self.event("control_authority_unavailable", None, None));
        update
    }

    /// Require the service's reply before any normal input at a boundary.
    ///
    /// # Errors
    /// Refuses a change after the channel has bound or dispatched input.
    pub fn require_boundary_authority(&mut self) -> Result<(), RunnerError> {
        if self.ready || self.current.is_some() || !self.pending.is_empty() {
            return Err(RunnerError::refused(
                "control_authority_already_bound",
                "boundary authority is selected before binding the channel",
            ));
        }
        self.authority.required = true;
        Ok(())
    }

    pub(super) fn request_boundary(&mut self, update: &mut Update) -> Result<(), RunnerError> {
        if !self.authority.required
            || (self.authority.waiting.is_some() && !self.authority.answered)
        {
            return Ok(());
        }
        self.authority.sequence = self.authority.sequence.checked_add(1).ok_or_else(|| {
            RunnerError::refused(
                "control_boundary_exhausted",
                "boundary sequence is exhausted",
            )
        })?;
        let boundary = format!(
            "boundary-{}-{}-{}",
            self.binding.session, self.binding.generation, self.authority.sequence
        );
        self.authority.normal = false;
        self.authority.compact = None;
        self.authority.answered = false;
        update
            .events
            .push(self.event("control_boundary", None, Some(boundary.clone())));
        self.authority.waiting = Some(boundary);
        Ok(())
    }

    pub(super) fn permitted(&self) -> bool {
        if !self.authority.required {
            return true;
        }
        let Some(next) = self.pending.front() else {
            return false;
        };
        if !self.authority.answered {
            return false;
        }
        if next.kind == Kind::Compact && self.authority.crossing.as_deref() == Some(&next.id) {
            return self.authority.compact.as_deref() == Some(&next.id);
        }
        self.authority.normal && (next.reminder.is_none() || next.authorised.is_some())
    }

    /// Queue the single action for an existing context crossing.
    ///
    /// # Errors
    /// Refuses another crossing while its predecessor still holds input.
    pub fn context_compact(&mut self, pending: super::Pending) -> Result<Update, RunnerError> {
        if pending.kind != Kind::Compact || !self.authority.required {
            return Err(RunnerError::refused(
                "control_context_invalid",
                "a context crossing requires controlled compaction",
            ));
        }
        if self
            .authority
            .crossing
            .as_ref()
            .is_some_and(|held| held != &pending.id)
        {
            return Err(RunnerError::refused(
                "control_context_already_held",
                "the previous crossing still holds the next turn",
            ));
        }
        self.authority.crossing = Some(pending.id.clone());
        self.authority.normal = false;
        let mut update = self.enqueue(pending)?;
        if self.boundary == Boundary::Idle {
            self.request_boundary(&mut update)?;
        }
        Ok(update)
    }
}

impl crate::session::Sessions {
    pub(crate) fn apply_boundary_reply(
        &self,
        operation: crate::operations::Operation,
    ) -> Result<crate::operations::OperationOutcome, RunnerError> {
        let OperationRequest::BoundaryReply { reply } = &operation.request else {
            return Err(RunnerError::refused(
                "control_boundary_invalid",
                "the request is not a boundary reply",
            ));
        };
        let encoded = serde_json::to_string(reply)
            .map_err(|error| RunnerError::refused("control_boundary_invalid", error.to_string()))?;
        let digest = crate::operations::TextDigest::of(&encoded);
        let mut table = self.lock()?;
        if let Some(held) = table.operations.get(&operation.operation) {
            if held.session != operation.session
                || held.request != "boundary_reply"
                || held.text.as_ref() != Some(&digest)
            {
                return Err(RunnerError::refused(
                    "operation_reused",
                    "the boundary reply already names another decision",
                ));
            }
            let outcome = held.clone();
            drop(table);
            self.writer.barrier()?;
            return Ok(outcome);
        }
        table
            .operations
            .check_control_identity(&operation.operation)?;
        let plan = runtime(&mut table, &operation.session, reply.generation)?
            .controller
            .plan_boundary_reply(reply);
        let (state, words) = match &plan {
            Ok(_) => (
                OperationState::Confirmed,
                "boundary_authority_applied".to_owned(),
            ),
            Err(error) => (OperationState::Refused, error.to_string()),
        };
        let outcome = crate::operations::OperationOutcome {
            operation: operation.operation,
            session: operation.session.clone(),
            request: "boundary_reply".to_owned(),
            state,
            at: now_ms(),
            words,
            text: Some(digest),
            ended: None,
        };
        table.operations.keep_control(outcome.clone())?;
        let controller = &mut runtime(&mut table, &operation.session, reply.generation)?.controller;
        let update = match plan {
            Ok(plan) => controller.apply_boundary_plan(plan),
            Err(_) => controller.refuse_held(&outcome.words),
        };
        apply(&mut table, &operation.session, reply.generation, update)?;
        drop(table);
        self.writer.barrier()?;
        self.wake();
        Ok(outcome)
    }
}

impl Controller {
    pub(super) fn preparation(
        &self,
        dispatch: &Dispatch,
    ) -> Result<(crate::operations::Prepared, crate::operations::TextDigest), RunnerError> {
        let current = self
            .current
            .as_ref()
            .filter(|pending| pending.id == dispatch.operation)
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
    pub(super) fn apply_boundary_plan(&mut self, plan: BoundaryPlan<'_>) -> Update {
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
                    update.receipts.push(super::receipt(
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
