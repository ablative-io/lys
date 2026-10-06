"""Preparation checks the actual socket path and never starts an installer or reserves TCP."""

import socket
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from upgrade_preflight import socket_paths
from upgrade_live import main


class PreflightTests(unittest.TestCase):
    def test_prepare_only_never_installs_or_claims_a_pass(self):
        args = ["upgrade_live.py", "--old-commit", "a" * 40, "--candidate-commit", "b" * 40,
                "--prepare-only"]
        for name in ("old-bin", "candidate-bin", "old-surface", "candidate-surface", "work"):
            args.extend(["--" + name, "/fixture/" + name])
        with patch("sys.argv", args), patch("upgrade_live.prepare", return_value={"prepared": True}), \
                patch("upgrade_live.exercise") as exercise, patch("builtins.print") as output:
            main()
        exercise.assert_not_called()
        self.assertNotIn('"passed"', output.call_args.args[0])

    def test_checks_both_paths_and_removes_only_its_probe_directories(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sentinel = root / "kept"
            sentinel.write_text("unrelated")
            with patch("upgrade_preflight.socket.socket") as factory:
                factory.return_value.__enter__.return_value.bind.side_effect = lambda path: Path(path).touch()
                paths = socket_paths(root / "fixture")
            self.assertEqual(len(paths), 2)
            self.assertEqual(factory.call_args_list[0].args, (socket.AF_UNIX, socket.SOCK_STREAM))
            self.assertEqual([call.args[0] for call in factory.return_value.__enter__.return_value.bind.call_args_list], paths)
            self.assertEqual(list(root.iterdir()), [sentinel])

    def test_kernel_refusal_names_exact_path_and_leaves_no_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "fixture"
            with patch("upgrade_preflight.socket.socket") as factory:
                factory.return_value.__enter__.return_value.bind.side_effect = OSError("path too long")
                with self.assertRaisesRegex(RuntimeError, str(root / "positive/install/run/runner.sock")):
                    socket_paths(root)
            self.assertFalse(root.exists())

    def test_existing_socket_path_is_never_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "positive/install/run/runner.sock"
            path.parent.mkdir(parents=True)
            path.write_text("belongs to another run")
            with self.assertRaisesRegex(RuntimeError, "already exists"):
                socket_paths(root)
            self.assertEqual(path.read_text(), "belongs to another run")


if __name__ == "__main__":
    unittest.main()
