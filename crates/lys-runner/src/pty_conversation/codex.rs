//! Codex terminal resume proves its local rollout before the CLI starts.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{checked_uuid, resume_config, resume_directory, resume_refused, transcript_file};
use crate::error::RunnerError;
use crate::protocol::Launch;

#[path = "codex_version.rs"]
mod version;

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;

#[derive(Deserialize)]
struct SessionMeta {
    #[serde(rename = "type")]
    kind: String,
    payload: Binding,
}

#[derive(Deserialize)]
struct Binding {
    id: String,
    cwd: PathBuf,
    session_id: Option<String>,
}

fn requested(arguments: &[String]) -> Result<Option<&str>, RunnerError> {
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

fn candidates(
    directory: &Path,
    depth: usize,
    ending: &str,
    found: &mut Vec<PathBuf>,
) -> Result<(), RunnerError> {
    let entries = std::fs::read_dir(directory).map_err(|error| {
        resume_refused(
            if depth == 0 && error.kind() == std::io::ErrorKind::NotFound {
                "pty_resume_missing"
            } else {
                "pty_resume_unreadable"
            },
            format!("the native Codex sessions hierarchy cannot be read: {error}"),
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            resume_refused(
                "pty_resume_unreadable",
                format!("a Codex sessions entry cannot be read: {error}"),
            )
        })?;
        let name = entry.file_name();
        let bytes = name.as_encoded_bytes();
        let matching =
            depth == 3 && bytes.starts_with(b"rollout-") && bytes.ends_with(ending.as_bytes());
        let date_directory = depth < 3
            && bytes.len() == if depth == 0 { 4 } else { 2 }
            && bytes.iter().all(u8::is_ascii_digit);
        if !matching && !date_directory {
            continue;
        }
        let kind = entry.file_type().map_err(|error| {
            resume_refused(
                "pty_resume_unreadable",
                format!("a Codex sessions entry cannot be inspected: {error}"),
            )
        })?;
        if matching {
            if !kind.is_file() {
                return Err(resume_refused(
                    "pty_resume_unreadable",
                    "the requested native Codex rollout is not a regular file",
                ));
            }
            found.push(entry.path());
        } else if kind.is_dir() {
            candidates(&entry.path(), depth + 1, ending, found)?;
        } else {
            return Err(resume_refused(
                "pty_resume_unreadable",
                "a native Codex date directory is not a directory",
            ));
        }
    }
    Ok(())
}

fn verify(path: &Path, directory: &Path, conversation: &str) -> Result<(), RunnerError> {
    let mut first = String::new();
    BufReader::new(transcript_file(path)?)
        .read_line(&mut first)
        .map_err(|error| {
            resume_refused(
                "pty_resume_unreadable",
                format!("native Codex metadata cannot be read: {error}"),
            )
        })?;
    if !first.ends_with('\n') || !first.trim_start().starts_with('{') {
        return Err(resume_refused(
            "pty_resume_metadata_invalid",
            "the native Codex rollout has no complete first metadata object",
        ));
    }
    let metadata: SessionMeta = serde_json::from_str(&first).map_err(|error| {
        resume_refused(
            "pty_resume_metadata_invalid",
            format!("native Codex metadata is invalid: {error}"),
        )
    })?;
    if metadata.kind != "session_meta" {
        return Err(resume_refused(
            "pty_resume_metadata_invalid",
            "the native Codex rollout does not begin with session_meta",
        ));
    }
    if metadata.payload.id != conversation
        || metadata
            .payload
            .session_id
            .as_deref()
            .is_some_and(|id| id != conversation)
    {
        return Err(resume_refused(
            "pty_resume_other_session",
            "the native Codex rollout names a different conversation",
        ));
    }
    if !metadata.payload.cwd.is_absolute() {
        return Err(resume_refused(
            "pty_resume_metadata_invalid",
            "the native Codex rollout names a relative working directory",
        ));
    }
    let recorded = metadata.payload.cwd.canonicalize().map_err(|error| {
        resume_refused(
            "pty_resume_metadata_unreadable",
            format!("the native Codex rollout's working directory cannot be resolved: {error}"),
        )
    })?;
    if recorded != directory {
        return Err(resume_refused(
            "pty_resume_other_directory",
            "the native Codex rollout belongs to a different working directory",
        ));
    }
    Ok(())
}

pub(super) fn preflight(launch: &Launch) -> Result<(), RunnerError> {
    let Some(conversation) = requested(&launch.arguments)? else {
        return Ok(());
    };
    let directory = resume_directory(launch)?;
    let config = resume_config(launch, "CODEX_HOME", ".codex")?;
    let mut found = Vec::new();
    candidates(
        &config.join("sessions"),
        0,
        &format!("-{conversation}.jsonl"),
        &mut found,
    )?;
    if found.is_empty() {
        return Err(resume_refused(
            "pty_resume_missing",
            "the requested native Codex rollout is absent from this computer's sessions hierarchy",
        ));
    }
    if found.len() != 1 {
        return Err(resume_refused(
            "pty_resume_ambiguous",
            "more than one native Codex rollout names the requested conversation",
        ));
    }
    verify(&found[0], &directory, conversation)?;
    version::prove(launch)
}

/// A fresh start remains available even when native resume has no reviewed proof.
pub(super) fn prepare(launch: &mut Launch) -> Result<(), RunnerError> {
    preflight(launch)?;
    if requested(&launch.arguments)?.is_none()
        && let Some(note) = version::fresh_note(launch)?
    {
        launch
            .environment
            .insert("LYS_PTY_STATUS_NOTE".to_owned(), note);
    }
    Ok(())
}
