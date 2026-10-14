//! Discovery and every launch search executable files on one login PATH rule.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rustix::fs::{Access, access};

pub(crate) fn login_path() -> Result<OsString, String> {
    let shell = std::env::var_os("SHELL").ok_or("the runner login names no SHELL")?;
    read_login_path(&shell)
}

fn read_login_path(shell: &OsStr) -> Result<OsString, String> {
    if !Path::new(shell).is_absolute() {
        return Err("the runner login's SHELL is not absolute".to_owned());
    }
    let output = Command::new(shell)
        .args(["-l", "-c", "printf '%s' \"$PATH\""])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("the login PATH could not be read: {error}"))?;
    if !output.status.success() {
        return Err(format!("the login PATH command exited {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(OsString::from)
        .map_err(|error| format!("the login PATH is not text: {error}"))
}

pub(crate) fn search_path(environment: &BTreeMap<String, String>) -> Result<OsString, String> {
    environment
        .get("PATH")
        .map_or_else(login_path, |path| Ok(OsString::from(path)))
}

fn executable(path: &Path) -> Result<bool, String> {
    match path.metadata() {
        Ok(metadata) if !metadata.is_file() => Ok(false),
        Ok(_) => match access(path, Access::EXEC_OK) {
            Ok(()) => Ok(true),
            Err(error)
                if matches!(
                    std::io::Error::from(error).kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(format!("{} cannot be executed: {error}", path.display())),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("{} cannot be inspected: {error}", path.display())),
    }
}

pub(crate) fn resolve_in(program: &str, search: &OsStr) -> Result<Option<PathBuf>, String> {
    if !lys_home::harness::rendering_launch::command_or_absolute_path(program) {
        return Err(lys_home::harness::rendering_launch::PROGRAM_RULE.to_owned());
    }
    let path = Path::new(program);
    if path.is_absolute() {
        return if executable(path)? {
            Ok(Some(path.to_owned()))
        } else {
            Err(format!(
                "{program} is missing or is not a regular executable file"
            ))
        };
    }
    for directory in std::env::split_paths(search).filter(|directory| directory.is_absolute()) {
        let candidate = directory.join(program);
        if executable(&candidate)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

pub(crate) fn selected(
    program: &str,
    environment: &BTreeMap<String, String>,
) -> Result<PathBuf, String> {
    let search = if Path::new(program).is_absolute() {
        OsString::new()
    } else {
        search_path(environment)?
    };
    resolve_in(program, &search)?.ok_or_else(|| format!("{program}: no executable in PATH"))
}

#[cfg(test)]
mod tests {
    use super::read_login_path;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn each_login_path_read_uses_the_current_shell_report() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let shell = directory.path().join("shell");
        for path in ["/first/bin", "/new/bin:/first/bin"] {
            std::fs::write(&shell, format!("#!/bin/sh\nprintf '%s' '{path}'\n"))?;
            std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
            assert_eq!(
                read_login_path(shell.as_os_str())?,
                std::ffi::OsStr::new(path)
            );
        }
        Ok(())
    }
}
