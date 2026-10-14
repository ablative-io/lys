//! Native terminal conversations have an explicit identity before launch.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use rustix::fs::{Mode, OFlags, open};
use serde::Deserialize;

use crate::error::RunnerError;

mod missing;
use crate::protocol::Launch;
pub(crate) use missing::{fresh_native, started_fresh};

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

fn transcript_path(launch: &Launch, conversation: &str) -> Result<(PathBuf, PathBuf), RunnerError> {
    checked_uuid(conversation)
        .map_err(|error| resume_refused("pty_resume_conversation_invalid", error))?;
    let directory = resume_directory(launch)?;
    let cwd = directory.to_str().ok_or_else(|| {
        resume_refused(
            "pty_resume_directory_invalid",
            "the resolved working directory is not UTF-8",
        )
    })?;
    let config = resume_config(launch, "CLAUDE_CONFIG_DIR", ".claude")?;
    let path = config
        .join("projects")
        .join(lys_home::harness::claude_code::projects_slug(cwd))
        .join(format!("{conversation}.jsonl"));
    Ok((path, directory))
}

fn resume_directory(launch: &Launch) -> Result<PathBuf, RunnerError> {
    let requested = Path::new(&launch.directory);
    if !requested.is_absolute() {
        return Err(resume_refused(
            "pty_resume_directory_invalid",
            "the native resume needs its explicit absolute working directory",
        ));
    }
    let directory = requested.canonicalize().map_err(|error| {
        resume_refused(
            "pty_resume_directory_unreadable",
            format!("the requested working directory cannot be resolved: {error}"),
        )
    })?;
    if !directory.is_dir() {
        return Err(resume_refused(
            "pty_resume_directory_invalid",
            "the requested working directory is not a directory",
        ));
    }
    Ok(directory)
}

fn resume_config(launch: &Launch, variable: &str, hidden: &str) -> Result<PathBuf, RunnerError> {
    let configured = launch.environment.contains_key(variable);
    let effective_home = if configured {
        None
    } else {
        launch.environment.get("HOME").map(Path::new)
    };
    let resolved = crate::trust::resolve_config_home(variable, effective_home, &launch.environment)
        .map_err(|error| resume_refused("pty_resume_home_unknown", error))?;
    let config = if configured {
        resolved.path
    } else {
        resolved.path.join(hidden)
    };
    Ok(config)
}

#[derive(Deserialize)]
struct Metadata {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    cwd: Option<String>,
}

fn verify_metadata(
    reader: impl BufRead,
    directory: &Path,
    conversation: &str,
) -> Result<(), RunnerError> {
    let mut proved = false;
    for (at, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            resume_refused(
                "pty_resume_unreadable",
                format!(
                    "native transcript metadata line {} cannot be read: {error}",
                    at + 1
                ),
            )
        })?;
        if !line.trim_start().starts_with('{') {
            return Err(resume_refused(
                "pty_resume_metadata_invalid",
                format!(
                    "native transcript metadata line {} is not a JSON object",
                    at + 1
                ),
            ));
        }
        let metadata: Metadata = serde_json::from_str(&line).map_err(|error| {
            resume_refused(
                "pty_resume_metadata_invalid",
                format!(
                    "native transcript metadata line {} is invalid: {error}",
                    at + 1
                ),
            )
        })?;
        if metadata
            .session_id
            .as_deref()
            .is_some_and(|id| id != conversation)
        {
            return Err(resume_refused(
                "pty_resume_other_session",
                "the native transcript names a different conversation",
            ));
        }
        if let Some(cwd) = &metadata.cwd {
            let recorded = Path::new(cwd);
            if !recorded.is_absolute() {
                return Err(resume_refused(
                    "pty_resume_metadata_invalid",
                    "the native transcript names a relative working directory",
                ));
            }
            let recorded = recorded.canonicalize().map_err(|error| {
                resume_refused(
                    "pty_resume_metadata_unreadable",
                    format!(
                        "the native transcript's working directory cannot be resolved: {error}"
                    ),
                )
            })?;
            if recorded != directory {
                return Err(resume_refused(
                    "pty_resume_other_directory",
                    "the native transcript belongs to a different working directory",
                ));
            }
        }
        proved |= metadata.session_id.is_some() && metadata.cwd.is_some();
    }
    if !proved {
        return Err(resume_refused(
            "pty_resume_unproved",
            "the native transcript has no record proving this conversation and working directory",
        ));
    }
    Ok(())
}

/// Prove that Claude's local transcript belongs to this native resume.
/// Only top-level identity metadata is retained; message fields are ignored.
///
/// # Errors
/// Refuses missing, unreadable, invalid or differently bound transcripts.
/// Every refusal directs the person to Start again; none starts a fresh run.
pub fn preflight_claude_resume(
    launch: &Launch,
    conversation: &str,
) -> Result<PathBuf, RunnerError> {
    let (path, directory) = transcript_path(launch, conversation)?;
    let inspected = || {
        let file = transcript_file(&path)?;
        verify_metadata(BufReader::new(file), &directory, conversation)
    };
    inspected().map_err(|error| {
        RunnerError::refused(
            &error.name(),
            format!("{error}; transcript path {}", path.display()),
        )
    })?;
    Ok(path)
}

fn transcript_file(path: &Path) -> Result<File, RunnerError> {
    // A replaced FIFO must be refused by file type without blocking on its writer.
    let file = open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(|error| {
        let error = std::io::Error::from(error);
        resume_refused(
            if error.kind() == std::io::ErrorKind::NotFound {
                "pty_resume_missing"
            } else {
                "pty_resume_unreadable"
            },
            if error.kind() == std::io::ErrorKind::NotFound {
                "the earlier conversation's transcript is gone; Start again begins a new one"
                    .to_owned()
            } else {
                format!("the requested native transcript cannot be opened: {error}")
            },
        )
    })?;
    let metadata = file.metadata().map_err(|error| {
        resume_refused(
            "pty_resume_unreadable",
            format!("the requested native transcript cannot be inspected: {error}"),
        )
    })?;
    if !metadata.is_file() {
        return Err(resume_refused(
            "pty_resume_unreadable",
            "the requested native transcript is not a regular file",
        ));
    }
    Ok(file)
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

/// Refuse a requested native resume before a manual terminal process starts.
/// Dispatch only the harness declared by the prepared launch configuration.
///
/// # Errors
/// Refuses ambiguous resume arguments or unproved local identity and folder.
pub fn preflight(launch: &Launch) -> Result<(), RunnerError> {
    match launch.config.as_ref().and_then(|config| config.harness) {
        Some(crate::tracking::Harness::ClaudeCode) => {
            if let Some(conversation) = claude_resume(&launch.arguments)? {
                preflight_claude_resume(launch, conversation)?;
            }
        }
        Some(crate::tracking::Harness::Codex) => codex::preflight(launch)?,
        None => {}
    }
    Ok(())
}

/// Bind fresh Lys operation terminals and prove their requested native resumes.
/// An explicit identity is preserved only when it agrees with the operation.
///
/// # Errors
/// Refuses invalid operation ids, conflicting native identities or resume proof.
pub fn prepare(launch: &mut Launch) -> Result<(), RunnerError> {
    if !launch.session.starts_with("op-") {
        return Ok(());
    }
    let harness = launch.config.as_ref().and_then(|config| config.harness);
    if harness == Some(crate::tracking::Harness::Codex) {
        return codex::prepare(launch);
    }
    if harness != Some(crate::tracking::Harness::ClaudeCode) {
        return preflight(launch);
    }
    let expected = conversation_uuid(&launch.session)?;
    if claude_resume(&launch.arguments)?.is_some() {
        return missing::claude(launch, &expected);
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
    preflight(launch)
}
