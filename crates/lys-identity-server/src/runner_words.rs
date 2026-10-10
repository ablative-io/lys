//! The wake-up slot's words for a wake (AGENTS-001 R3), asked of the words
//! owner by `runner_api::wake`; kept beside it so the routes file stays under
//! the length gate.

use std::sync::Arc;

use crate::routes::AppState;

/// The wake-up slot's words for `session` of `agent`, carrying `message`;
/// none when no layer sets the slot or the words cannot be rendered.
pub(crate) fn wake_words(
    state: &Arc<AppState>,
    agent: &str,
    session: &str,
    message: &str,
) -> Option<String> {
    use crate::words_state::Slot;
    let words = state.words.as_ref()?;
    let resolved =
        crate::words_store::resolve(words, Slot::WakeUp, Some(agent), Some(session)).ok()?;
    if resolved.source == "built_in" {
        return None;
    }
    let mut numbers = std::collections::BTreeMap::new();
    numbers.insert("message".to_owned(), message.to_owned());
    match crate::words_api::deliverable(
        state,
        Slot::WakeUp,
        Some(agent),
        Some(session),
        numbers,
        None,
    ) {
        Ok(delivered) => Some(delivered.text),
        Err(error) => {
            (state.say)(&format!(
                "wake of {agent}: the wake_up words could not be rendered, the plain message stands: {error}"
            ));
            None
        }
    }
}
