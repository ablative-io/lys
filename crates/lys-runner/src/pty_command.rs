//! PTY launches use a checked directory change; portable-pty's home fallback is never used.

use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;

use crate::error::RunnerError;

/// The native env utility performs chdir immediately before exec, refusing a
/// directory removed after our preflight instead of selecting the login home.
const DIRECTORY_EXEC: &str = "/usr/bin/env";

fn refused(path: &Path, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("spawn_failed", format!("{}: {reason}", path.display()))
}

fn resolve(command: &CommandBuilder, program: &str) -> Result<PathBuf, RunnerError> {
    let path = Path::new(program);
    let search = if path.is_absolute() {
        std::ffi::OsString::new()
    } else {
        command
            .get_env("PATH")
            .ok_or_else(|| refused(path, "command requires an explicit PATH"))?
            .to_owned()
    };
    crate::program_path::resolve_in(program, &search)
        .map_err(|reason| refused(path, reason))?
        .ok_or_else(|| refused(path, "no executable in PATH"))
}

/// Keep the already prepared environment, resolve its program before spawn,
/// and delegate only the final directory change and exec to the native utility.
/// No shell parses either the directory or the argument vector.
pub(crate) fn set_directory(
    command: &mut CommandBuilder,
    program: &str,
    directory: &Path,
) -> Result<(), RunnerError> {
    let program = resolve(command, program)?;
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
