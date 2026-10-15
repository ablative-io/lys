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
    #[cfg(target_os = "macos")]
    let mut command = dialog(named);
    #[cfg(not(target_os = "macos"))]
    let mut command = dialog(named)?;
    let output = command.output().map_err(|error| {
        RunnerError::refused(
            "folder_dialog_unavailable",
            format!("this computer could not open its folder dialog: {error}"),
        )
    })?;
    let folder = dialog_answer(
        output.status.code(),
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
fn dialog(start: &str) -> Result<Command, RunnerError> {
    let mut command = Command::new("zenity");
    command
        .arg("--file-selection")
        .arg("--directory")
        .arg("--title=Choose the folder this agent works in")
        .arg(format!("--filename={}/", start.trim_end_matches('/')));
    #[cfg(target_os = "linux")]
    {
        let desktop = Command::new("systemctl")
            .args(["--user", "show-environment"])
            .output()
            .map_err(|error| {
                dialog_unavailable(&format!(
                    "the desktop environment could not be read: {error}"
                ))
            })?;
        if !desktop.status.success() {
            return Err(dialog_unavailable(
                "systemctl --user show-environment could not read the desktop environment",
            ));
        }
        let environment = std::str::from_utf8(&desktop.stdout)
            .map_err(|_| dialog_unavailable("the desktop environment is not valid UTF-8"))?;
        let environment = desktop_environment(environment, |name| std::env::var(name).ok())?;
        for name in DESKTOP_VARIABLES {
            command.env_remove(name);
        }
        command.envs(environment);
    }
    Ok(command)
}

#[cfg(any(target_os = "linux", test))]
const DESKTOP_VARIABLES: [&str; 4] = [
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "DBUS_SESSION_BUS_ADDRESS",
];

#[cfg(any(target_os = "linux", test))]
fn dialog_unavailable(words: &str) -> RunnerError {
    RunnerError::refused("folder_dialog_unavailable", words)
}

#[cfg(any(target_os = "linux", test))]
fn desktop_environment(
    manager: &str,
    inherited: impl Fn(&str) -> Option<String>,
) -> Result<std::collections::BTreeMap<String, String>, RunnerError> {
    let mut environment = std::collections::BTreeMap::new();
    for name in DESKTOP_VARIABLES {
        if let Some(value) = inherited(name).filter(|value| !value.is_empty()) {
            environment.insert(name.to_owned(), value);
        }
    }
    for line in manager.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if DESKTOP_VARIABLES.contains(&name) {
            let value = desktop_value(value)?;
            environment.remove(name);
            if !value.is_empty() {
                environment.insert(name.to_owned(), value);
            }
        }
    }
    if !environment.contains_key("DISPLAY") && !environment.contains_key("WAYLAND_DISPLAY") {
        return Err(dialog_unavailable(
            "this computer has no DISPLAY or WAYLAND_DISPLAY for a desktop folder dialog; sign in to its desktop, then try again",
        ));
    }
    Ok(environment)
}

#[cfg(any(target_os = "linux", test))]
fn desktop_value(value: &str) -> Result<String, RunnerError> {
    let mut decoded = String::new();
    let mut quote = None;
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(held), closing) if held == closing => quote = None,
            (None | Some('"'), '\\') => {
                let next = characters.next().ok_or_else(|| {
                    dialog_unavailable("a desktop environment value has an unfinished escape")
                })?;
                if quote == Some('"') && !matches!(next, '$' | '`' | '"' | '\\' | '\n') {
                    decoded.push('\\');
                }
                decoded.push(next);
            }
            (None, '$') if characters.clone().next() == Some('\'') => {
                return Err(dialog_unavailable(
                    "a desktop environment value uses unsupported shell quoting",
                ));
            }
            _ => decoded.push(character),
        }
    }
    if quote.is_some() || decoded.chars().any(char::is_control) {
        return Err(dialog_unavailable(
            "a desktop environment value has invalid quoting or control characters",
        ));
    }
    Ok(decoded)
}

/// What the dialog's exit says: the folder picked, trailing separator gone,
/// or the refusal by name. A cancel is the dialog's own (-128 from
/// `AppleScript`, an empty answer from zenity), never a fault of this computer.
fn dialog_answer(status: Option<i32>, stdout: &str, stderr: &str) -> Result<String, RunnerError> {
    if status == Some(0) {
        let picked = stdout.trim();
        let folder = if picked.len() > 1 {
            picked.trim_end_matches('/')
        } else {
            picked
        };
        if !folder.starts_with('/') {
            return Err(RunnerError::refused(
                "folder_invalid",
                format!(
                    "the folder dialog answered with {picked:?}, which is not an absolute path"
                ),
            ));
        }
        return Ok(folder.to_owned());
    }
    let said = stderr.trim();
    if said.contains("(-128)")
        || said.contains("User canceled")
        || (status == Some(1) && stdout.trim().is_empty())
    {
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
    use super::{desktop_environment, dialog_answer};
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
            dialog_answer(Some(0), "/Users/ada/Developer/receipts/\n", "").ok(),
            Some("/Users/ada/Developer/receipts".to_owned())
        );
        assert_eq!(dialog_answer(Some(0), "/\n", "").ok(), Some("/".to_owned()));
    }

    #[test]
    fn a_relative_answer_is_refused_as_invalid() {
        assert_eq!(
            refusal(dialog_answer(Some(0), "receipts\n", "")),
            "folder_invalid"
        );
    }

    #[test]
    fn a_cancel_is_the_persons_and_named_so() {
        assert_eq!(
            refusal(dialog_answer(
                Some(1),
                "",
                "execution error: User canceled. (-128)\n"
            )),
            "folder_choice_cancelled"
        );
        assert_eq!(
            refusal(dialog_answer(Some(1), "", "")),
            "folder_choice_cancelled"
        );
    }

    #[test]
    fn any_other_failure_names_the_computers_words() {
        let answer = dialog_answer(Some(2), "", "osascript: no display available\n");
        assert_eq!(refusal(answer), "folder_dialog_unavailable");
    }

    #[test]
    fn zenity_cancel_ignores_gtk_warnings_but_other_exits_remain_failures() {
        let warning = "Gtk-WARNING: theme could not be loaded";
        assert_eq!(
            refusal(dialog_answer(Some(1), " \n", warning)),
            "folder_choice_cancelled"
        );
        assert_eq!(
            refusal(dialog_answer(Some(2), "", warning)),
            "folder_dialog_unavailable"
        );
        assert_eq!(
            refusal(dialog_answer(None, "", "")),
            "folder_dialog_unavailable"
        );
        assert_eq!(
            refusal(dialog_answer(Some(1), "/project", warning)),
            "folder_dialog_unavailable"
        );
    }

    #[test]
    fn the_user_managers_desktop_reaches_a_service_without_a_display() -> Result<(), RunnerError> {
        let environment = desktop_environment(
            "DISPLAY=:7\nWAYLAND_DISPLAY=wayland-2\nXAUTHORITY='/home/ada/a file'\nDBUS_SESSION_BUS_ADDRESS=\"unix:path=/run/user/1000/bus\"\nUNRELATED=omitted\n",
            |_| None,
        )?;
        assert_eq!(environment.len(), 4);
        assert_eq!(environment.get("DISPLAY").map(String::as_str), Some(":7"));
        assert_eq!(
            environment.get("WAYLAND_DISPLAY").map(String::as_str),
            Some("wayland-2")
        );
        assert_eq!(
            environment.get("XAUTHORITY").map(String::as_str),
            Some("/home/ada/a file")
        );
        assert_eq!(
            environment
                .get("DBUS_SESSION_BUS_ADDRESS")
                .map(String::as_str),
            Some("unix:path=/run/user/1000/bus")
        );
        Ok(())
    }

    #[test]
    fn desktop_values_override_stale_service_values_and_keep_known_inherited_values()
    -> Result<(), RunnerError> {
        let environment = desktop_environment("DISPLAY=:9\n", |name| match name {
            "DISPLAY" => Some(":1".to_owned()),
            "XAUTHORITY" => Some("/home/ada/.Xauthority".to_owned()),
            _ => None,
        })?;
        assert_eq!(environment.get("DISPLAY").map(String::as_str), Some(":9"));
        assert!(environment.contains_key("XAUTHORITY"));
        assert!(desktop_environment("WAYLAND_DISPLAY=wayland-0\n", |_| None).is_ok());
        assert!(
            desktop_environment("", |name| (name == "DISPLAY").then(|| ":0".to_owned())).is_ok()
        );
        Ok(())
    }

    #[test]
    fn no_desktop_or_an_invalid_selected_environment_is_refused_in_words() {
        for manager in [
            "",
            "DISPLAY=\nWAYLAND_DISPLAY=\n",
            "DISPLAY='unfinished",
            "DISPLAY=$':0'",
        ] {
            match desktop_environment(manager, |_| None) {
                Err(RunnerError::Refused { refusal, words, .. }) => {
                    assert_eq!(refusal, "folder_dialog_unavailable");
                    assert!(!words.is_empty());
                }
                answer => panic!("expected a named desktop refusal, got {answer:?}"),
            }
        }
    }
}
