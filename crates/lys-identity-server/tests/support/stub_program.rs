//! Launching tests own the executable they ask the runner to start.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Ignore declaration flags while honouring an explicit account-resume command.
pub const SHELL: &str = "if [ \"${1-}\" = '-c' ]; then\n  exec /bin/sh \"$@\"\nfi\nexec /bin/sh";

/// Keep an executable in the caller's temporary folder without shell startup files.
///
/// # Errors
/// Returns the filesystem error if the script or its permissions cannot be kept.
pub fn write(folder: &Path, body: &str) -> std::io::Result<PathBuf> {
    let program = folder.join("stub-program");
    std::fs::write(&program, format!("#!/bin/sh\nunset ENV\n{body}\n"))?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
    Ok(program)
}
