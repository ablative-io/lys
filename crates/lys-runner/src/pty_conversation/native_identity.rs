//! Native conversation identity and historical fresh-start status for running sessions.

use crate::error::RunnerError;
use crate::protocol::Launch;

const FRESH: &str = "LYS_PTY_STARTED_FRESH";

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
        // The conversation the program was started in, whether new (--session-id) or resumed (--resume).
        if matches!(flag, "--session-id" | "--resume" | "-r") {
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
