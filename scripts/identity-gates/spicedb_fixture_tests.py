"""The identity leg runs its explicit live target in an owned database."""

import json
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
            program = directory / "command"
            program.write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, pathlib, sys\n"
                "name = pathlib.Path(sys.argv[0]).name\n"
                "args = sys.argv[1:]\n"
                "row = [name, args]\n"
                "if name == 'cargo' and 'lys-identity-server' in args:\n"
                "    key = pathlib.Path(os.environ['LYS_SPICEDB_KEY_FILE'])\n"
                "    assert key.stat().st_mode & 0o777 == 0o600\n"
                "    assert key.read_text().strip()\n"
                "    row.append(os.environ['LYS_SPICEDB_ENDPOINT'])\n"
                "with open(os.environ['TRACE'], 'a') as output:\n"
                "    output.write(json.dumps(row) + '\\n')\n"
                "if name == 'docker':\n"
                "    if args[0] == 'run': print('a' * 64)\n"
                "    elif args[0] == 'logs':\n"
                "        print(json.dumps({'message': 'HTTP server started serving'}))\n"
                "    elif args[0] == 'port': print('127.0.0.1:18765')\n"
            )
            program.chmod(0o700)
            for name in ["docker", "cargo", "git"]:
                (directory / name).symlink_to(program)
            source = (ROOT / ".land/gates.sh").read_text()
            function = source.split("identity_leg() {", 1)[1].split("\n}", 1)[0]
            environment = dict(os.environ, PATH=f"{directory}:{os.environ['PATH']}", TRACE=str(trace))
            result = subprocess.run(
                ["sh", "-c", f"identity_leg() {{{function}\n}}\nidentity_leg"],
                cwd=ROOT,
                env=environment,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            rows = [json.loads(line) for line in trace.read_text().splitlines()]
            live = [row for row in rows if row[0] == "cargo" and "nextest" in row[1] and "identity_spicedb" in row[1]]
            self.assertEqual(len(live), 1, rows)
            self.assertIn("lys-identity-server", live[0][1])
            self.assertIn("--no-tests", live[0][1])
            self.assertIn("fail", live[0][1])
            self.assertEqual(live[0][2], "127.0.0.1:18765")
            owned = "a" * 64
            self.assertTrue(any(row[:2] == ["docker", ["rm", "--force", owned]] for row in rows))


if __name__ == "__main__":
    unittest.main()
