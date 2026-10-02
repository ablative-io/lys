"""The upgrade verifier rejects lost history and loosened effective budgets."""

import copy
import unittest

from upgrade_legacy import expected_hold, verify

PERSON = "person-" + "11" * 16
AGENT = "agent-" + "22" * 16
OWNER = "person-" + "33" * 16
TEAM = "op-" + "44" * 16
ISSUER = "http://localhost:18080/auth/v1"
LOGIN = {"provider": ISSUER, "subject": "legacy-member-subject"}
ZONE = "Australia/Melbourne"


def budget(measure, limit, period, act, version, by, at):
    return {"holder": {"kind": "person", "id": PERSON}, "measure": measure, "limit": limit,
            "period": period, "act": act, "version": version, "by": by, "at": at}


def fixture():
    """The old release's own answers, and the candidate's answers after upgrade."""
    first = budget("context_percent", 60, None, "tell", 1, PERSON, 1000)
    week = {"length": "week", "zone": "UTC"}
    original = {"tokens": budget("tokens", 100, week, "stop", 1, OWNER, 1001),
                "running_ms": budget("running_ms", 1000, week, "stop", 1, OWNER, 1003)}
    requested = [first, budget("tokens", 200, week, "stop", 2, PERSON, 1002),
                 budget("running_ms", 1000, {"length": "day", "zone": "UTC"}, "stop", 2, PERSON, 1004)]
    members = [PERSON, AGENT, OWNER]
    added = {who: {"operation": "op-" + str(index) * 32, "act": "added", "member": who,
                   "by": LOGIN, "at": 990 + index} for index, who in enumerate(members)}
    old_team = {"id": TEAM, "owner": PERSON, "name": "Legacy team",
                "description": "Upgrade authority proof", "members": members, "state": "active",
                "created_by": LOGIN, "created_at": 980, "retired_at": None}
    before = {"team": TEAM, "person": PERSON, "foreign": [AGENT, OWNER], "added": added,
              "first": first, "original": original, "team_before": copy.deepcopy(old_team),
              "budgets_before": {"holder": {"kind": "person", "id": PERSON},
                                 "budgets": copy.deepcopy(requested)}}
    team = dict(copy.deepcopy(old_team), parent=None, lead=None,
                held=[expected_hold(TEAM, who, added[who]) for who in (AGENT, OWNER)])
    effective = [first, original["tokens"], original["running_ms"]]
    limits = [{"unit": "context_percent", "amount": 60, "period": None, "act": "tell"},
              {"unit": "tokens", "amount": 200, "period": "week", "act": "stop", "zone": "UTC"},
              {"unit": "running_ms", "amount": 1000, "period": "day", "act": "stop", "zone": "UTC"}]
    enforced = [limits[0],
                {"unit": "tokens", "amount": 100, "period": "week", "act": "stop", "zone": "UTC"},
                {"unit": "running_ms", "amount": 1000, "period": "week", "act": "stop", "zone": "UTC"}]
    budgets = {
        "holder": {"kind": "person", "id": PERSON}, "limits": limits, "warn_at": None,
        "zone": ZONE, "version": 5, "by": PERSON, "at": 1004,
        "used": [{"unit": "context_percent", "period": None, "figure": None, "since_ms": None,
                  "unavailable": "no live session has reported its context"},
                 {"unit": "tokens", "period": "week", "figure": 0, "since_ms": 5, "unavailable": None},
                 {"unit": "running_ms", "period": "week", "figure": 0, "since_ms": 5,
                  "unavailable": None}],
        "unavailable": [{"unit": "dollars", "reason": "no source reports dollars"},
                        {"unit": "context_percent",
                         "reason": "no live session has reported its context"}],
        "within": [], "effective_limits": enforced,
        "unconfirmed": [{"requested": copy.deepcopy(value), "effective": copy.deepcopy(kept),
                         "reason": "a legacy self-set personal budget requires confirmation"}
                        for value, kept in zip(requested, effective)],
    }
    configuration = {"organisation": {"zone": ZONE, "version": 1, "by": "host_setup", "at": 2000}}
    return before, Reader(team, budgets, configuration)


class Reader:
    def __init__(self, team, budgets, configuration):
        self.team = team
        self.budgets = budgets
        self.configuration = configuration

    def ask(self, method, path):
        if method != "GET":
            raise AssertionError("verification must not write")
        if path.startswith("/teams/"):
            return self.team
        if path == "/configuration":
            return self.configuration
        return self.budgets


class LegacyProofTests(unittest.TestCase):
    def test_counts_every_case_and_preserves_the_complete_earlier_budget(self):
        before, reader = fixture()
        self.assertEqual(verify(reader, before), {"held_members": 2, "unconfirmed_budgets": 3})

    def test_the_hold_names_the_old_adding_record(self):
        hold = expected_hold(TEAM, AGENT, {"operation": "op-1", "by": LOGIN, "at": 7})
        self.assertEqual(hold["operation"], "op-54308d40818188eec8941926c2267c10")
        self.assertEqual((hold["team"], hold["member"], hold["at"]), (TEAM, AGENT, 7))
        self.assertEqual(hold["reason"], f"membership added by {ISSUER}/legacy-member-subject under "
                                         "operation `op-1` has no current authority for this member")

    def test_each_lost_or_loosened_observation_is_refused(self):
        mutations = [
            lambda read: read.team["members"].remove(AGENT),
            lambda read: read.team["members"].remove(PERSON),
            lambda read: read.team.update(created_by={"provider": ISSUER, "subject": "invented"}),
            lambda read: read.team.update(name="Renamed"),
            lambda read: read.team.update(parent=TEAM),
            lambda read: read.team.update(lead=PERSON),
            lambda read: read.team.update(unexpected=True),
            lambda read: read.team["held"].pop(),
            lambda read: read.team["held"].reverse(),
            lambda read: read.team["held"][0].update(reason=""),
            lambda read: read.team["held"][0].update(at=1),
            lambda read: read.team["held"][1].update(operation="op-" + "0" * 32),
            lambda read: read.team["held"].append(dict(read.team["held"][0], member=PERSON)),
            lambda read: read.budgets["limits"][1].update(amount=100),
            lambda read: read.budgets["limits"][2].update(period="week"),
            lambda read: read.budgets["limits"][1].pop("zone"),
            lambda read: read.budgets["limits"].pop(),
            lambda read: read.budgets.update(holder={"kind": "person", "id": OWNER}),
            lambda read: read.budgets.update(version=4),
            lambda read: read.budgets.update(by=OWNER),
            lambda read: read.budgets.update(at=1003),
            lambda read: read.budgets.update(zone="UTC"),
            lambda read: read.budgets.update(warn_at=80),
            lambda read: read.budgets.update(within=[{"team": TEAM}]),
            lambda read: read.budgets.pop("effective_limits"),
            lambda read: read.budgets["effective_limits"][1].update(amount=200),
            lambda read: read.budgets["unconfirmed"].pop(),
            lambda read: read.budgets["unconfirmed"][1]["requested"].update(limit=100),
            lambda read: read.budgets["unconfirmed"][1]["effective"].update(limit=200),
            lambda read: read.budgets["unconfirmed"][2]["effective"].update(
                period={"length": "day", "zone": "UTC"}),
            lambda read: read.budgets["unconfirmed"][2]["effective"].update(act="tell"),
            lambda read: read.budgets["unconfirmed"][0].update(reason=""),
            lambda read: read.budgets["unconfirmed"].append(read.budgets["unconfirmed"][0]),
            lambda read: read.budgets["used"].pop(),
            lambda read: read.budgets["used"][1].update(figure=None),
            lambda read: read.budgets["used"][0].update(figure=3),
            lambda read: read.budgets["unavailable"].pop(),
            lambda read: read.budgets["unavailable"][0].update(reason=""),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(case=index):
                before, reader = fixture()
                # Expected values are independent of the readback under mutation.
                before = copy.deepcopy(before)
                mutate(reader)
                with self.assertRaises(RuntimeError):
                    verify(reader, before)
        self.assertEqual(len(mutations), 38)

    def test_the_old_answer_must_hold_the_three_pending_measures(self):
        before, reader = fixture()
        before["budgets_before"]["budgets"].pop()
        with self.assertRaisesRegex(RuntimeError, "measures differ"):
            verify(reader, before)


if __name__ == "__main__":
    unittest.main()
