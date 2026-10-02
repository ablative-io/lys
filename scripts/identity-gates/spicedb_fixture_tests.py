"""The identity leg runs its explicit live target in an owned database."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class SpiceDbLegTests(unittest.TestCase):
    def test_identity_leg_runs_live_target_and_removes_its_container(self):
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


if __name__ == "__main__":
    unittest.main()
