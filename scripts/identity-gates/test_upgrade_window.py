"""Upgrade proof oracles reject added records, damaged snapshots and generic failures."""

import json
from pathlib import Path
import tempfile
import unittest

from upgrade_negative import require_old_refusal
from upgrade_window import family_files, pending, profile_file, refuse_operator, refused_writes, unchanged


class WindowProofTests(unittest.TestCase):
    def test_operator_refusal_requires_named_reason_and_no_appended_leaf(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            token = root / "operator-token"
            token.write_text("fixture-token")
            log = root / "log"
            leaves = log / "leaves"
            leaves.mkdir(parents=True)
            (leaves / "000").write_bytes(b"old signed record")
            config = {"operator_token_file": str(token), "log_dir": str(log)}
            class Reader:
                def ask(inner, method, path, body, **options):
                    self.assertEqual((method, path), ("POST", "/agents"))
                    self.assertEqual(options, {"expected_status": 401, "operator": "fixture-token"})
                    if inner.append:
                        (leaves / "001").write_bytes(b"unexpected write")
                    return inner.answer
            reader = Reader()
            reader.append = False
            reader.answer = {"refusal": "OperatorRefused", "reason": "upgrade is reversible"}
            refuse_operator(reader, root, config)
            reader.append = True
            with self.assertRaisesRegex(RuntimeError, "001"):
                refuse_operator(reader, root, config)
            (leaves / "001").unlink()
            reader.append = False
            reader.answer = {"refusal": "OperatorRefused", "reason": "wrong token"}
            with self.assertRaisesRegex(RuntimeError, "reversible"):
                refuse_operator(reader, root, config)

    def test_all_five_mutations_use_real_record_ids_and_stored_versions(self):
        legacy = {"team": "kept-team", "foreign": ["kept-member"], "person": "kept-person",
                  "budgets_before": {"budgets": [{"measure": "tokens", "version": 17}]}}
        profile = {"version": 9, "model_access": ["kept-model"], "tools": ["Read"],
                   "skills": [], "mcp_servers": [{"name": "kept", "url": "https://example.test/mcp"}],
                   "instructions": "Kept instructions", "note": "Kept note"}
        writes = refused_writes(legacy, {"agent": "kept-agent", "profile": profile})
        self.assertEqual([route for route, body, kind in writes], [
            "/teams/kept-team/members/kept-member/confirm", "/budgets/person/kept-person/confirm",
            "/agents/kept-agent/provisioning", "/agents/kept-agent/provisioning/9/review", "/skills"])
        self.assertEqual(writes[1][1], {"measure": "tokens", "version": 17})
        self.assertEqual(writes[2][1]["from_version"], 9)
        self.assertEqual(writes[2][1]["mcp_servers"], profile["mcp_servers"])
        self.assertEqual([kind for route, body, kind in writes], [
            "TeamsUnavailable", "BudgetsUnavailable", *(["ProvisioningUnavailable"] * 3)])

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
