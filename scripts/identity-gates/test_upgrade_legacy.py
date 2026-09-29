"""The upgrade verifier rejects lost history and loosened effective budgets."""

import copy
import unittest

from upgrade_legacy import verify


class Reader:
    def __init__(self, team, budgets):
        self.team = team
        self.budgets = budgets

    def ask(self, method, path):
        if method != "GET":
            raise AssertionError("verification must not write")
        return self.team if path.startswith("/teams/") else self.budgets


def fixture():
    first = {"measure": "context_percent", "limit": 60, "period": None, "act": "tell"}
    tokens = {"measure": "tokens", "limit": 100, "period": {"length": "week"}, "act": "stop"}
    runtime = {"measure": "running_ms", "limit": 1000, "period": {"length": "week"}, "act": "stop"}
    requested = [first, {**tokens, "limit": 200}, {**runtime, "period": {"length": "day"}}]
    team = {"members": ["self", "foreign-agent", "foreign-person"], "created_by": "old-owner"}
    budgets = {"budgets": requested}
    original = {"tokens": tokens, "running_ms": runtime}
    before = {"team": "team", "person": "person", "foreign": team["members"][1:],
              "first": first, "original": original, "team_before": copy.deepcopy(team),
              "budgets_before": copy.deepcopy(budgets)}
    team["held"] = [{"member": who, "reason": "no authority"} for who in before["foreign"]]
    budgets["unconfirmed"] = [
        {"requested": value, "effective": original.get(value["measure"], first), "reason": "legacy"}
        for value in requested]
    return before, Reader(team, budgets)


class LegacyProofTests(unittest.TestCase):
    def test_counts_every_case_and_preserves_the_complete_earlier_budget(self):
        before, reader = fixture()
        self.assertEqual(verify(reader, before), {"held_members": 2, "unconfirmed_budgets": 3})

    def test_each_lost_or_loosened_observation_is_refused(self):
        mutations = [
            lambda read: read.team["members"].remove("foreign-agent"),
            lambda read: read.team["held"].pop(),
            lambda read: read.team["held"][0].update(reason=""),
            lambda read: read.team.update(created_by="invented"),
            lambda read: read.budgets["budgets"][1].update(limit=100),
            lambda read: read.budgets["unconfirmed"].pop(),
            lambda read: read.budgets["unconfirmed"][1]["effective"].update(limit=200),
            lambda read: read.budgets["unconfirmed"][2]["effective"].update(period={"length": "day"}),
            lambda read: read.budgets["unconfirmed"][2]["effective"].update(act="tell"),
            lambda read: read.budgets["unconfirmed"][0].update(reason=""),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(case=index):
                before, reader = fixture()
                # Expected values are independent of the readback under mutation.
                before = copy.deepcopy(before)
                mutate(reader)
                with self.assertRaises(RuntimeError):
                    verify(reader, before)
        self.assertEqual(len(mutations), 10)


if __name__ == "__main__":
    unittest.main()
