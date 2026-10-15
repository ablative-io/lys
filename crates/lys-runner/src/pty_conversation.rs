//! Native terminal conversations have an explicit identity before launch.

use crate::error::RunnerError;

mod native_identity;
use crate::protocol::Launch;
pub(crate) use native_identity::{fresh_native, started_fresh};

mod codex;
mod restart;
pub(crate) use restart::prepare_restart;

#[cfg(test)]
#[path = "pty_conversation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pty_conversation_prepare_tests.rs"]
mod prepare_tests;

/// Derive the native UUID from a Lys operation's complete hexadecimal id.
///
/// # Errors
/// Refuses ids that are not `op-` followed by exactly 32 ASCII hex digits.
pub fn conversation_uuid(session: &str) -> Result<String, RunnerError> {
    let hex = session.strip_prefix("op-").ok_or_else(|| {
        RunnerError::refused(
            "pty_conversation_invalid",
            "the native conversation needs a Lys op- id with 32 hexadecimal digits",
        )
    })?;
    if hex.len() != 32 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RunnerError::refused(
            "pty_conversation_invalid",
            "the native conversation needs a Lys op- id with 32 hexadecimal digits",
        ));
    }
    let hex = hex.to_ascii_lowercase();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

fn checked_uuid(conversation: &str) -> Result<(), RunnerError> {
    if conversation.len() == 36
        && conversation.bytes().enumerate().all(|(at, byte)| {
            if [8, 13, 18, 23].contains(&at) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
    {
        return Ok(());
    }
    Err(RunnerError::refused(
        "pty_conversation_invalid",
        "the native conversation must be an explicit UUID",
    ))
}

/// Preserve terminal configuration arguments and bind one native conversation.
///
/// # Errors
/// Refuses invalid UUIDs and arguments that select or discard a conversation.
pub fn claude_arguments(
    given: &[String],
    conversation: &str,
    resume: bool,
) -> Result<Vec<String>, RunnerError> {
    checked_uuid(conversation)?;
    for argument in given {
        let flag = argument
            .split_once('=')
            .map_or(argument.as_str(), |(flag, _)| flag);
        if matches!(
            flag,
            "--session-id"
                | "--resume"
                | "-r"
                | "--continue"
                | "-c"
                | "--fork-session"
                | "--no-session-persistence"
                | "--"
        ) || (!flag.starts_with("--") && (flag.starts_with("-r") || flag.starts_with("-c")))
        {
            return Err(RunnerError::refused(
                "pty_conversation_conflict",
                "the given arguments replace, discard or hide the native conversation; remove the conflicting conversation argument",
            ));
        }
    }
    let mut arguments = given.to_vec();
    arguments.extend([
        if resume { "--resume" } else { "--session-id" }.to_owned(),
        conversation.to_owned(),
    ]);
    Ok(arguments)
}

fn resume_refused(name: &str, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(name, format!("{reason}; choose Start again"))
}

fn claude_resume(arguments: &[String]) -> Result<Option<&str>, RunnerError> {
    let mut resume = None;
    for (at, argument) in arguments.iter().enumerate() {
        let (flag, attached) = argument
            .split_once('=')
            .map_or((argument.as_str(), None), |(flag, value)| {
                (flag, Some(value))
            });
        if matches!(flag, "--continue" | "-c" | "--fork-session")
            || (!flag.starts_with("--") && flag.starts_with("-c"))
        {
            return Err(resume_refused(
                "pty_resume_arguments_invalid",
                "native resume must name its exact conversation and must not fork it",
            ));
        }
        if matches!(flag, "--resume" | "-r") {
            if resume.is_some() {
                return Err(resume_refused(
                    "pty_resume_arguments_invalid",
                    "native resume names more than one conversation",
                ));
            }
            resume = Some(
                attached
                    .or_else(|| arguments.get(at + 1).map(String::as_str))
                    .ok_or_else(|| {
                        resume_refused(
                            "pty_resume_arguments_invalid",
                            "native resume names no conversation",
                        )
                    })?,
            );
        } else if argument.starts_with("-r") && !argument.starts_with("--") {
            return Err(resume_refused(
                "pty_resume_arguments_invalid",
                "native resume must use a separate explicit conversation UUID",
            ));
        }
    }
    if resume.is_some()
        && arguments.iter().any(|argument| {
            matches!(
                argument
                    .split_once('=')
                    .map_or(argument.as_str(), |(flag, _)| flag),
                "--session-id" | "--no-session-persistence" | "--"
            )
        })
    {
        return Err(resume_refused(
            "pty_resume_arguments_invalid",
            "native resume has conflicting identity or persistence arguments",
        ));
    }
    Ok(resume)
}

/// Validate only the declared harness's resume argument shape.
fn validate_arguments(launch: &Launch) -> Result<(), RunnerError> {
    match launch.config.as_ref().and_then(|config| config.harness) {
        Some(crate::tracking::Harness::ClaudeCode) => {
            if let Some(conversation) = claude_resume(&launch.arguments)? {
                checked_uuid(conversation)
                    .map_err(|error| resume_refused("pty_resume_conversation_invalid", error))?;
            }
        }
        Some(crate::tracking::Harness::Codex) => {
            codex::requested(&launch.arguments)?;
        }
        None => {}
    }
    Ok(())
}

/// Bind fresh Lys operation terminals and preserve requested native resumes.
/// An explicit identity is preserved only when it agrees with the operation.
///
/// # Errors
/// Refuses invalid operation ids, conflicting native identities or malformed resume arguments.
pub fn prepare(launch: &mut Launch) -> Result<(), RunnerError> {
    if !launch.session.starts_with("op-") {
        return validate_arguments(launch);
    }
    let harness = launch.config.as_ref().and_then(|config| config.harness);
    if harness == Some(crate::tracking::Harness::Codex) {
        return codex::prepare(launch);
    }
    if harness != Some(crate::tracking::Harness::ClaudeCode) {
        return validate_arguments(launch);
    }
    let expected = conversation_uuid(&launch.session)?;
    if claude_resume(&launch.arguments)?.is_some() {
        return validate_arguments(launch);
    }
    let mut identity = None;
    for (at, argument) in launch.arguments.iter().enumerate() {
        let (flag, attached) = argument
            .split_once('=')
            .map_or((argument.as_str(), None), |(flag, value)| {
                (flag, Some(value))
            });
        if matches!(flag, "--no-session-persistence" | "--") {
            return Err(RunnerError::refused(
                "pty_conversation_conflict",
                "the given arguments discard or hide the native conversation",
            ));
        }
        if flag != "--session-id" {
            continue;
        }
        if identity.is_some() {
            return Err(RunnerError::refused(
                "pty_conversation_conflict",
                "the given arguments name more than one native conversation",
            ));
        }
        let conversation = attached
            .or_else(|| launch.arguments.get(at + 1).map(String::as_str))
            .ok_or_else(|| {
                RunnerError::refused(
                    "pty_conversation_invalid",
                    "the native identity has no UUID",
                )
            })?;
        checked_uuid(conversation)?;
        if conversation != expected {
            return Err(RunnerError::refused(
                "pty_conversation_conflict",
                "the given native conversation differs from the Lys operation's UUID",
            ));
        }
        identity = Some(conversation);
    }
    if identity.is_none() {
        launch.arguments = claude_arguments(&launch.arguments, &expected, false)?;
    }
    validate_arguments(launch)
}
