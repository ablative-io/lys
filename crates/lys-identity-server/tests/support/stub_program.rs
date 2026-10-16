//! Launching tests own the executable they ask the runner to start.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Ignore declaration flags while honouring an explicit account-resume command.
///
/// The runner types into a harness's screen as a bracketed paste, as Claude
/// Code and Codex expect. This stand-in takes the paste markers off as their
/// screens do and runs each line, the same on every computer: Debian's
/// `/bin/sh` (dash) has no line editor to drop them, so an interactive shell
/// there would run the markers as part of the command. `command eval` keeps
/// a line that does not parse from ending the stand-in, as an interactive
/// shell would; a typed `exit` still ends it. A Ctrl-L clears a screen and
/// is dropped, never run as part of the next line.
pub const SHELL: &str = concat!(
    "if [ \"${1-}\" = '-c' ]; then\n  exec /bin/sh \"$@\"\nfi\n",
    "esc=$(printf '\\033')\n",
    "while IFS= read -r line; do\n",
    "  line=$(printf '%s' \"$line\" | tr -d '\\014')\n",
    "  line=${line#\"$esc[200~\"}\n",
    "  line=${line%\"$esc[201~\"}\n",
    "  command eval \"$line\"\n",
    "done",
);

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
