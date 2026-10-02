"""The upgrade proof rejects missing data and rewritten historical app leaves."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from upgrade_fixture import same_records
from upgrade_live import app_leaves, installation, network_subnets, preserve_leaves, stamp


AGENT = "agent-" + "22" * 16
OWNER = "person-" + "33" * 16


def records():
    """The old release's readbacks and the candidate's answers to the same routes."""
    budget = {"holder": {"kind": "agent", "id": AGENT}, "measure": "tokens", "limit": 100,
              "period": {"length": "day", "zone": "Australia/Melbourne"}, "act": "tell",
              "version": 1, "by": OWNER, "at": 1500}
    goal = {"id": "op-goal", "holder": {"kind": "agent", "id": AGENT}, "kind": "goal",
            "words": "Preserve this goal", "deadline": 4102444800, "evidence": None,
            "judged_by": None, "reminders": [], "responsible": OWNER, "set_by": OWNER, "at": 1501}
    profile = {"version": 1, "operation": "op-profile", "model_access": ["fixture-model"],
               "tools": ["Read"], "skills": [],
               "mcp_servers": [{"name": "fixture-records", "url": "https://mcp.example.test/records"}],
               "instructions": "Preserve this ordinary legacy profile.", "note": "Old-install proof",
               "set_by": OWNER, "set_at": 1502, "reviewed_by": None, "reviewed_at": None,
               "self_reviewed": False}
    configuration = {
        "source": "startup_configuration", "mutable_in_browser": False,
        "sign_in": {"provider_origin": "http://localhost:18080", "session_seconds": 3600,
                    "secure_cookie": False},
        "directory": {"roles_configured": True},
        "permissions": {"model_version": 1, "projection": "local"},
        "secrets": {"configured": True},
        "runtimes": {"machines_configured": True, "provisioning_configured": True},
        "storage": {"directory_format": "signed_leaf_log", "grant_format": "signed_leaf_log",
                    "requests_configured": True},
    }
    before = {
        "sessions": {"person": OWNER, "sessions": [{"id": "s", "started_at": 1400}]},
        "grant": {"id": "grant"}, "app": {"id": "upgrade_fixture", "state": "approved"},
        "policy": {"agent": AGENT, "policy": {"rules": [{"id": "preserve-denial"}]}},
        "budgets": {"holder": {"kind": "agent", "id": AGENT}, "budgets": [budget]},
        "configuration": configuration,
        "goals": {"goals": [{"goal": goal, "standing": "open", "marked": None,
                             "timers": [], "fired": []}]},
        "provisioning": {"agent": AGENT, "profile": profile,
                         "versions": [{"version": 1, "set_by": OWNER, "set_at": 1502,
                                       "note": "Old-install proof"}],
                         "enforced": False, "recorded": None},
    }
    after = copy.deepcopy(before)
    after["budgets"] = {
        "holder": {"kind": "agent", "id": AGENT},
        "limits": [{"unit": "tokens", "amount": 100, "period": "day", "act": "tell",
                    "zone": "Australia/Melbourne"}],
        "warn_at": None, "zone": "Australia/Melbourne", "version": 1, "by": OWNER, "at": 1500,
        "used": [{"unit": "tokens", "period": "day", "figure": 0, "since_ms": 1,
                  "unavailable": None}],
        "unavailable": [{"unit": "dollars", "reason": "no source reports dollars"},
                        {"unit": "plan_percent", "reason": "no plan window is reported"}],
        "within": [], "unconfirmed": [],
    }
    after["configuration"]["permissions"]["model_version"] = 2
    after["configuration"]["organisation"] = {"zone": "Australia/Melbourne", "version": 1,
                                              "by": "host_setup", "at": 1600}
    after["goals"]["goals"][0]["goal"]["active"] = True
    after["provisioning"]["profile"].update(instructions_mode="append", session=None,
                                            skill_pins=[], harness=None, permissions=None)
    return before, after


def already_new_records():
    """A limits release's own answers: already the candidate's shapes, so the upgrade
    must answer every one of them exactly as the old release did."""
    old, new = records()
    before = copy.deepcopy(new)
    return before, copy.deepcopy(before)


class AlreadyNewReleaseTests(unittest.TestCase):
    def test_equal_answers_pass(self):
        before, after = already_new_records()
        same_records(before, after)

    def test_each_changed_value_is_refused(self):
        mutations = [
            ("budgets", lambda value: value["budgets"]["limits"][0].update(amount=200)),
            ("budgets", lambda value: value["budgets"]["limits"][0].update(period="week")),
            ("budgets", lambda value: value["budgets"]["limits"][0].update(zone="UTC")),
            ("budgets", lambda value: value["budgets"]["limits"][0].update(act="stop")),
            ("budgets", lambda value: value["budgets"]["limits"].append(
                {"unit": "running_ms", "amount": 1, "period": "day", "act": "stop"})),
            ("budgets", lambda value: value["budgets"].update(unconfirmed=[{"requested": {}}])),
            ("budgets", lambda value: value["budgets"].update(
                effective_limits=value["budgets"]["limits"])),
            ("budgets", lambda value: value["budgets"].update(version=2)),
            ("budgets", lambda value: value["budgets"].update(by="person-other")),
            ("budgets", lambda value: value["budgets"].update(warn_at=80)),
            ("budgets", lambda value: value["budgets"]["used"][0].update(figure=5)),
            ("budgets", lambda value: value["budgets"]["unavailable"].pop()),
            ("budgets", lambda value: value["budgets"].update(within=[{"team": "t"}])),
            ("configuration", lambda value: value["configuration"]["permissions"].update(
                model_version=3)),
            ("configuration", lambda value: value["configuration"]["sign_in"].update(
                session_seconds=1)),
            ("configuration", lambda value: value["configuration"]["organisation"].update(
                zone="UTC")),
            ("configuration", lambda value: value["configuration"]["organisation"].update(at=1)),
            ("configuration", lambda value: value["configuration"].pop("organisation")),
            ("goals", lambda value: value["goals"]["goals"][0]["goal"].update(active=False)),
            ("goals", lambda value: value["goals"]["goals"][0]["goal"].pop("active")),
            ("goals", lambda value: value["goals"]["goals"][0]["goal"].update(words="Other")),
            ("provisioning", lambda value: value["provisioning"]["profile"].update(
                instructions_mode="replace")),
            ("provisioning", lambda value: value["provisioning"]["profile"].update(
                skill_pins=["pinned"])),
            ("provisioning", lambda value: value["provisioning"]["profile"].pop("harness")),
            ("provisioning", lambda value: value["provisioning"]["profile"].update(note="Other")),
        ]
        for index, (domain, mutate) in enumerate(mutations):
            with self.subTest(case=index, domain=domain):
                before, after = already_new_records()
                mutate(after)
                with self.assertRaisesRegex(RuntimeError, domain):
                    same_records(before, after)

    def test_a_limits_answer_holding_unconfirmed_budgets_is_refused(self):
        before, after = already_new_records()
        before["budgets"]["unconfirmed"] = [{"requested": {}}]
        after["budgets"]["unconfirmed"] = [{"requested": {}}]
        with self.assertRaisesRegex(RuntimeError, "already held budgets unconfirmed"):
            same_records(before, after)


class UpgradeProofTests(unittest.TestCase):
    def test_installed_stamps_use_only_the_installers_declared_binaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            installed = ("lys-secrets", "lys-identity-server")
            for name in installed:
                (root / name).write_bytes(b"binary")
            with patch("upgrade_live.subprocess.check_output", return_value="program (commit)") as run:
                result = stamp(root, "commit", programs=installed)
            self.assertEqual(list(result), list(installed))
            self.assertEqual([call.args[0][0] for call in run.call_args_list],
                             [str(root / name) for name in installed])
            self.assertFalse((root / "lys").exists())

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
        before, after = records()
        rolled_back = copy.deepcopy(before)
        rolled_back["configuration"]["permissions"]["model_version"] = 2
        same_records(before, rolled_back)
        with self.assertRaisesRegex(RuntimeError, "model version is not 2"):
            same_records(before, copy.deepcopy(before))

    def test_realistic_old_and_candidate_readbacks_agree(self):
        before, after = records()
        same_records(before, after)

    def test_every_domain_difference_is_refused(self):
        before, after = records()
        for name in before:
            with self.subTest(domain=name):
                changed = copy.deepcopy(after)
                changed[name] = {}
                with self.assertRaisesRegex(RuntimeError, name):
                    same_records(before, changed)

    def test_each_agent_budget_fact_is_refused(self):
        mutations = [
            ("holder", lambda budgets: budgets.update(holder={"kind": "agent", "id": "other"})),
            ("amount", lambda budgets: budgets["limits"][0].update(amount=200)),
            ("period", lambda budgets: budgets["limits"][0].update(period="week")),
            ("zone", lambda budgets: budgets["limits"][0].update(zone="UTC")),
            ("act", lambda budgets: budgets["limits"][0].update(act="stop")),
            ("version", lambda budgets: budgets.update(version=2)),
            ("by", lambda budgets: budgets.update(by="person-other")),
            ("at", lambda budgets: budgets.update(at=1)),
            ("organisation zone", lambda budgets: budgets.update(zone="UTC")),
            ("warn_at", lambda budgets: budgets.update(warn_at=80)),
            ("within", lambda budgets: budgets.update(within=[{"team": "held"}])),
            ("unconfirmed", lambda budgets: budgets.update(unconfirmed=[{"requested": {}}])),
            ("effective", lambda budgets: budgets.update(effective_limits=budgets["limits"])),
            ("used", lambda budgets: budgets.update(used=[])),
            ("used unit", lambda budgets: budgets["used"][0].update(unit="running_ms")),
            ("gap", lambda budgets: budgets["used"][0].update(figure=None)),
            ("unavailable", lambda budgets: budgets["unavailable"].append(budgets["unavailable"][0])),
            ("extra", lambda budgets: budgets.update(budgets=[])),
        ]
        for fact, mutate in mutations:
            with self.subTest(fact=fact):
                before, after = records()
                mutate(after["budgets"])
                with self.assertRaisesRegex(RuntimeError, "budgets"):
                    same_records(before, after)

    def test_each_configuration_fact_is_refused(self):
        mutations = [
            ("model kept", lambda value: value["permissions"].update(model_version=1)),
            ("model beyond", lambda value: value["permissions"].update(model_version=3)),
            ("old setting", lambda value: value["sign_in"].update(session_seconds=1)),
            ("zone version", lambda value: value["organisation"].update(version=2)),
            ("zone source", lambda value: value["organisation"].update(by="person-other")),
            ("zone empty", lambda value: value["organisation"].update(zone="")),
            ("zone before session", lambda value: value["organisation"].update(at=1)),
            ("zone member", lambda value: value["organisation"].update(extra=True)),
        ]
        for fact, mutate in mutations:
            with self.subTest(fact=fact):
                before, after = records()
                mutate(after["configuration"])
                with self.assertRaisesRegex(RuntimeError, "configuration"):
                    same_records(before, after)

    def test_a_shipped_model_is_never_raised(self):
        before, after = records()
        before["configuration"]["permissions"]["model_version"] = 2
        same_records(before, after)
        after["configuration"]["permissions"]["model_version"] = 3
        with self.assertRaisesRegex(RuntimeError, "configuration"):
            same_records(before, after)

    def test_each_goal_fact_is_refused(self):
        mutations = [
            ("inactive", lambda goals: goals["goals"][0]["goal"].update(active=False)),
            ("words", lambda goals: goals["goals"][0]["goal"].update(words="Rewritten")),
            ("deadline", lambda goals: goals["goals"][0]["goal"].update(deadline=None)),
            ("standing", lambda goals: goals["goals"][0].update(standing="met")),
            ("lost", lambda goals: goals["goals"].pop()),
        ]
        for fact, mutate in mutations:
            with self.subTest(fact=fact):
                before, after = records()
                mutate(after["goals"])
                with self.assertRaisesRegex(RuntimeError, "goals"):
                    same_records(before, after)

    def test_empty_session_proof_is_refused(self):
        empty = {"sessions": {"sessions": []}}
        with self.assertRaisesRegex(RuntimeError, "no session"):
            same_records(empty, empty)

    def test_legacy_profile_allows_only_declared_empty_additions(self):
        before, after = records()
        same_records(before, after)
        for field, changed in [("skill_pins", ["unexpected"]), ("harness", {}),
                               ("permissions", {}), ("instructions_mode", "replace"),
                               ("session", {}), ("runs_on", "machine"), ("version", 2),
                               ("unknown", None)]:
            value = copy.deepcopy(after)
            value["provisioning"]["profile"][field] = changed
            with self.subTest(field=field), self.assertRaisesRegex(RuntimeError, "provisioning"):
                same_records(before, value)
        self.assertEqual(before["provisioning"]["profile"]["version"], 1)

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
