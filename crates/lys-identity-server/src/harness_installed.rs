//! Discover one executable without changing the process search path.

use std::ffi::OsStr;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::process::Command;

use super::{BuildSource, BuildView};

fn refused(name: &str, reason: impl std::fmt::Display) -> String {
    format!("{name}: {reason}")
}

pub(super) type CopyKey = (PathBuf, u64, u64, i64, i64, u64);

pub(super) struct Executable {
    pub(super) key: CopyKey,
}

pub(super) fn resolve(command: &str, path: &OsStr) -> Result<Executable, String> {
    if !matches!(command, "claude" | "codex") {
        return Err(refused(
            "InvalidProgramCommand",
            "the command is not a standard program",
        ));
    }
    let directories: Vec<PathBuf> = std::env::split_paths(path).collect();
    if path.is_empty()
        || directories.is_empty()
        || directories.iter().any(|entry| !entry.is_absolute())
    {
        return Err(refused(
            "UnsafeSearchPath",
            "every search directory must be absolute and nonempty",
        ));
    }
    let mut executable = None;
    for directory in directories {
        let candidate = directory.join(command);
        let metadata = match std::fs::metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(refused("ProgramMetadataUnreadable", error)),
        };
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            continue;
        }
        let absolute = std::fs::canonicalize(&candidate)
            .map_err(|error| refused("ProgramPathUnreadable", error))?;
        executable = Some(Executable {
            key: (
                absolute,
                metadata.dev(),
                metadata.ino(),
                metadata.mtime(),
                metadata.mtime_nsec(),
                metadata.len(),
            ),
        });
        break;
    }
    executable.ok_or_else(|| {
        refused(
            "CommandNotFound",
            format!("{command} is not installed on the search path"),
        )
    })
}

pub(super) fn version(
    command: &str,
    executable: &Executable,
    path: &OsStr,
) -> Result<BuildView, String> {
    let executable = &executable.key.0;
    let output = Command::new(executable)
        .arg("--version")
        .env("PATH", path)
        .output()
        .map_err(|error| refused("VersionCommandUnreadable", error))?;
    if !output.status.success() {
        return Err(refused("VersionCommandFailed", output.status));
    }
    let version = String::from_utf8(output.stdout)
        .map_err(|error| refused("VersionOutputUnreadable", error))?;
    let version = version.trim();
    if version.is_empty() || version.len() > 256 || version.chars().any(char::is_control) {
        return Err(refused(
            "VersionOutputUnreadable",
            "the version is not one short nonempty line",
        ));
    }
    let program = executable
        .to_str()
        .ok_or_else(|| refused("ProgramPathUnreadable", "the executable path is not UTF-8"))?;
    Ok(BuildView {
        name: format!("Installed {command}"),
        program: program.to_owned(),
        package: version.to_owned(),
        source: BuildSource::Installed,
    })
}
