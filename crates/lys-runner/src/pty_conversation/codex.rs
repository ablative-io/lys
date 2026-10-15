//! Codex owns thread lookup; Lys validates only explicit resume argument shape.

use super::{checked_uuid, resume_refused};
use crate::error::RunnerError;
use crate::protocol::Launch;

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;

pub(super) fn requested(arguments: &[String]) -> Result<Option<&str>, RunnerError> {
    let Some(at) = arguments.iter().position(|argument| argument == "resume") else {
        return Ok(None);
    };
    if at + 2 != arguments.len()
        || arguments.iter().any(|argument| {
            argument == "--last" || argument.starts_with("--last=") || argument == "--"
        })
    {
        return Err(resume_refused(
            "pty_resume_arguments_invalid",
            "Codex native resume must end with resume and one explicit conversation UUID",
        ));
    }
    let conversation = &arguments[at + 1];
    checked_uuid(conversation)
        .map_err(|error| resume_refused("pty_resume_conversation_invalid", error))?;
    Ok(Some(conversation))
}

pub(super) fn prepare(launch: &mut Launch) -> Result<(), RunnerError> {
    requested(&launch.arguments)?;
    Ok(())
}
