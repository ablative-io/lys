//! A peer restart retains a native conversation by passing its identity to the harness.

use super::{claude_arguments, conversation_uuid, validate_arguments};
use crate::error::RunnerError;
use crate::protocol::Launch;
use crate::tracking::Harness;

#[cfg(test)]
#[path = "restart_tests.rs"]
mod tests;

pub(crate) fn prepare_restart(
    launch: &mut Launch,
    observed: Option<&str>,
) -> Result<(), RunnerError> {
    match launch.config.as_ref().and_then(|config| config.harness) {
        Some(Harness::ClaudeCode) => {
            let mut arguments = Vec::new();
            let mut retained = None;
            let mut given = launch.arguments.iter();
            while let Some(argument) = given.next() {
                if matches!(argument.as_str(), "--session-id" | "--resume" | "-r") {
                    retained = Some(
                        given
                            .next()
                            .ok_or_else(|| {
                                RunnerError::refused(
                                    "pty_resume_arguments_invalid",
                                    "the retained native conversation argument has no UUID",
                                )
                            })?
                            .as_str(),
                    );
                } else {
                    arguments.push(argument.clone());
                }
            }
            let remembered = observed.or(retained);
            let fresh;
            let conversation = if let Some(conversation) = remembered {
                conversation
            } else {
                fresh = conversation_uuid(&format!("op-{}", lys_home::record::fresh_id()))?;
                &fresh
            };
            launch.arguments = claude_arguments(&arguments, conversation, remembered.is_some())?;
        }
        Some(Harness::Codex) => {
            if let Some(conversation) = observed {
                if let Some(at) = launch
                    .arguments
                    .iter()
                    .position(|argument| argument == "resume")
                {
                    launch.arguments.truncate(at);
                }
                launch
                    .arguments
                    .extend(["resume".to_owned(), conversation.to_owned()]);
            }
        }
        None => return Ok(()),
    }
    validate_arguments(launch)
}
