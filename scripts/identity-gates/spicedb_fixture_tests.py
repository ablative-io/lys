"""The identity leg runs its explicit live target in an owned database."""

import os
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

import spicedb_fixture


ROOT = Path(__file__).resolve().parents[2]


class SpiceDbLegTests(unittest.TestCase):
    def test_identity_leg_names_live_target_with_owned_fixture(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            trace = directory / "trace"
            source = (ROOT / ".land/gates.sh").read_text()
            function = source.split("identity_leg() {", 1)[1].split("\n}", 1)[0]
            environment = dict(os.environ, TRACE=str(trace))
            stubs = """
docker() { printf 'docker %s\n' "$*" >> "$TRACE"; }
git() { printf 'git %s\n' "$*" >> "$TRACE"; }
cargo() { printf 'cargo %s\n' "$*" >> "$TRACE"; }
python3() { printf 'python3 %s\n' "$*" >> "$TRACE"; }
"""
            result = subprocess.run(
                ["sh", "-c", f"{stubs}\nidentity_leg() {{{function}\n}}\nidentity_leg"],
                cwd=ROOT,
                env=environment,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            rows = trace.read_text().splitlines()
            live = [row for row in rows if "nextest" in row and "identity_spicedb" in row]
            self.assertEqual(len(live), 1, rows)
            self.assertIn("lys-identity-server", live[0])
            self.assertIn("--no-tests fail", live[0])
            self.assertIn("spicedb_fixture.py", live[0])

    def test_command_failure_keeps_its_exit_and_removes_the_owned_container(self):
        owned = "a" * 64
        key_files = []

        def command(arguments, env):
            key = Path(env["LYS_SPICEDB_KEY_FILE"])
            self.assertEqual(key.stat().st_mode & 0o777, 0o600)
            self.assertTrue(key.read_text().strip())
            self.assertEqual(env["LYS_SPICEDB_ENDPOINT"], "127.0.0.1:18765")
            key_files.append(key)
            return Mock(returncode=17)

        with patch.object(spicedb_fixture, "docker", side_effect=["", owned, "127.0.0.1:18765", ""]) as docker, patch.object(spicedb_fixture, "ready"), patch.object(spicedb_fixture.subprocess, "run", side_effect=command), patch("sys.stdout", new_callable=io.StringIO):
            self.assertEqual(spicedb_fixture.run(["cargo", "nextest"]), 17)
        docker.assert_any_call("rm", "--force", owned)
        self.assertTrue(key_files)
        self.assertFalse(key_files[0].exists())

    def test_early_database_exit_refuses_and_removes_its_container(self):
        owned = "a" * 64
        with patch.object(spicedb_fixture, "docker", side_effect=["", owned, ""]) as docker, patch.object(spicedb_fixture, "ready", side_effect=RuntimeError("spicedb_not_ready")), patch.object(spicedb_fixture.subprocess, "run") as command, patch("sys.stderr", new_callable=io.StringIO) as error:
            self.assertEqual(spicedb_fixture.main(["cargo", "nextest"]), 1)
            self.assertIn("spicedb_not_ready", error.getvalue())
        command.assert_not_called()
        docker.assert_any_call("rm", "--force", owned)

    def test_readiness_uses_the_http_event_and_reaps_only_its_logger(self):
        logger = Mock()
        logger.stdout = io.StringIO(json.dumps({"message": "http server started serving", "service": "http"}) + "\n")
        logger.poll.return_value = None
        with patch.object(spicedb_fixture.subprocess, "Popen", return_value=logger):
            spicedb_fixture.ready("a" * 64)
        logger.terminate.assert_called_once_with()
        logger.wait.assert_called_once_with()
        self.assertTrue(logger.stdout.closed)


if __name__ == "__main__":
    unittest.main()
