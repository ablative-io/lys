//! A proved harness identity fills an initially unbound owner exactly once.

use super::OwnerState;
use crate::error::RunnerError;
use crate::seat_owner::record::Intent;

impl OwnerState {
    /// Keep the conversation proved by the owned managed reader.
    ///
    /// # Errors
    /// The store's errors or `seat_owner_binding_mismatch` for another session or conversation.
    pub fn bind_conversation(&self, session: &str, conversation: &str) -> Result<(), RunnerError> {
        if session != self.binding.session || conversation.is_empty() {
            return Err(RunnerError::refused(
                "seat_owner_binding_mismatch",
                "the proof must name this owner's session and a nonempty conversation",
            ));
        }
        if self.view()?.binding.conversation == conversation {
            return Ok(());
        }
        let intent = super::derived_intent(session, conversation);
        self.record(
            &intent,
            &Intent::Conversation {
                session: session.to_owned(),
                conversation: conversation.to_owned(),
            },
        )?;
        Ok(())
    }
}
