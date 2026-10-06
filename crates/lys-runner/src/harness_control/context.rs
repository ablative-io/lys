//! The service judges policy; a channel reserves only an exact owned boundary.

use serde::{Deserialize, Serialize};

use super::{Boundary, Controller, Kind, ReminderDecision, Update};
use crate::error::RunnerError;

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

/// Current control identifiers, read directly from the owned live controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlStatus {
    /// The current process generation.
    pub generation: u64,
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
}

impl Controller {
    /// Current identifiers for service recovery, without transcript or goal text.
    #[must_use]
    pub fn control_status(&self) -> ControlStatus {
        ControlStatus {
            generation: self.binding.generation,
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

    /// Apply current service authority only to the exact waiting boundary.
    ///
    /// # Errors
    /// Refuses stale generations, unmatched boundaries and mismatched crossings.
    pub fn boundary_reply(&mut self, reply: &BoundaryReply) -> Result<Update, RunnerError> {
        if self.closed
            || !self.authority.required
            || reply.generation != self.binding.generation
            || self.authority.waiting != reply.boundary
        {
            return Err(RunnerError::refused(
                "control_boundary_changed",
                "the reply does not name the currently held owned boundary",
            ));
        }
        if let ContextDecision::Unavailable { reason } = &reply.context {
            return Ok(self.refuse_held(reason));
        }
        let boundary = reply.boundary.as_deref().ok_or_else(|| {
            RunnerError::refused(
                "control_boundary_missing",
                "a release names a currently held boundary",
            )
        })?;
        if self.boundary != Boundary::Idle {
            return Err(RunnerError::refused(
                "control_boundary_active",
                "a service reply cannot release an active turn",
            ));
        }
        if let ContextDecision::Compact { crossing } = &reply.context
            && self.authority.crossing.as_deref() != Some(crossing.as_str())
        {
            return Err(RunnerError::refused(
                "control_crossing_changed",
                "compaction is not the held crossing",
            ));
        }
        let mut update = self.review_reminders(boundary, &reply.reminders)?;
        self.authority.answered = true;
        match &reply.context {
            ContextDecision::Unavailable { reason } => {
                return Err(RunnerError::refused("control_boundary_unavailable", reason));
            }
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
                        crate::operations::OperationState::Refused,
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
        }
        self.dispatch(&mut update)?;
        Ok(update)
    }
}
