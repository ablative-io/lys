//! The managed dispatcher journals into 051's existing operation record.
//! This adds no store, scheduler or authority to start a process or a turn.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{OperationOutcome, OperationState, Operations, TextDigest};
use crate::error::RunnerError;
use crate::harness_control::events::{Event, Projection, Source};
use crate::harness_control::process::WriteAhead;
use crate::tracking_store::Feed;

mod evidence;
pub use evidence::Evidence;

/// Evidence fixed before the only pipe write, retained on uncertain outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    /// The process generation and conversation bound by the launch owner.
    pub source: Source,
    /// Digest of the exact encoded native request, never its message text.
    pub frame: TextDigest,
    /// Native facts retained separately from write success or goal satisfaction.
    #[serde(default)]
    pub evidence: Evidence,
}

// Constructed while the session table is held, borrowing its existing
// operation owner and the dispatcher's proved source. The dispatcher validates
// the typed request and current authority before invoking this journal.
impl WriteAhead for (&mut Operations, &Source) {
    fn before_write(&mut self, operation: &str, encoded: &[u8]) -> Result<(), RunnerError> {
        let (operations, source) = self;
        Projection::new((*source).clone())?;
        let held = operations
            .held
            .iter_mut()
            .find(|held| held.operation == operation)
            .ok_or_else(|| {
                RunnerError::refused(
                    "operation_unknown",
                    "managed delivery has no accepted operation",
                )
            })?;
        if held.session != source.binding.session {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "operation belongs to another runner session",
            ));
        }
        if held.state != OperationState::Accepted || held.control.is_some() {
            return Err(RunnerError::refused(
                "control_delivery_uncertain",
                "operation is no longer safely unsent; reconcile its existing receipt",
            ));
        }
        held.control = Some(Box::new(Delivery {
            source: (*source).clone(),
            frame: TextDigest {
                length: encoded.len(),
                sha256: crate::protocol::hex(&Sha256::digest(encoded)),
            },
            evidence: Evidence::default(),
        }));
        // If persistence fails, the flight remains reserved and no pipe write
        // follows. In memory, this operation is fenced against any new send.
        operations.set(operation, OperationState::Delivering,
            "managed request may be sent after this durable receipt; native admission not yet observed".to_owned())?;
        Ok(())
    }
}

impl Operations {
    /// The feed is durable first. A failed receipt write leaves the old in-memory
    /// outcome; the same kept event can reconcile it without another pipe write.
    pub(crate) fn keep_control(
        &mut self,
        feed: &mut Feed,
        operation: &str,
        event: &Event,
    ) -> Result<OperationOutcome, RunnerError> {
        let index = self
            .held
            .iter()
            .position(|held| held.operation == operation)
            .ok_or_else(|| {
                RunnerError::refused("operation_unknown", "no managed operation is held")
            })?;
        let before = self.held[index].clone();
        let mut next = before.clone();
        let delivery = next.control.as_mut().ok_or_else(|| {
            RunnerError::refused(
                "control_delivery_unprepared",
                "no managed request was journalled for this operation",
            )
        })?;
        if event.source != delivery.source || event.source_id.is_empty() {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "observation differs from the journalled process and conversation",
            ));
        }
        let (state, words) = delivery.evidence.observe(operation, &next.request, event)?;
        feed.append_control(&delivery.source, crate::session::now_ms(), event)?;
        if before.state != OperationState::Uncertain || state != OperationState::Delivering {
            next.state = state;
            words.clone_into(&mut next.words);
        }
        if next == before {
            return Ok(before);
        }
        next.at = crate::session::now_ms();
        self.held[index] = next.clone();
        if let Err(error) = self.persist() {
            self.held[index] = before;
            return Err(error);
        }
        Ok(next)
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
