//! The folders a computer holds, named so a person chooses where an agent
//! works from what is there and never types a path.
//!
//! One folder is looked in at a time: the answer names each folder directly
//! inside it. A link counts as the folder it names. A name that is not
//! UTF-8 cannot be carried in a profile, so it is not offered.
//!
//! The computer's own folder dialog is the other way: `chosen` opens it on
//! this computer's screen and answers the folder the person picked there,
//! so choosing a folder for an agent is the same act as choosing one for any
//! other program on that computer.

use std::fs::DirEntry;
use std::io;
use std::path::PathBuf;
use std::process::Command;

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

/// The folder a person picks in this computer's own folder dialog, opened in
/// `start` or in this runner's home folder. Waits for the person.
///
/// # Errors
///
/// `home_unknown` when no start is given and the runner's environment names
/// no home; `folder_invalid` when the start, or the folder picked, is not an
/// absolute UTF-8 path; `folder_choice_cancelled` when the person cancels the
/// dialog; `folder_dialog_unavailable` when this computer cannot show one,
/// with the computer's own words.
pub fn chosen(start: Option<&str>) -> Result<Answer, RunnerError> {
    let start = match start {
        Some(given) => PathBuf::from(given),
        None => std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
            RunnerError::refused(
                "home_unknown",
                "this runner's environment names no home folder",
            )
        })?,
    };
    let Some(named) = start.to_str().filter(|_| start.is_absolute()) else {
        return Err(RunnerError::refused(
            "folder_invalid",
            "the folder to start the dialog in is not an absolute UTF-8 path",
        ));
    };
    let output = dialog(named).output().map_err(|error| {
        RunnerError::refused(
            "folder_dialog_unavailable",
            format!("this computer could not open its folder dialog: {error}"),
        )
    })?;
    let folder = dialog_answer(
        output.status.success(),
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )?;
    Ok(Answer::FolderChosen { folder })
}

/// The command that shows this computer's folder dialog starting in `start`
/// and prints the folder picked. On macOS it is the system dialog through
/// osascript, brought to the front; the start folder travels as an argument,
/// never inside the script, so no path needs escaping.
#[cfg(target_os = "macos")]
fn dialog(start: &str) -> Command {
    let mut command = Command::new("/usr/bin/osascript");
    command
        .arg("-e")
        .arg("on run argv")
        .arg("-e")
        .arg("activate")
        .arg("-e")
        .arg("POSIX path of (choose folder with prompt \"Choose the folder this agent works in\" default location (POSIX file (item 1 of argv)))")
        .arg("-e")
        .arg("end run")
        .arg(start);
    command
}

/// On other systems the desktop's dialog is zenity's; a computer without it
/// says so through `folder_dialog_unavailable`.
#[cfg(not(target_os = "macos"))]
fn dialog(start: &str) -> Command {
    let mut command = Command::new("zenity");
    command
        .arg("--file-selection")
        .arg("--directory")
        .arg("--title=Choose the folder this agent works in")
        .arg(format!("--filename={}/", start.trim_end_matches('/')));
    command
}

/// What the dialog's exit says: the folder picked, trailing separator gone,
/// or the refusal by name. A cancel is the dialog's own (-128 from
/// AppleScript, an empty answer from zenity), never a fault of this computer.
fn dialog_answer(succeeded: bool, stdout: &str, stderr: &str) -> Result<String, RunnerError> {
    if succeeded {
        let picked = stdout.trim();
        let folder = if picked.len() > 1 {
            picked.trim_end_matches('/')
        } else {
            picked
        };
        if !folder.starts_with('/') {
            return Err(RunnerError::refused(
                "folder_invalid",
                format!("the folder dialog answered with {picked:?}, which is not an absolute path"),
            ));
        }
        return Ok(folder.to_owned());
    }
    let said = stderr.trim();
    if said.contains("(-128)") || said.contains("User canceled") || (said.is_empty() && stdout.trim().is_empty()) {
        return Err(RunnerError::refused(
            "folder_choice_cancelled",
            "the folder dialog was cancelled on this computer, so no folder was chosen",
        ));
    }
    Err(RunnerError::refused(
        "folder_dialog_unavailable",
        format!("this computer could not show its folder dialog: {said}"),
    ))
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

#[cfg(test)]
mod tests {
    use super::dialog_answer;
    use crate::error::RunnerError;

    fn refusal(answer: Result<String, RunnerError>) -> String {
        match answer {
            Err(RunnerError::Refused { refusal, .. }) => refusal,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_picked_folder_loses_its_trailing_separator_and_newline() {
        assert_eq!(
            dialog_answer(true, "/Users/ada/Developer/receipts/\n", "").ok(),
            Some("/Users/ada/Developer/receipts".to_owned())
        );
        assert_eq!(dialog_answer(true, "/\n", "").ok(), Some("/".to_owned()));
    }

    #[test]
    fn a_relative_answer_is_refused_as_invalid() {
        assert_eq!(refusal(dialog_answer(true, "receipts\n", "")), "folder_invalid");
    }

    #[test]
    fn a_cancel_is_the_persons_and_named_so() {
        assert_eq!(
            refusal(dialog_answer(false, "", "execution error: User canceled. (-128)\n")),
            "folder_choice_cancelled"
        );
        assert_eq!(refusal(dialog_answer(false, "", "")), "folder_choice_cancelled");
    }

    #[test]
    fn any_other_failure_names_the_computers_words() {
        let answer = dialog_answer(false, "", "osascript: no display available\n");
        assert_eq!(refusal(answer), "folder_dialog_unavailable");
    }
}
