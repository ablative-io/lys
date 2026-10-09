"""Isolation refuses feature-unified builds and retains every independent outcome."""

import errno
import io
import json
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

import release_isolation


COMMIT = "a" * 40
TREE = "b" * 40
PACKAGES = ("lys", "lys-identity-server", "lys-secrets", "lys-runner")


class IsolationTests(unittest.TestCase):
    def exercise(self, exits=None, identities=None, unavailable=None):
        git_values = iter(identities or (COMMIT, TREE, "", COMMIT, TREE, ""))
        outcomes = exits or {}
        calls = []

        def run(command, **options):
            self.assertEqual(options["cwd"], Path("/build/lys"))
            self.assertFalse(options["check"])
            if command[0] == "git":
                return subprocess.CompletedProcess(command, 0, next(git_values), "")
            calls.append(command)
            if command[-1] == unavailable:
                raise FileNotFoundError(errno.ENOENT, "compiler unavailable")
            return subprocess.CompletedProcess(command, outcomes.get(command[-1], 0))

        with patch("release_isolation.subprocess.run", side_effect=run):
            result = release_isolation.prove(Path("/build/lys"))
        return result, calls

    def test_every_package_builds_in_a_separate_plain_release_process(self):
        result, commands = self.exercise()
        self.assertEqual(commands, [
            ["cargo", "build", "--locked", "--release", "-p", package]
            for package in PACKAGES
        ])
        self.assertTrue(result["passed"])
        self.assertEqual([row["package"] for row in result["builds"]], list(PACKAGES))

    def test_success_retains_exact_identity_full_counts_and_actual_exits(self):
        result, commands = self.exercise()
        self.assertEqual(len(commands), 4)
        self.assertEqual((result["commit"], result["tree"]), (COMMIT, TREE))
        self.assertEqual(result["counts"], {
            "expected": 4, "executed": 4, "passed": 4, "failed": 0, "not_started": 0,
        })
        self.assertEqual([row["exit_code"] for row in result["builds"]], [0, 0, 0, 0])
        self.assertTrue(all(row["elapsed_seconds"] >= 0 for row in result["builds"]))

    def test_server_refusal_names_it_and_does_not_hide_the_other_builds(self):
        result, commands = self.exercise(exits={"lys-identity-server": 101})
        self.assertEqual(len(commands), 4)
        self.assertFalse(result["passed"])
        self.assertEqual(result["counts"]["failed"], 1)
        self.assertEqual(result["counts"]["passed"], 3)
        self.assertEqual(result["builds"][1]["refusal"],
                         "release_isolation_build_failed: lys-identity-server exited 101")

    def test_every_failed_exit_is_retained_including_a_signal(self):
        result, commands = self.exercise(exits={"lys": 101, "lys-runner": -9})
        self.assertEqual(len(commands), 4)
        self.assertFalse(result["passed"])
        self.assertEqual([row["exit_code"] for row in result["builds"]], [101, 0, 0, -9])
        self.assertEqual(result["counts"]["failed"], 2)
        self.assertIn("lys-runner exited -9", result["builds"][3]["refusal"])

    def test_a_missing_compiler_is_unstarted_rather_than_a_successful_exit(self):
        result, commands = self.exercise(unavailable="lys-identity-server")
        self.assertEqual(len(commands), 4)
        self.assertFalse(result["passed"])
        self.assertIsNone(result["builds"][1]["exit_code"])
        self.assertIn("release_isolation_build_unavailable: lys-identity-server:",
                      result["builds"][1]["refusal"])
        self.assertEqual(result["counts"], {
            "expected": 4, "executed": 3, "passed": 3, "failed": 0, "not_started": 1,
        })

    def test_dirty_source_refuses_before_starting_cargo(self):
        result, commands = self.exercise(identities=(COMMIT, TREE, " M Cargo.toml"))
        self.assertEqual(commands, [])
        self.assertFalse(result["passed"])
        self.assertIn("release_isolation_dirty_checkout:", result["source_refusal"])
        self.assertEqual(result["counts"]["not_started"], 4)

    def test_invalid_identity_refuses_before_starting_cargo(self):
        result, commands = self.exercise(identities=("not a commit", TREE, ""))
        self.assertEqual(commands, [])
        self.assertIn("release_isolation_invalid_identity:", result["source_refusal"])
        self.assertFalse(result["passed"])

    def test_changed_head_refuses_even_when_every_build_exits_zero(self):
        changed = "c" * 40
        result, commands = self.exercise(identities=(COMMIT, TREE, "", changed, TREE, ""))
        self.assertEqual(len(commands), 4)
        self.assertEqual(result["counts"]["passed"], 4)
        self.assertFalse(result["passed"])
        self.assertIn(f"became {changed}/{TREE}", result["source_refusal"])

    def test_changed_tree_or_lockfile_refuses_a_success_claim(self):
        for final in ((COMMIT, "c" * 40, ""), (COMMIT, TREE, " M Cargo.lock")):
            with self.subTest(final=final):
                result, commands = self.exercise(identities=(COMMIT, TREE, "", *final))
                self.assertEqual(len(commands), 4)
                self.assertFalse(result["passed"])
                self.assertIsNotNone(result["source_refusal"])

    def test_git_failure_and_missing_git_are_named_without_starting_cargo(self):
        outcomes = (
            subprocess.CompletedProcess(["git"], 128, "", "repository unavailable"),
            FileNotFoundError(errno.ENOENT, "git unavailable"),
        )
        for outcome in outcomes:
            with self.subTest(outcome=outcome):
                options = ({"side_effect": outcome} if isinstance(outcome, OSError)
                           else {"return_value": outcome})
                with patch("release_isolation.subprocess.run", **options) as run:
                    result = release_isolation.prove(Path("/build/lys"))
                self.assertEqual(run.call_count, 1)
                self.assertFalse(result["passed"])
                self.assertEqual(result["builds"], [])
                self.assertIn("release_isolation_git_", result["source_refusal"])

    def test_combined_package_selection_is_the_negative_control(self):
        combined = [["cargo", "build", "--locked", "--release",
                     "-p", "lys", "-p", "lys-identity-server", "-p", "lys-secrets",
                     "-p", "lys-runner"]]
        with patch("release_isolation.build_commands", return_value=combined):
            result, commands = self.exercise()
        self.assertEqual(len(commands), 1)
        self.assertFalse(result["passed"])
        self.assertNotEqual(commands, [
            ["cargo", "build", "--locked", "--release", "-p", package]
            for package in PACKAGES
        ])


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.evidence = tempfile.TemporaryDirectory()
        self.addCleanup(self.evidence.cleanup)
        self.report = Path(self.evidence.name) / "proof.json"

    def invoke(self, result):
        output, errors = io.StringIO(), io.StringIO()
        with patch("release_isolation.prove", return_value=result) as prove, \
                redirect_stdout(output), redirect_stderr(errors):
            code = release_isolation.main(["--report", str(self.report)])
        return code, output.getvalue(), errors.getvalue(), prove

    def test_report_persists_the_measured_outcome_and_cli_exit(self):
        for passed in (True, False):
            with self.subTest(passed=passed):
                result = {"commit": COMMIT, "tree": TREE, "builds": [], "counts": {},
                          "passed": passed, "source_refusal": None}
                code, output, errors, prove = self.invoke(result)
                self.assertEqual(code, 0 if passed else 1)
                self.assertEqual(json.loads(self.report.read_text()), result)
                self.assertEqual(json.loads(output)["passed"], passed)
                self.assertEqual(errors, "")
                prove.assert_called_once()
                self.report.unlink()

    def test_existing_report_is_preserved_and_never_starts_a_build(self):
        self.report.write_text("retained evidence")
        code, output, errors, prove = self.invoke({})
        self.assertEqual(code, 1)
        self.assertEqual(output, "")
        self.assertIn("release_isolation_report_unavailable:", errors)
        self.assertEqual(self.report.read_text(), "retained evidence")
        prove.assert_not_called()

    def test_report_cannot_dirty_the_checkout_it_measures(self):
        self.report = Path(release_isolation.__file__).resolve().parents[2] / "proof.json"
        code, output, errors, prove = self.invoke({})
        self.assertEqual(code, 1)
        self.assertEqual(output, "")
        self.assertIn("release_isolation_report_inside_checkout:", errors)
        prove.assert_not_called()

    def test_missing_report_directory_refuses_before_any_build(self):
        self.report = self.report.parent / "absent" / "proof.json"
        code, output, errors, prove = self.invoke({})
        self.assertEqual(code, 1)
        self.assertEqual(output, "")
        self.assertIn("release_isolation_report_unavailable:", errors)
        prove.assert_not_called()

    def test_named_build_and_source_refusals_are_visible(self):
        result = {"commit": COMMIT, "tree": TREE, "counts": {}, "passed": False,
                  "builds": [{"refusal": "release_isolation_build_failed: lys exited 101"}],
                  "source_refusal": "release_isolation_dirty_checkout: M Cargo.lock"}
        code, output, errors, prove = self.invoke(result)
        self.assertEqual(code, 1)
        self.assertEqual(json.loads(self.report.read_text()), result)
        self.assertIn(result["builds"][0]["refusal"], errors)
        self.assertIn(result["source_refusal"], errors)
        self.assertFalse(json.loads(output)["passed"])
        prove.assert_called_once()


if __name__ == "__main__":
    unittest.main()
