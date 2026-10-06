"""The login the upgrade proof starts every real program with, named in full.

The installed programs and the upgrade_window driver read their login's
variables and start its SHELL as a login shell, as they do for a person. The
proof never passes on its own environment: HOME, USER and LOGNAME are the
account's own, read from the account database (the container runtime's
command-line plugins live under that home); SHELL is a script written here
that reads no profile and answers PATH, the folder whose open and xdg-open
find no browser and then the folders named in FOLDERS.
"""

import os
import pwd

# Where the container runtime is installed on a Mac, by its installer or by
# Homebrew, and the system's own folders.
FOLDERS = ("/usr/local/bin", "/opt/homebrew/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin")


def login(at):
    """Write the login in `at` and answer its variables, PATH among them."""
    account = pwd.getpwuid(os.getuid())
    headless = at / "headless"
    headless.mkdir()
    for name in ["open", "xdg-open"]:
        executable = headless / name
        executable.write_text("#!/bin/sh\nexit 1\n")
        executable.chmod(0o700)
    path = os.pathsep.join((str(headless),) + FOLDERS)
    if "'" in path:
        raise RuntimeError(f"the proof's login PATH cannot be quoted for its shell: {path}")
    shell = at / "login-shell"
    shell.write_text(
        "#!/bin/sh\n"
        "# The upgrade proof's login shell: no profile, the proof's PATH.\n"
        f"PATH='{path}'\n"
        "export PATH\n"
        'if [ "$1" = -l ]; then shift; fi\n'
        'exec /bin/sh "$@"\n'
    )
    shell.chmod(0o700)
    return {
        "HOME": account.pw_dir,
        "USER": account.pw_name,
        "LOGNAME": account.pw_name,
        "SHELL": str(shell),
        "TMPDIR": "/tmp",
        "PATH": path,
        "PYTHONDONTWRITEBYTECODE": "1",
    }
