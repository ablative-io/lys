//! Native admission, terminal state and compaction are independent facts.
//! A terminal turn alone cannot confirm compaction or satisfaction of a goal.
use serde::{Deserialize, Serialize};

use crate::error::RunnerError;
use crate::harness_control::events::{Event, Kind};
use crate::operations::OperationState;

/// Safe native identities folded into the existing durable operation receipt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// The one native turn bound to the operation.
    pub turn: Option<String>,
    /// Source event proving native admission, not completion or obedience.
    pub admission: Option<String>,
    /// Source event proving termination of the bound turn.
    pub terminal: Option<String>,
    /// Source event proving actual native compaction.
    pub compaction: Option<String>,
    /// Native compaction item or boundary, separate from the turn identity.
    pub item: Option<String>,
    /// A transport gap remains visible until sufficient native proof exists.
    pub lost: bool,
}

fn mismatch() -> RunnerError {
    RunnerError::refused(
        "control_operation_mismatch",
        "native evidence does not identify this operation and its one turn",
    )
}

impl Evidence {
    fn bind_turn(&mut self, turn: &str) -> Result<(), RunnerError> {
        if turn.is_empty() || self.turn.as_deref().is_some_and(|held| held != turn) {
            return Err(mismatch());
        }
        self.turn = Some(turn.to_owned());
        Ok(())
    }

    pub(super) fn observe(
        &mut self,
        operation: &str,
        request: &str,
        event: &Event,
    ) -> Result<(OperationState, &'static str), RunnerError> {
        match &event.kind {
            Kind::Admitted {
                operation: native,
                turn,
            } => {
                if native != operation {
                    return Err(mismatch());
                }
                if let Some(turn) = turn {
                    self.bind_turn(turn)?;
                }
                self.admission
                    .get_or_insert_with(|| event.source_id.clone());
            }
            Kind::TurnStarted { turn } => self.bind_turn(turn)?,
            Kind::TurnCompleted { turn } => {
                if self.turn.as_deref() != Some(turn.as_str()) {
                    return Err(mismatch());
                }
                self.terminal.get_or_insert_with(|| event.source_id.clone());
            }
            Kind::Compacted {
                operation: native,
                turn,
                item,
            } => {
                if native != operation
                    || request != "compact"
                    || item.is_empty()
                    || self.turn.as_deref() != Some(turn.as_str())
                    || self.item.as_deref().is_some_and(|prior| prior != item)
                {
                    return Err(mismatch());
                }
                self.compaction
                    .get_or_insert_with(|| event.source_id.clone());
                self.item = Some(item.clone());
            }
            Kind::Lost { .. } | Kind::Exited => self.lost = true,
            Kind::IdleReconciled | Kind::Rejected { .. } => {
                return Err(RunnerError::refused(
                    "control_evidence_unrelated",
                    "this fact does not reconcile an operation; retain it in the session feed",
                ));
            }
        }
        if self.admission.is_some()
            && request == "compact"
            && self.terminal.is_some()
            && self.compaction.is_some()
        {
            return Ok((
                OperationState::Confirmed,
                "matching native admission, compaction and terminal evidence were retained",
            ));
        }
        if self.admission.is_some() && request != "compact" {
            return Ok((
                OperationState::Delivered,
                "native admission observed; completion, obedience and goal satisfaction are not asserted",
            ));
        }
        if self.lost {
            return Ok((
                OperationState::Uncertain,
                "managed transport ended or lost evidence; reconcile the original receipt, never automatically resend",
            ));
        }
        if self.admission.is_some() {
            return Ok((
                OperationState::Delivered,
                "native admission observed; actual compaction is not yet proved",
            ));
        }
        Ok((
            OperationState::Delivering,
            "managed request may have been written; native admission not yet observed",
        ))
    }
}
