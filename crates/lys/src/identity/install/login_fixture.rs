//! The login a test build of `lys` starts every service and the runner with,
//! in place of the variables of whoever runs the tests. It exists in test
//! builds only (`cfg(test)`), so no product path can reach it: the program
//! itself always reads its own login, see [`super::login`]. The tests that
//! give `login_from` variables of their own start from [`variables`] too.
//!
//! Its folder is fixed, never the runner's `HOME` or `TMPDIR`, and its shell
//! is a script this module writes there: started as a login shell, it
//! answers [`PATH`] and reads no profile, so an answer from any other shell
//! is told apart by it.

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::super::super::error::IdentityResult;
use super::refuse;

/// The test login's folder.
pub const ROOT: &str = "/tmp/lys-test-login";

/// The `PATH` the test login's shell answers. Its first entry names the test
/// login, and no other shell answers it.
pub const PATH: &str = "/tmp/lys-test-login/bin:/usr/bin:/bin:/usr/sbin:/sbin";

/// The test login's user, as `USER` and `LOGNAME`.
pub const USER: &str = "lys-test-login";

/// The test login's shell: what a login shell does with `-l -c command`,
/// with the test login's `PATH` and no profile.
const SHELL: &str = "#!/bin/sh\n\
# The test login's shell. It reads no profile and answers the test login's PATH.\n\
PATH='/tmp/lys-test-login/bin:/usr/bin:/bin:/usr/sbin:/sbin'\n\
export PATH\n\
if [ \"$1\" = -l ]; then shift; fi\n\
exec /bin/sh \"$@\"\n";

/// The test login's shell, as `SHELL` names it.
pub fn shell() -> PathBuf {
    Path::new(ROOT).join("shell")
}

/// The test login's home, as `HOME` names it.
pub fn home() -> PathBuf {
    Path::new(ROOT).join("home")
}

/// The test login's variables, its shell written in place first. The shell
/// is written beside itself and renamed over, so a test process starting it
/// never meets it half written.
///
/// # Errors
/// `Unready` when the folder, the home or the shell cannot be written.
pub fn variables() -> IdentityResult<Vec<(OsString, OsString)>> {
    let unwritten = |what: &Path, error: std::io::Error| {
        refuse(
            "prepare test login",
            "environment",
            format!("{} could not be written: {error}", what.display()),
        )
    };
    let home = home();
    std::fs::create_dir_all(&home).map_err(|error| unwritten(&home, error))?;
    let shell = shell();
    let written = Path::new(ROOT).join(format!("shell.{}", std::process::id()));
    std::fs::write(&written, SHELL).map_err(|error| unwritten(&written, error))?;
    std::fs::set_permissions(&written, std::fs::Permissions::from_mode(0o755))
        .map_err(|error| unwritten(&written, error))?;
    std::fs::rename(&written, &shell).map_err(|error| unwritten(&shell, error))?;
    Ok(vec![
        ("HOME".into(), home.into()),
        ("USER".into(), USER.into()),
        ("LOGNAME".into(), USER.into()),
        ("SHELL".into(), shell.into()),
        ("TMPDIR".into(), "/tmp".into()),
    ])
}
