"""The proof's login is named in full and its shell answers the proof's PATH."""

import os
import pwd
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from upgrade_login import FOLDERS, login


class LoginTests(unittest.TestCase):
    def test_the_login_is_named_and_nothing_of_the_proof_s_environment_reaches_it(self):
        with tempfile.TemporaryDirectory() as directory, \
                patch.dict(os.environ, {"SHELL": "/the/runner/s/shell", "HOME": "/the/runner/s/home",
                                        "LYS_PROOF_RUNNER_OWN": "leaked"}):
            env = login(Path(directory))
            account = pwd.getpwuid(os.getuid())
            self.assertEqual(env["HOME"], account.pw_dir)
            self.assertEqual(env["USER"], account.pw_name)
            self.assertEqual(env["LOGNAME"], account.pw_name)
            self.assertEqual(env["SHELL"], str(Path(directory) / "login-shell"))
            self.assertEqual(env["TMPDIR"], "/tmp")
            self.assertNotIn("LYS_PROOF_RUNNER_OWN", env)
            headless = Path(directory) / "headless"
            self.assertEqual(env["PATH"].split(os.pathsep), [str(headless), *FOLDERS])
            for name in ("open", "xdg-open"):
                refused = subprocess.run([str(headless / name)], env=env)
                self.assertEqual(refused.returncode, 1)

    def test_the_login_shell_started_as_a_login_shell_answers_the_proof_s_path(self):
        with tempfile.TemporaryDirectory() as directory:
            env = login(Path(directory))
            answered = subprocess.run(
                [env["SHELL"], "-l", "-c", 'printf %s "$PATH"'],
                env={"HOME": env["HOME"], "SHELL": env["SHELL"]},
                capture_output=True, text=True, check=True,
            )
            self.assertEqual(answered.stdout, env["PATH"])


if __name__ == "__main__":
    unittest.main()
