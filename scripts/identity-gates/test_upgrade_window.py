"""Upgrade proof oracles reject added records, damaged snapshots and generic failures."""

import json
from pathlib import Path
import tempfile
import unittest

from upgrade_negative import require_old_refusal
from upgrade_window import family_files, pending, profile_file, unchanged


class WindowProofTests(unittest.TestCase):
    def test_an_added_profile_member_is_a_byte_change_even_when_empty(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            path = root / "profiles.json"
            path.write_text('{"profiles":[]}')
            config = {"provisioning_file": str(path)}
            before = profile_file(root, config)
            path.write_text('{"profiles":[],"skills":[]}')
            with self.assertRaisesRegex(RuntimeError, "profiles.json"):
                unchanged(before, profile_file(root, config))

    def test_every_added_removed_or_rewritten_byte_names_the_record(self):
        before = {"teams/leaves/000": "original", "budgets/snapshot": "signed"}
        for after, name in [
            ({**before, "teams/leaves/001": "held"}, "teams/leaves/001"),
            ({"teams/leaves/000": "original"}, "budgets/snapshot"),
            ({**before, "budgets/snapshot": "v2"}, "budgets/snapshot"),
        ]:
            with self.subTest(record=name), self.assertRaisesRegex(RuntimeError, name):
                unchanged(before, after)
        unchanged(before, dict(before))

    def test_family_hashes_include_snapshots_and_refuse_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            config = {}
            for family in ("teams", "budgets"):
                folder = root / family
                folder.mkdir()
                (folder / "snapshot").write_bytes(b"signed legacy snapshot")
                config[family + "_dir"] = str(folder)
            before = family_files(root, config)
            self.assertEqual(set(before), {"teams/snapshot", "budgets/snapshot"})
            (root / "teams/snapshot").write_bytes(b"new format")
            with self.assertRaisesRegex(RuntimeError, "teams/snapshot"):
                unchanged(before, family_files(root, config))
            with self.assertRaisesRegex(RuntimeError, "teams_dir"):
                family_files(root, {**config, "teams_dir": str(root.parent)})

    def test_ready_window_must_not_have_committed_started(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "install").mkdir()
            record = root / "install/upgrade.json"
            record.write_text(json.dumps({"steps": ["binaries_placed"]}))
            pending(root)
            for steps in ([], ["binaries_placed", "started"]):
                record.write_text(json.dumps({"steps": steps}))
                with self.assertRaisesRegex(RuntimeError, "reversible window"):
                    pending(root)

    def test_negative_control_requires_its_exact_leaf_decoder_refusal(self):
        path = Path("/fixture/teams/leaves/00000000000000000012")
        require_old_refusal("leaf 12 is not a team line: unknown variant `held`", path)
        for text in ("port busy", "leaf 11 is not a team line: unknown variant `held`",
                     "leaf 12 is not a team line: malformed json"):
            with self.subTest(text=text), self.assertRaisesRegex(RuntimeError, str(path)):
                require_old_refusal(text, path)


if __name__ == "__main__":
    unittest.main()
