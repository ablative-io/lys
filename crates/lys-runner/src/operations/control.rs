//! Prepared frames stay private; their receipts export only correlation and evidence.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{OperationOutcome, OperationState, Operations, TextDigest};
use crate::error::RunnerError;
use crate::harness_control::{Binding, ReminderReference};
use crate::session::{Sessions, now_ms};

/// What durable evidence says about an attempted pipe delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    /// The request was prepared without arming a pipe write.
    SafelyUnsent,
    /// A write may have happened; missing evidence cannot authorise replay.
    PossiblySent,
    /// The bound harness admitted this exact correlated request.
    Observed,
}

/// A responsible person's explicit reconciliation choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "choice", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reconciliation {
    /// Record the person's decision that delivery was seen.
    Seen,
    /// Record the person's decision that delivery was not seen.
    NotSeen,
    /// Record a distinct authorised occurrence with possible prior delivery.
    Resent {
        /// The new occurrence, kept by its existing goal owner.
        occurrence: String,
    },
}

/// Reconciliation is a person's recorded decision, separate from harness evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reconciled {
    /// The stable identity of this decision.
    pub operation: String,
    /// The verified responsible person.
    pub by: String,
    /// The decision instant recorded by the owner.
    pub at: u64,
    /// The explicit choice; it never silently resends a request.
    pub decision: Reconciliation,
}

/// Operational readback carries no prepared frame or saved goal words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlReceipt {
    /// The original operation.
    pub operation: String,
    /// Its explicit session.
    pub session: String,
    /// The named request.
    pub request: String,
    /// Its recorded outcome, without inferring completed work.
    pub state: OperationState,
    /// The recorded outcome instant.
    pub at: u64,
    /// The payload's digest, without its contents.
    pub text: Option<TextDigest>,
    /// Whether the exact prepared request was kept.
    pub prepared: bool,
    /// Evidence of a write attempt or harness admission.
    pub certainty: Option<Certainty>,
    /// The owning process generation, when a preparation exists.
    pub generation: Option<u64>,
    /// The request's correlation identity, never an exactly-once claim.
    pub uuid: Option<String>,
    /// The goal, revision, occurrence and possible prior delivery.
    pub reference: Option<ReminderReference>,
    /// Whether matching admission was durably observed.
    pub admitted: bool,
    /// The person's recorded decision, without rewriting harness evidence.
    pub reconciled: Option<Reconciled>,
}

/// A bounded page of current operation evidence for one explicit session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPage {
    /// The requested session, including an empty page's owner.
    pub session: String,
    /// This page's public receipts.
    pub receipts: Vec<ControlReceipt>,
    /// Continue strictly after this identity; absent only on the last page.
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Prepared {
    pub(crate) binding: Binding,
    pub(crate) uuid: String,
    pub(crate) frame: Value,
    pub(crate) reference: Option<ReminderReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Admission {
    pub(super) binding: Binding,
    pub(super) uuid: String,
    pub(super) turn: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Control {
    pub(super) prepared: Option<Prepared>,
    pub(super) original_text: Option<TextDigest>,
    pub(super) certainty: Certainty,
    pub(super) admitted: Option<Admission>,
    pub(super) decision: Option<Reconciled>,
}

impl Control {
    pub(super) fn matching_admission(&self) -> bool {
        self.prepared
            .as_ref()
            .zip(self.admitted.as_ref())
            .is_some_and(|(prepared, admission)| {
                admission.binding == prepared.binding && admission.uuid == prepared.uuid
            })
    }

    pub(super) fn validate(&self, outcome: &OperationOutcome) -> Result<(), RunnerError> {
        let Some(prepared) = &self.prepared else {
            if self.certainty != Certainty::PossiblySent || self.admitted.is_some() {
                return Err(RunnerError::refused(
                    "control_record_invalid",
                    "legacy delivery has no preparation or admission evidence",
                ));
            }
            return Ok(());
        };
        if prepared.binding.session != outcome.session
            || prepared.binding.generation == 0
            || prepared.binding.conversation.is_empty()
            || prepared.uuid.is_empty()
            || !prepared.frame.is_object()
            || (outcome.request == "goal_reminder") != prepared.reference.is_some()
            || self.admitted.is_some() && !self.matching_admission()
            || (self.certainty == Certainty::Observed) != self.matching_admission()
        {
            return Err(RunnerError::refused(
                "control_record_invalid",
                "preparation and evidence do not name the same operation source",
            ));
        }
        Ok(())
    }
}

impl ControlReceipt {
    pub(super) fn of(outcome: &OperationOutcome, control: Option<&Control>) -> Self {
        let prepared = control.and_then(|control| control.prepared.as_ref());
        Self {
            operation: outcome.operation.clone(),
            session: outcome.session.clone(),
            request: outcome.request.clone(),
            state: outcome.state,
            at: outcome.at,
            text: outcome.text.clone(),
            prepared: prepared.is_some(),
            certainty: control.map(|control| control.certainty),
            generation: prepared.map(|prepared| prepared.binding.generation),
            uuid: prepared.map(|prepared| prepared.uuid.clone()),
            reference: prepared.and_then(|prepared| prepared.reference.clone()),
            admitted: control.is_some_and(Control::matching_admission),
            reconciled: control.and_then(|control| control.decision.clone()),
        }
    }
}

impl Operations {
    pub(crate) fn prepare(
        &mut self,
        operation: &str,
        prepared: Prepared,
        text: TextDigest,
    ) -> Result<(), RunnerError> {
        let mut outcome = self.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused("operation_unknown", "no operation is held for preparation")
        })?;
        if outcome.state != OperationState::Accepted || self.controls.contains_key(operation) {
            return Err(RunnerError::refused(
                "control_preparation_repeated",
                "an operation is prepared only once",
            ));
        }
        let original_text = outcome.text.replace(text);
        let control = Control {
            prepared: Some(prepared),
            original_text,
            certainty: Certainty::SafelyUnsent,
            admitted: None,
            decision: None,
        };
        self.record_control(outcome, control)
    }

    pub(crate) fn arm(&mut self, operation: &str) -> Result<(), RunnerError> {
        let mut control = self.controls.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused(
                "control_preparation_missing",
                "a pipe write requires its retained preparation",
            )
        })?;
        if control.certainty != Certainty::SafelyUnsent {
            return Err(RunnerError::refused(
                "control_write_repeated",
                "a prepared request is armed only once",
            ));
        }
        control.certainty = Certainty::PossiblySent;
        let mut outcome = self.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused("operation_unknown", "the prepared operation is not held")
        })?;
        outcome.state = OperationState::Delivering;
        outcome.at = now_ms();
        "control_write_armed: delivery may occur after the durability barrier"
            .clone_into(&mut outcome.words);
        self.record_control(outcome, control)
    }

    pub(crate) fn observed(
        &mut self,
        operation: &str,
        binding: &Binding,
        uuid: &str,
        turn: Option<String>,
    ) -> Result<(), RunnerError> {
        let mut control = self.controls.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused(
                "control_preparation_missing",
                "admission requires the original retained preparation",
            )
        })?;
        if control
            .prepared
            .as_ref()
            .is_none_or(|prepared| &prepared.binding != binding || prepared.uuid != uuid)
            || control.certainty == Certainty::SafelyUnsent
        {
            return Err(RunnerError::refused(
                "control_admission_mismatch",
                "admission does not match the armed preparation",
            ));
        }
        control.admitted = Some(Admission {
            binding: binding.clone(),
            uuid: uuid.to_owned(),
            turn,
        });
        control.certainty = Certainty::Observed;
        let outcome = self.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused("operation_unknown", "the admitted operation is not held")
        })?;
        self.record_control(outcome, control)
    }

    pub(super) fn original_text(&self, operation: &str) -> Option<&TextDigest> {
        self.controls.get(operation).map_or_else(
            || {
                self.get(operation)
                    .and_then(|outcome| outcome.text.as_ref())
            },
            |control| control.original_text.as_ref(),
        )
    }

    fn receipt(&self, operation: &str) -> Result<ControlReceipt, RunnerError> {
        let outcome = self
            .get(operation)
            .ok_or_else(|| RunnerError::refused("operation_unknown", "no operation is held"))?;
        Ok(ControlReceipt::of(outcome, self.controls.get(operation)))
    }

    fn page(&self, session: &str, after: Option<&str>) -> Result<ControlPage, RunnerError> {
        use std::ops::Bound;
        if after == Some("") {
            return Err(RunnerError::refused(
                "control_cursor_invalid",
                "an operation cursor cannot be empty",
            ));
        }
        let mut page = ControlPage {
            session: session.to_owned(),
            receipts: Vec::new(),
            after: None,
        };
        let Some(ids) = self.by_session.get(session) else {
            return Ok(page);
        };
        let start = after.map_or(Bound::Unbounded, Bound::Excluded);
        let mut remaining = ids.range::<str, _>((start, Bound::Unbounded));
        for id in remaining.by_ref().take(crate::tracking_store::PAGE_MAX) {
            page.receipts.push(self.receipt(id)?);
        }
        if remaining.next().is_some() {
            page.after = page
                .receipts
                .last()
                .map(|receipt| receipt.operation.clone());
        }
        Ok(page)
    }

    fn reconcile(
        &mut self,
        operation: &str,
        decision: Reconciled,
    ) -> Result<ControlReceipt, RunnerError> {
        if decision.operation.is_empty()
            || decision.by.is_empty()
            || decision.at > now_ms()
            || matches!(&decision.decision, Reconciliation::Resent { occurrence } if occurrence.is_empty())
        {
            return Err(RunnerError::refused(
                "control_reconciliation_invalid",
                "the decision must name its identity, responsible person and instant",
            ));
        }
        let outcome = self.get(operation).cloned().ok_or_else(|| {
            RunnerError::refused("operation_unknown", "the operation is not held")
        })?;
        if outcome.state != OperationState::Uncertain {
            return Err(RunnerError::refused(
                "control_not_uncertain",
                "only an uncertain operation needs a person's reconciliation",
            ));
        }
        let mut control = match self.controls.get(operation) {
            Some(control) => control.clone(),
            None => Control {
                prepared: None,
                original_text: outcome.text.clone(),
                certainty: Certainty::PossiblySent,
                admitted: None,
                decision: None,
            },
        };
        if let Some(kept) = &control.decision {
            if *kept == decision {
                return self.receipt(operation);
            }
            return Err(RunnerError::refused(
                "control_already_reconciled",
                "a different decision already reconciles this operation",
            ));
        }
        control.decision = Some(decision);
        self.record_control(outcome, control)?;
        self.receipt(operation)
    }
}

impl Sessions {
    /// Read public evidence without exporting the private preparation.
    ///
    /// # Errors
    /// Refuses an unknown identity or an unavailable owner.
    pub fn control_receipt(&self, operation: &str) -> Result<ControlReceipt, RunnerError> {
        let receipt = self.lock()?.operations.receipt(operation)?;
        self.writer.barrier()?;
        Ok(receipt)
    }

    /// Read every current receipt through bounded, explicit continuation pages.
    ///
    /// # Errors
    /// Refuses an unknown session, malformed cursor or unavailable owner.
    pub fn control_receipts(
        &self,
        session: &str,
        after: Option<&str>,
    ) -> Result<ControlPage, RunnerError> {
        let table = self.lock()?;
        if !table.sessions.contains_key(session)
            && !table.operations.by_session.contains_key(session)
        {
            return Err(RunnerError::refused(
                "session_unknown",
                "no current session or retained operation names this session",
            ));
        }
        let page = table.operations.page(session, after)?;
        drop(table);
        self.writer.barrier()?;
        Ok(page)
    }

    pub(crate) fn reconcile_control(
        &self,
        operation: &str,
        decision: Reconciled,
    ) -> Result<ControlReceipt, RunnerError> {
        let receipt = self.lock()?.operations.reconcile(operation, decision)?;
        self.writer.barrier()?;
        Ok(receipt)
    }
}
