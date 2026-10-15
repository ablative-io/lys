//! Account identity and paths used by native login services.

use super::{Platform, refused};
use crate::commands::error::CliResult;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(super) fn user(argument: &str) -> CliResult<String> {
    let output = Command::new("id")
        .arg(argument)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            refused(
                "runner_service_user_unknown",
                format!("id could not run: {error}"),
            )
        })?;
    let value = String::from_utf8(output.stdout).map_err(|error| {
        refused(
            "runner_service_user_unknown",
            format!("id did not return a readable user: {}", error.utf8_error()),
        )
    })?;
    let value = value.trim();
    if !output.status.success()
        || value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(refused(
            "runner_service_user_unknown",
            "id did not return a valid user",
        ));
    }
    Ok(value.to_owned())
}

pub(super) fn path_text(path: &Path) -> CliResult<&str> {
    path.to_str()
        .filter(|text| !text.chars().any(char::is_control))
        .ok_or_else(|| {
            refused(
                "runner_service_path_invalid",
                "login service paths must be readable text without control characters",
            )
        })
}

pub(super) fn account_shell(platform: Platform, person: &str) -> CliResult<PathBuf> {
    let mut command = match platform {
        Platform::Mac => {
            let mut command = Command::new("/usr/bin/dscl");
            command.args([".", "-read", &format!("/Users/{person}"), "UserShell"]);
            command
        }
        Platform::Linux => {
            let mut command = Command::new("/usr/bin/getent");
            command.args(["passwd", person]);
            command
        }
    };
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            refused(
                "runner_service_shell_unknown",
                format!(
                    "this person's login shell could not be read from the account record: {error}"
                ),
            )
        })?;
    if !output.status.success() {
        return Err(refused(
            "runner_service_shell_unknown",
            format!(
                "the account service did not answer this person's login shell ({})",
                output.status
            ),
        ));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|error| {
        refused(
            "runner_service_shell_unknown",
            format!("the account record does not contain a readable login shell: {error}"),
        )
    })?;
    shell_from_record(platform, person, text)
}

pub(super) fn shell_from_record(
    platform: Platform,
    person: &str,
    record: &str,
) -> CliResult<PathBuf> {
    let shell = match platform {
        Platform::Mac => record.trim().strip_prefix("UserShell:").map(str::trim),
        Platform::Linux => {
            let fields: Vec<_> = record.trim_end_matches(['\r', '\n']).split(':').collect();
            if fields.len() == 7 && fields[0] == person {
                Some(fields[6])
            } else {
                None
            }
        }
    }
    .filter(|shell| Path::new(shell).is_absolute() && !shell.chars().any(char::is_control));
    shell.map(PathBuf::from).ok_or_else(|| refused("runner_service_shell_unknown", "the account record names no absolute login shell; set this person's login shell before installing the runner"))
}
