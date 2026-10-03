//! The folders a computer holds, named so a person chooses where an agent
//! works from what is there and never types a path.
//!
//! One folder is looked in at a time: the answer names each folder directly
//! inside it. A link counts as the folder it names. A name that is not
//! UTF-8 cannot be carried in a profile, so it is not offered.

use std::fs::DirEntry;
use std::io;
use std::path::PathBuf;

use crate::error::RunnerError;
use crate::protocol::Answer;

/// The folders directly inside `under`, or inside this runner's home folder
/// when none is given.
///
/// # Errors
///
/// `home_unknown` when no folder is given and the runner's environment names
/// no home; `folder_invalid` when the folder is not an absolute UTF-8 path;
/// `folder_unreadable` when it, or an entry in it, cannot be read.
pub fn inside(under: Option<&str>) -> Result<Answer, RunnerError> {
    let under = match under {
        Some(given) => PathBuf::from(given),
        None => std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
            RunnerError::refused(
                "home_unknown",
                "this runner's environment names no home folder",
            )
        })?,
    };
    let Some(named) = under.to_str().filter(|_| under.is_absolute()) else {
        return Err(RunnerError::refused(
            "folder_invalid",
            "the folder to look in is not an absolute UTF-8 path",
        ));
    };
    let unreadable = |error: io::Error| {
        RunnerError::refused(
            "folder_unreadable",
            format!("{named} could not be read: {error}"),
        )
    };
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(&under).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        if !is_folder(&entry).map_err(unreadable)? {
            continue;
        }
        if let Ok(name) = entry.file_name().into_string() {
            folders.push(name);
        }
    }
    folders.sort();
    Ok(Answer::Folders {
        under: named.to_owned(),
        folders,
    })
}

/// Whether `entry` is a folder. A link is the folder it names; a link that
/// names nothing, or names what this runner may not look at, is not a
/// folder an agent can work in.
fn is_folder(entry: &DirEntry) -> io::Result<bool> {
    let kind = entry.file_type()?;
    if !kind.is_symlink() {
        return Ok(kind.is_dir());
    }
    match std::fs::metadata(entry.path()) {
        Ok(named) => Ok(named.is_dir()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}
