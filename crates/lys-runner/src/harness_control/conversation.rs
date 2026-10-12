//! Resume identities are checked before a managed launch can change a session.

use super::Transport;
use crate::error::RunnerError;

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

pub(super) fn is_fresh(resume: &bool) -> bool {
    !*resume
}
