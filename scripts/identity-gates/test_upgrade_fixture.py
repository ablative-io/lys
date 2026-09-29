"""The upgrade proof rejects missing data and rewritten historical app leaves."""

import copy
import tempfile
import unittest
from pathlib import Path

from upgrade_fixture import same_records
from upgrade_live import app_leaves, preserve_leaves


class UpgradeProofTests(unittest.TestCase):
    def test_every_domain_difference_is_refused(self):
        before = {name: {"value": name} for name in (
            "people", "grant", "app", "configuration", "budgets", "goals", "policy")}
        before["sessions"] = {"sessions": [{"id": "preserved"}]}
        same_records(before, copy.deepcopy(before))
        for name in before:
            with self.subTest(domain=name):
                after = copy.deepcopy(before)
                after[name] = {}
                with self.assertRaisesRegex(RuntimeError, name):
                    same_records(before, after)

    def test_empty_session_proof_is_refused(self):
        empty = {"sessions": {"sessions": []}}
        with self.assertRaisesRegex(RuntimeError, "no session"):
            same_records(empty, empty)

    def test_missing_domain_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, "domains differ"):
            same_records({"people": []}, {})

    def test_old_app_leaves_must_stay_byte_exact_but_new_leaves_are_allowed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            leaves = root / "data/apps/leaves"
            leaves.mkdir(parents=True)
            for index in range(3):
                (leaves / f"{index:020}").write_bytes(f"old event {index}".encode())
            before = app_leaves(root, {"grant_log_dir": str(root / "data/grants")})
            (leaves / f"{3:020}").write_bytes(b"candidate event")
            preserve_leaves(root, before)
            (leaves / f"{1:020}").write_bytes(b"rewritten history")
            with self.assertRaisesRegex(RuntimeError, "old app leaf"):
                preserve_leaves(root, before)

    def test_app_log_outside_fixture_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(RuntimeError, "leaves its root"):
                app_leaves(root, {"grant_log_dir": "/unrelated/grants"})


if __name__ == "__main__":
    unittest.main()
