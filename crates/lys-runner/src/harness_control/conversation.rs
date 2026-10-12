//! Resume identities are checked before a managed launch can change a session.

use crate::error::RunnerError;
use crate::protocol::Launch;
use serde::{Deserialize, Serialize};

/// The transport selected in the signed launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Transport {
    /// Existing manual terminal transport.
    Pty,
    /// Claude stream-json pipes.
    Claude,
    /// Codex app-server pipes.
    Codex,
}

/// A transport choice carrying the exact reviewed control requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedLaunch {
    /// The existing admitted process inputs.
    pub launch: Launch,
    /// The selected transport.
    pub transport: Transport,
    /// An explicit conversation to resume, or empty for a new Codex thread.
    pub conversation: String,
    /// Resume the named conversation instead of creating it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resume: bool,
    /// Whether a manual transport must refuse the launch.
    pub requires_controls: bool,
    /// The supervised-seat binding, when AGENTS-002 started this as a seat:
    /// the session then gets an independent owner (AGENTS-004 R1). Absent
    /// for a manual launch, which keeps its lifecycle; never inferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<crate::seat_owner::protocol::OwnerBinding>,
}

#[derive(Debug, Clone)]
pub(crate) struct Settings {
    pub(crate) transport: Transport,
    pub(crate) conversation: String,
    pub(crate) resume: bool,
}

impl Transport {
    /// Validate an existing session or thread identity for this harness.
    ///
    /// # Errors
    /// `control_binding_invalid` for a non-UUID or an unmanaged transport.
    pub fn validate_resume(self, conversation: &str) -> Result<(), RunnerError> {
        let uuid = conversation.len() == 36
            && conversation.bytes().enumerate().all(|(at, byte)| {
                if [8, 13, 18, 23].contains(&at) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        if !uuid || self == Self::Pty {
            return Err(RunnerError::refused(
                "control_binding_invalid",
                "the managed conversation must be a Claude session UUID or a Codex thread UUID",
            ));
        }
        Ok(())
    }
}
