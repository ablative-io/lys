//! Terminal seat starts choose a native conversation before the launch is kept.

use std::str::FromStr;

use lys_identity::OperationId;
use lys_runner::RunnerError;
use lys_runner::harness_control::Transport;
use lys_runner::tracking::Harness;

use crate::error::ServerError;
use crate::seats_state::Seat;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Plan {
    pub(super) resume: Option<String>,
    pub(super) expected: Option<String>,
}

pub(super) fn pty(
    held: &Seat,
    operation: &str,
    explicit: Option<&str>,
    fresh: bool,
    harness: Option<Harness>,
) -> Result<Plan, ServerError> {
    OperationId::from_str(operation)?;
    if held.session.as_deref() == Some(operation) {
        return Err(RunnerError::refused(
            "pty_launch_replay_unresolved",
            "the retained operation must replay its kept launch before choosing a conversation",
        )
        .into());
    }
    if fresh && explicit.is_some() {
        return Err(RunnerError::refused(
            "pty_resume_invalid",
            "Start again cannot also select an existing conversation to resume",
        )
        .into());
    }
    let transport = match harness {
        Some(Harness::ClaudeCode) => Transport::Claude,
        Some(Harness::Codex) => Transport::Codex,
        None => {
            if explicit.is_some() {
                return Err(RunnerError::refused(
                    "pty_resume_unavailable",
                    "an existing conversation requires a declared Claude Code or Codex harness",
                )
                .into());
            }
            return Ok(Plan {
                resume: None,
                expected: None,
            });
        }
    };
    if !fresh && let Some(conversation) = explicit.or(held.harness_session.as_deref()) {
        transport.validate_resume(conversation).map_err(|error| {
            RunnerError::refused(&error.name(), format!("{error}; choose Start again"))
        })?;
        return Ok(Plan {
            resume: Some(conversation.to_owned()),
            expected: Some(conversation.to_owned()),
        });
    }
    if !fresh && held.session.is_some() {
        return Err(RunnerError::refused(
            "pty_resume_unavailable",
            "the previous terminal session has no remembered native conversation; choose Start again to begin a new conversation",
        )
        .into());
    }
    let expected = match harness {
        Some(Harness::ClaudeCode) => {
            Some(lys_runner::pty_conversation::conversation_uuid(operation)?)
        }
        Some(Harness::Codex) | None => None,
    };
    Ok(Plan {
        resume: None,
        expected,
    })
}

#[cfg(test)]
mod tests;
