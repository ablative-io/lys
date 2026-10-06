//! Prepared frames stay private; their receipts export only correlation and evidence.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{OperationOutcome, OperationState, TextDigest};
use crate::harness_control::{Binding, ReminderReference};

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
    /// Named evidence or refusal.
    pub words: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prepared {
    pub(super) binding: Binding,
    pub(super) uuid: String,
    pub(super) frame: Value,
    pub(super) reference: Option<ReminderReference>,
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
    pub(super) prepared: Prepared,
    pub(super) certainty: Certainty,
    pub(super) admitted: Option<Admission>,
    pub(super) decision: Option<Reconciled>,
}

impl Control {
    pub(super) fn matching_admission(&self) -> bool {
        self.admitted.as_ref().is_some_and(|admission|
            admission.binding == self.prepared.binding && admission.uuid == self.prepared.uuid)
    }
}

impl ControlReceipt {
    pub(super) fn of(outcome: &OperationOutcome, control: Option<&Control>) -> Self {
        let prepared = control.map(|control| &control.prepared);
        Self {
            operation: outcome.operation.clone(), session: outcome.session.clone(),
            request: outcome.request.clone(), state: outcome.state, at: outcome.at,
            words: outcome.words.clone(), text: outcome.text.clone(), prepared: prepared.is_some(),
            certainty: control.map(|control| control.certainty),
            generation: prepared.map(|prepared| prepared.binding.generation),
            uuid: prepared.map(|prepared| prepared.uuid.clone()),
            reference: prepared.and_then(|prepared| prepared.reference.clone()),
            admitted: control.is_some_and(Control::matching_admission),
            reconciled: control.and_then(|control| control.decision.clone()),
        }
    }
}
