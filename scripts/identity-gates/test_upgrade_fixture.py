"""The upgrade proof rejects missing data and rewritten historical app leaves."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from upgrade_fixture import same_records
from upgrade_live import app_leaves, installation, network_subnets, preserve_leaves


class UpgradeProofTests(unittest.TestCase):
    def test_nullable_network_fields_keep_every_assigned_subnet(self):
        self.assertEqual(network_subnets([
            {}, {"IPAM": None}, {"IPAM": {}}, {"IPAM": {"Config": None}},
            {"IPAM": {"Config": []}},
            {"IPAM": {"Config": [{}, {"Subnet": None}, {"Subnet": "172.29.48.0/24"},
                                 {"Subnet": "fd00::/64"}]}},
        ]), {"172.29.48.0/24", "fd00::/64"})

    def test_invalid_subnet_is_not_treated_as_free_space(self):
        with self.assertRaisesRegex(ValueError, "not-a-subnet"):
            network_subnets([{"IPAM": {"Config": [{"Subnet": "not-a-subnet"}]}}])

    def test_docker_null_ipam_config_does_not_hide_an_occupied_subnet(self):
        networks = [
            {"Name": "host", "IPAM": {"Config": None}},
            {"Name": "none", "IPAM": {"Config": []}},
            {"Name": "occupied", "IPAM": {"Config": [{"Subnet": "172.29.48.0/24"}]}},
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            layout = source / "crates/lys/src/identity/install"
            layout.mkdir(parents=True)
            (layout / "layout.rs").write_text(
                "pub const SERVICE_PORT: u16 = 8490;\npub const BROKER_PORT: u16 = 8472;\n")
            (layout / "deployment.template.toml").write_text(
                'project = "lys-identity"\nsubnet = "172.29.47.0/24"\n')
            with patch("upgrade_live.require_free"), patch("upgrade_live.port", return_value=12345), \
                    patch("upgrade_live.subprocess.check_output", side_effect=[
                        "host none occupied", json.dumps(networks)]):
                self.assertEqual(installation(root, source, "fixture"), 8490)
            self.assertEqual((root / "deployment.toml").read_text(),
                             'project = "fixture"\nsubnet = "172.29.49.0/24"\n')

    def test_rollback_readback_accepts_the_actual_old_budget_shape(self):
        before = {"sessions": {"sessions": ["old"]}, "budgets": {"budgets": [100]}}
        same_records(before, copy.deepcopy(before))

    def test_every_domain_difference_is_refused(self):
        before = {name: {"value": name} for name in (
            "people", "grant", "app", "configuration", "budgets", "goals", "policy")}
        before["sessions"] = {"sessions": [{"id": "preserved"}]}
        after = copy.deepcopy(before)
        after["budgets"]["unconfirmed"] = []
        same_records(before, after)
        for name in before:
            with self.subTest(domain=name):
                after = copy.deepcopy(before)
                after["budgets"]["unconfirmed"] = []
                after[name] = {}
                with self.assertRaisesRegex(RuntimeError, name):
                    same_records(before, after)

    def test_only_empty_agent_budget_confirmation_is_additive(self):
        before = {"sessions": {"sessions": ["original"]}, "budgets": {"budgets": [100]}}
        after = copy.deepcopy(before)
        after["budgets"]["unconfirmed"] = []
        same_records(before, after)
        for changed in ([{"requested": 200}], None, "", False):
            with self.subTest(changed=changed):
                after["budgets"]["unconfirmed"] = changed
                with self.assertRaisesRegex(RuntimeError, "budgets"):
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
