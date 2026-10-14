//! A missing native transcript starts a new operation, while unreadable history remains refused.

use super::{claude_arguments, preflight};
use crate::error::RunnerError;
use crate::protocol::Launch;

pub(super) const NOTE: &str = "the earlier conversation left no transcript; started a new one";
const FRESH: &str = "LYS_PTY_STARTED_FRESH";

pub(super) fn mark(launch: &mut Launch) {
    launch.environment.insert(FRESH.to_owned(), "1".to_owned());
    launch
        .environment
        .insert("LYS_PTY_STATUS_NOTE".to_owned(), NOTE.to_owned());
}

pub(crate) fn started_fresh(launch: &Launch) -> bool {
    launch
        .environment
        .get(FRESH)
        .is_some_and(|value| value == "1")
}

pub(crate) fn fresh_native(launch: &Launch) -> Result<Option<String>, RunnerError> {
    if !launch.config.as_ref().is_some_and(|config| {
        !config.requires_controls && config.harness == Some(crate::tracking::Harness::ClaudeCode)
    }) {
        return Ok(None);
    }
    for (at, argument) in launch.arguments.iter().enumerate() {
        let (flag, attached) = argument
            .split_once('=')
            .map_or((argument.as_str(), None), |(flag, value)| {
                (flag, Some(value))
            });
        if flag == "--session-id" {
            let id = attached
                .or_else(|| launch.arguments.get(at + 1).map(String::as_str))
                .ok_or_else(|| {
                    RunnerError::refused(
                        "pty_conversation_invalid",
                        "the started native conversation has no UUID",
                    )
                })?;
            super::checked_uuid(id)?;
            return Ok(Some(id.to_owned()));
        }
    }
    Ok(None)
}

pub(super) fn claude(launch: &mut Launch, expected: &str) -> Result<(), RunnerError> {
    match preflight(launch) {
        Ok(()) => return Ok(()),
        Err(error) if error.name() == "pty_resume_missing" => {}
        Err(error) => return Err(error),
    }
    let mut arguments = launch.arguments.clone();
    let at = arguments
        .iter()
        .position(|argument| {
            matches!(
                argument
                    .split_once('=')
                    .map_or(argument.as_str(), |(flag, _)| flag),
                "--resume" | "-r"
            )
        })
        .ok_or_else(|| {
            RunnerError::refused(
                "pty_resume_arguments_invalid",
                "the missing transcript has no explicit resume argument",
            )
        })?;
    let attached = arguments[at].contains('=');
    arguments.remove(at);
    if !attached {
        arguments.remove(at);
    }
    launch.arguments = claude_arguments(&arguments, expected, false)?;
    mark(launch);
    Ok(())
}
