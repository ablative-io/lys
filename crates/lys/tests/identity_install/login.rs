//! The login this test starts the real `lys` program with, named here in
//! full and never passed on from the process running the test: the
//! program reads its login's variables and starts its `SHELL` as a login
//! shell, as it does for a person (`identity::install::login`).
//!
//! `HOME`, `USER` and `LOGNAME` are the account's own, read from the
//! account database rather than from this process; the container runtime's
//! command-line plugins live under that home. `SHELL` is a script written
//! here: started as a login shell it reads no profile and answers
//! [`PATH`] behind the folder whose `open` and `xdg-open` find no browser,
//! so the install hands its setup code over as a headless one does.

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use nix::unistd::{Uid, User};

use super::identity_support::fixtures::TestResult;

/// The folders the login's shell answers after the one with no browser:
/// where the container runtime is installed on a Mac, by its installer or
/// by Homebrew, and the system's own.
pub const PATH: &str = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin";

/// The login written in `at`: its variables, `PATH` among them.
pub fn login(at: &Path) -> TestResult<Vec<(OsString, OsString)>> {
    let account = User::from_uid(Uid::current())?
        .ok_or("login_account_missing: the account database has no entry for this user")?;
    let bin = at.join("no-browser");
    std::fs::create_dir(&bin)?;
    for name in ["open", "xdg-open"] {
        let script = bin.join(name);
        std::fs::write(&script, "#!/bin/sh\nexit 1\n")?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    }
    let path = format!("{}:{PATH}", bin.display());
    let shell = at.join("login-shell");
    std::fs::write(
        &shell,
        format!(
            "#!/bin/sh\n# This test's login shell: no profile, this test's PATH.\nPATH='{path}'\nexport PATH\nif [ \"$1\" = -l ]; then shift; fi\nexec /bin/sh \"$@\"\n"
        ),
    )?;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755))?;
    Ok(vec![
        ("HOME".into(), account.dir.into()),
        ("USER".into(), account.name.clone().into()),
        ("LOGNAME".into(), account.name.into()),
        ("SHELL".into(), shell.into()),
        ("TMPDIR".into(), "/tmp".into()),
        ("PATH".into(), path.into()),
    ])
}
