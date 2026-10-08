"""Fixture evidence cannot authorise a qualification pin."""

import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parent
ROOT = SCRIPTS.parents[1]
FIXTURES = ROOT / "crates/lys-runner/tests/fixtures/qualifier"
ADAPTERS = ("claude-code", "codex")
EXAMPLE = r"""import json
from pathlib import Path
import sys

arguments = sys.argv[1:]
fields = dict(zip(arguments[::2], arguments[1::2]))
if len(arguments) != 6 or set(fields) != {"--adapter", "--directory", "--evidence"}:
    raise RuntimeError("fixture_arguments_invalid")
adapter = fields["--adapter"]
root = Path(__file__).resolve().parent
controls = json.loads((root / "controls.json").read_text())
if adapter not in controls["fixtures"]:
    raise RuntimeError("fixture_adapter_invalid")
empty = not any(Path(fields["--directory"]).iterdir())
if not empty:
    raise RuntimeError("fixture_directory_not_empty")
with (root / "calls.jsonl").open("a") as calls:
    calls.write(json.dumps({"adapter": adapter, "directory_empty": empty}) + "\n")
records = controls["records"]
records[0]["fixture"] = controls["fixtures"][adapter]
with Path(fields["--evidence"]).open("x") as evidence:
    for record in records:
        evidence.write(json.dumps(record) + "\n")
"""


class QualificationPinTests(unittest.TestCase):
    def stage_a(self, fixtures):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            tools = root / "tools"
            tools.mkdir()
            disk = tools / "df"
            disk.write_text(
                "#!/bin/sh\n"
                "printf 'Filesystem Blocks Used Available Capacity Mounted\\n"
                "fixture 100 0 100 0%% /\\n'\n"
            )
            disk.chmod(0o700)
            records = [json.loads(line) for line in
                       (FIXTURES / "fixture-pass.jsonl").read_text().splitlines()]
            (root / "controls.json").write_text(json.dumps({
                "fixtures": fixtures, "records": records,
            }))
            helper = root / "example.py"
            helper.write_text(EXAMPLE)
            example = root / "example"
            example.write_text(
                "#!/bin/sh\nexec " + shlex.quote(sys.executable) + " -B "
                + shlex.quote(str(helper)) + ' "$@"\n'
            )
            example.chmod(0o700)
            output = root / "output"
            output.mkdir()
            (output / "versions-stage-a.txt").write_text(
                "claude-code 0.1.2\ncodex 0.1.2\n"
            )
            environment = {
                "PATH": f"{tools}:/usr/bin:/bin",
                "LC_ALL": "C",
                "PYTHONDONTWRITEBYTECODE": "1",
            }
            completed = subprocess.run(
                ["/bin/bash", str(SCRIPTS / "qual.sh"), "a", "--out", str(output),
                 "--example", str(example)],
                cwd=root, env=environment, capture_output=True, text=True, check=False,
            )
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            stage = output / "stage-a"
            result = {
                "stdout": completed.stdout,
                "stderr": completed.stderr,
                "pins": (stage / "pins.txt").read_bytes(),
                "calls": [json.loads(line) for line in
                          (root / "calls.jsonl").read_text().splitlines()],
                "evidence": {adapter: (stage / f"evidence-{adapter}.jsonl").read_bytes()
                             for adapter in ADAPTERS},
            }
        self.assertEqual(result["calls"], [
            {"adapter": adapter, "directory_empty": True} for adapter in ADAPTERS
        ])
        return result

    def end(self, result):
        lines = [line for line in result["stdout"].splitlines() if line.startswith("END ")]
        self.assertEqual(len(lines), 1, result["stdout"] + result["stderr"])
        return lines[0]

    def assert_pins(self, result, adapters):
        pins = [line.split() for line in result["pins"].decode().splitlines()]
        self.assertEqual(len(pins), len(adapters))
        for pin, adapter in zip(pins, adapters):
            self.assertEqual(len(pin), 4)
            self.assertEqual(pin[:3], [
                adapter, "0.1.2", hashlib.sha256(result["evidence"][adapter]).hexdigest(),
            ])
            self.assertEqual(Path(pin[3]).name, f"evidence-{adapter}.jsonl")

    def test_fixture_evidence_is_refused_by_name_and_leaves_no_pins(self):
        result = self.stage_a(dict.fromkeys(ADAPTERS, True))
        self.assertEqual(result["pins"], b"")
        self.assertIn("result=FAIL proved=0", self.end(result))
        for adapter in ADAPTERS:
            self.assertIn(f"{adapter}:fixture-evidence-is-never-a-pin", self.end(result))
            self.assertTrue(json.loads(result["evidence"][adapter].splitlines()[0])["fixture"])

    def test_real_marked_control_pins_both_exact_evidence_digests(self):
        result = self.stage_a(dict.fromkeys(ADAPTERS, False))
        self.assertIn("result=PASS proved=2 failed=[]", self.end(result))
        self.assert_pins(result, ADAPTERS)
        for adapter in ADAPTERS:
            self.assertFalse(json.loads(result["evidence"][adapter].splitlines()[0])["fixture"])

    def test_a_fixture_partner_cannot_supply_a_second_pin(self):
        result = self.stage_a({"claude-code": False, "codex": True})
        self.assertIn("result=FAIL proved=1", self.end(result))
        self.assertIn("codex:fixture-evidence-is-never-a-pin", self.end(result))
        self.assert_pins(result, ("claude-code",))

    def test_stored_fixture_pass_and_failure_keep_their_named_verdicts(self):
        for name, code, prefix in [
            ("fixture-pass.jsonl", 0, "FIXTURE codex 0.1.2 "),
            ("fail-stop-red.jsonl", 1,
             "FAIL codex version_report not-observed:qualification_cancelled: "
             "the qualification was stopped;cleanup:ok"),
        ]:
            with self.subTest(fixture=name):
                completed = subprocess.run(
                    [sys.executable, "-B", str(SCRIPTS / "judge.py"), "stage-a",
                     str(FIXTURES / name), "codex"],
                    cwd=ROOT, env={"PATH": os.defpath, "LC_ALL": "C",
                                   "PYTHONDONTWRITEBYTECODE": "1"},
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(completed.returncode, code, completed.stdout + completed.stderr)
                self.assertTrue(completed.stdout.startswith(prefix), completed.stdout)
                self.assertEqual(len(completed.stdout.splitlines()), 1)
                if code == 0:
                    self.assertEqual(completed.stdout.strip(), prefix + hashlib.sha256(
                        (FIXTURES / name).read_bytes()).hexdigest())


if __name__ == "__main__":
    unittest.main()
