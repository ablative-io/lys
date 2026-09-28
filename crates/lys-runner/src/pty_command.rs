//! PTY launches use a checked directory change; portable-pty's home fallback is never used.

use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;
use rustix::fs::{Access, access};

use crate::error::RunnerError;

/// The native env utility performs chdir immediately before exec, refusing a
/// directory removed after our preflight instead of selecting the login home.
const DIRECTORY_EXEC: &str = "/usr/bin/env";

fn refused(path: &Path, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("spawn_failed", format!("{}: {reason}", path.display()))
}

fn executable(path: &Path) -> Result<(), RunnerError> {
    let metadata = path.metadata().map_err(|error| refused(path, error))?;
    if !metadata.is_file() {
        return Err(refused(path, "program is not a regular executable file"));
    }
    access(path, Access::EXEC_OK).map_err(|error| refused(path, error))
}

fn resolve(command: &CommandBuilder, program: &str, cwd: &Path) -> Result<PathBuf, RunnerError> {
    let path = Path::new(program);
    if program.is_empty() || program.contains(['=', '\0']) {
        return Err(refused(path, "program is empty or contains '=' or NUL"));
    }
    if path.is_absolute() || program.contains('/') {
        let resolved = cwd.join(path);
        executable(&resolved)?;
        return Ok(resolved);
    }
    let search = command
        .get_env("PATH")
        .ok_or_else(|| refused(path, "relative program requires an explicit PATH"))?;
    let mut failures = Vec::new();
    for directory in std::env::split_paths(search) {
        let candidate = cwd.join(directory).join(path);
        match executable(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) => failures.push(error.to_string()),
        }
    }
    Err(refused(
        path,
        format!("no executable in PATH: {}", failures.join("; ")),
    ))
}

/// Keep the already prepared environment, resolve its program before spawn,
/// and delegate only the final directory change and exec to the native utility.
/// No shell parses either the directory or the argument vector.
pub(crate) fn set_directory(
    command: &mut CommandBuilder,
    program: &str,
    directory: &Path,
) -> Result<(), RunnerError> {
    let program = resolve(command, program, directory)?;
    if program.as_os_str().as_encoded_bytes().contains(&b'=') {
        return Err(refused(&program, "resolved program contains '='"));
    }
    let arguments = command.get_argv_mut();
    let original_arguments: Vec<_> = arguments.drain(1..).collect();
    arguments.clear();
    arguments.extend([
        DIRECTORY_EXEC.into(),
        "-C".into(),
        directory.as_os_str().to_owned(),
        "--".into(),
        program.into_os_string(),
    ]);
    arguments.extend(original_arguments);
    // This existing root is only the bootstrap cwd. env must successfully
    // chdir to the requested path before the requested program is executed.
    command.cwd("/");
    Ok(())
}
