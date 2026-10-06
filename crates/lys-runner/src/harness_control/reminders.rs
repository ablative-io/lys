//! Current service authority replaces only an occurrence's unsent payload.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    Boundary, BoundaryReply, ContextDecision, Controller, Kind, Pending, Transport, Update, receipt,
};
use crate::error::RunnerError;
use crate::operations::OperationState;

/// The goal revision and stable occurrence behind a queued delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReminderReference {
    /// The saved aim.
    pub goal: String,
    /// The occurrence kept under its original due instant.
    pub occurrence: String,
    /// The operation that names the saved revision.
    pub version: String,
    /// Possible prior delivery named by an explicitly authorised resend.
    pub prior: Option<String>,
}

/// One queued delivery's identities for service recovery and current authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QueuedReminder {
    /// This session's delivery identity.
    pub operation: String,
    /// The aim's current queued revision and stable occurrence.
    pub reference: ReminderReference,
}

/// A current decision for one occurrence at the named boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReminderDecision {
    /// Deliver these currently authorised words under the existing occurrence.
    Deliver {
        /// This session's delivery identity.
        operation: String,
        /// The goal, occurrence and current revision.
        reference: ReminderReference,
        /// Current words already carried by the reminder operation.
        text: String,
    },
    /// Refuse the queued occurrence without sending its words.
    Refuse {
        /// This session's delivery identity.
        operation: String,
        /// The goal and occurrence being refused.
        reference: ReminderReference,
        /// The named reason.
        reason: String,
    },
}

impl Pending {
    /// Retain occurrence identity while keeping goal words as input data.
    #[must_use]
    pub fn for_goal(id: String, text: String, reference: ReminderReference) -> Self {
        let mut pending = Self::new(id, Kind::Reminder, text);
        pending.reminder = Some(reference);
        pending
    }
}

impl Controller {
    fn reminder_decisions<'a>(
        &self,
        decisions: &'a [ReminderDecision],
    ) -> Result<Decisions<'a>, RunnerError> {
        let mut by_operation = std::collections::BTreeMap::new();
        for decision in decisions {
            let operation = match decision {
                ReminderDecision::Deliver { operation, .. }
                | ReminderDecision::Refuse { operation, .. } => operation,
            };
            if by_operation.insert(operation.as_str(), decision).is_some() {
                return Err(RunnerError::refused(
                    "control_reminder_repeated",
                    "one boundary decision names the same delivery twice",
                ));
            }
        }
        if by_operation.keys().any(|operation| {
            !self
                .pending
                .iter()
                .any(|pending| pending.id == *operation && pending.reminder.is_some())
        }) {
            return Err(RunnerError::refused(
                "control_reminder_unknown",
                "the decision names no queued goal occurrence",
            ));
        }
        for pending in &self.pending {
            let Some(held) = &pending.reminder else {
                continue;
            };
            let decision = by_operation.get(pending.id.as_str()).ok_or_else(|| {
                RunnerError::refused(
                    "control_reminder_authority_missing",
                    "the boundary reply omits a queued goal occurrence",
                )
            })?;
            let reference = match decision {
                ReminderDecision::Deliver { reference, .. }
                | ReminderDecision::Refuse { reference, .. } => reference,
            };
            if held.goal != reference.goal
                || held.occurrence != reference.occurrence
                || held.prior != reference.prior
            {
                return Err(RunnerError::refused(
                    "control_reminder_changed",
                    "the service reply names another goal or occurrence",
                ));
            }
        }
        Ok(by_operation)
    }

    pub(super) fn apply_reminders(
        &mut self,
        boundary: &str,
        by_operation: &Decisions<'_>,
    ) -> Update {
        let mut update = Update::default();
        self.pending.retain_mut(|pending| {
            let Some(decision) = by_operation.get(pending.id.as_str()) else {
                return true;
            };
            match decision {
                ReminderDecision::Deliver {
                    reference, text, ..
                } => {
                    pending.reminder = Some(reference.clone());
                    pending.text.clone_from(text);
                    pending.authorised = Some(boundary.to_owned());
                    true
                }
                ReminderDecision::Refuse { reason, .. } => {
                    self.pending_ids.remove(&pending.id);
                    update
                        .receipts
                        .push(receipt(&pending.id, OperationState::Refused, reason));
                    false
                }
            }
        });
        update
    }
}

impl Pending {
    /// Construct a correlated input without putting its words into an identity.
    #[must_use]
    pub fn new(id: String, kind: Kind, text: String) -> Self {
        let hash = crate::protocol::hex(&Sha256::digest(id.as_bytes()));
        let uuid = format!(
            "{}-{}-4{}-a{}-{}",
            &hash[..8],
            &hash[8..12],
            &hash[13..16],
            &hash[17..20],
            &hash[20..32]
        );
        Self {
            id,
            uuid,
            kind,
            text,
            reminder: None,
            authorised: None,
        }
    }
}

type Decisions<'a> = std::collections::BTreeMap<&'a str, &'a ReminderDecision>;

pub(super) enum BoundaryPlan<'a> {
    Unavailable(&'a str),
    Current {
        reply: &'a BoundaryReply,
        boundary: &'a str,
        decisions: Decisions<'a>,
        next: Option<Box<(Pending, serde_json::Value)>>,
    },
}

impl Controller {
    /// Apply current service authority only to the exact waiting boundary.
    ///
    /// # Errors
    /// Refuses stale generations, unmatched boundaries and mismatched crossings.
    pub fn boundary_reply(&mut self, reply: &BoundaryReply) -> Result<Update, RunnerError> {
        let plan = self.plan_boundary_reply(reply)?;
        Ok(self.apply_boundary_plan(plan))
    }

    pub(super) fn plan_boundary_reply<'a>(
        &self,
        reply: &'a BoundaryReply,
    ) -> Result<BoundaryPlan<'a>, RunnerError> {
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
            return Ok(BoundaryPlan::Unavailable(reason));
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
        let decisions = self.reminder_decisions(&reply.reminders)?;
        let next = self
            .boundary_next(boundary, &reply.context, &decisions)?
            .map(Box::new);
        Ok(BoundaryPlan::Current {
            reply,
            boundary,
            decisions,
            next,
        })
    }

    fn boundary_next(
        &self,
        boundary: &str,
        context: &ContextDecision,
        decisions: &Decisions<'_>,
    ) -> Result<Option<(Pending, serde_json::Value)>, RunnerError> {
        let next = self.pending.iter().find(|pending| {
            let refused = matches!(
                decisions.get(pending.id.as_str()),
                Some(ReminderDecision::Refuse { .. })
            );
            let released_compact = matches!(context, ContextDecision::Released)
                && pending.kind == Kind::Compact
                && self.authority.crossing.as_deref() == Some(pending.id.as_str());
            !(refused || released_compact)
        });
        let Some(next) = next else {
            return Ok(None);
        };
        let permitted = match context {
            ContextDecision::Released => true,
            ContextDecision::Compact { crossing } => {
                next.kind == Kind::Compact && next.id == *crossing
            }
            ContextDecision::Held { .. } | ContextDecision::Unavailable { .. } => false,
        };
        if !permitted {
            return Ok(None);
        }
        let mut next = next.clone();
        if let Some(ReminderDecision::Deliver {
            reference, text, ..
        }) = decisions.get(next.id.as_str())
        {
            next.reminder = Some(reference.clone());
            next.text.clone_from(text);
            next.authorised = Some(boundary.to_owned());
        }
        let frame = match self.transport {
            Transport::Claude => super::claude::request(&self.binding.conversation, &next),
            Transport::Codex => super::codex::request(&self.binding.conversation, &next),
            Transport::Pty => return Err(super::unsupported()),
        };
        Ok(Some((next, frame)))
    }
}
