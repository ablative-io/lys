//! The managed dispatcher journals into 051's existing operation record.
//! This adds no store, scheduler or authority to start a process or a turn.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{OperationState, Operations, TextDigest};
use crate::error::RunnerError;
use crate::harness_control::events::{Projection, Source};
use crate::harness_control::process::WriteAhead;

/// Evidence fixed before the only pipe write, retained on uncertain outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    /// The process generation and conversation bound by the launch owner.
    pub source: Source,
    /// Digest of the exact encoded native request, never its message text.
    pub frame: TextDigest,
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
        }));
        // If persistence fails, the flight remains reserved and no pipe write
        // follows. In memory, this operation is fenced against any new send.
        operations.set(operation, OperationState::Delivering,
            "managed request may be sent after this durable receipt; native admission not yet observed".to_owned())?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
