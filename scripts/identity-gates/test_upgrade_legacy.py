"""The upgrade verifier rejects lost history and loosened effective budgets."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from upgrade_legacy import PERSON_LIMITS, expected_hold, seed, verify

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
    before = {"release": {"budgets": "per_measure", "teams": "unguarded"},
              "team": TEAM, "person": PERSON, "foreign": [AGENT, OWNER], "added": added,
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


class OldRelease:
    """One old release's routes as the seed reaches them, with that release's authority.

    `unguarded` per-measure releases accept the member's own budget edits and foreign
    team adds; `guarded` limits releases refuse both by name, as their source does.
    """

    def __init__(self, budgets, teams, refusal_names=None):
        self.budgets = budgets
        self.teams = teams
        self.guards_teams = teams == "guarded"
        self.names = refusal_names or {"agent": "not_permitted", "person": "NotAdmitted",
                                       "budget": "not_permitted"}
        self.calls = []
        self.members = []
        self.limits = None
        self.version = 0

    def caller(self, who):
        release = self

        class Caller:
            port = 1

            def ask(inner, method, path, body=None, expected_status=200):
                status, answer = release.answer(who, method, path, body)
                release.calls.append((who, method, path, body, status))
                if status != expected_status:
                    raise RuntimeError(f"{method} {path}: HTTP {status}: {answer}")
                return answer
        return Caller()

    def answer(self, who, method, path, body):
        if path == "/me":
            return 200, {"person": {"id": PERSON}}
        if path == "/teams" and method == "POST":
            return 200, {"id": TEAM}
        if path == f"/teams/{TEAM}/members":
            foreign = body["member"] != PERSON
            if self.guards_teams and who == "member" and foreign:
                name = self.names["agent" if body["member"] == AGENT else "person"]
                return 403, {"refusal": name, "reason": "refused"}
            self.members.append(body["member"])
            return 200, {"recorded": {"operation": body["operation"], "member": body["member"],
                                      "by": LOGIN, "at": 990 + len(self.members)}}
        if path == f"/teams/{TEAM}":
            return 200, {"id": TEAM, "members": list(self.members), "held": [], "parent": None,
                         "lead": None}
        if path == f"/budgets/person/{PERSON}" and method == "PUT":
            if self.budgets == "per_measure":
                return 200, dict(body, holder={"kind": "person", "id": PERSON})
            if who == "member":
                return 403, {"refusal": self.names["budget"], "reason": "refused"}
            self.limits, self.version = body["limits"], self.version + 1
            return 200, {"limits": self.limits, "unconfirmed": [], "version": self.version}
        if path == f"/budgets/person/{PERSON}":
            return 200, {"limits": self.limits, "unconfirmed": [], "version": self.version}
        return 200, {}

    def seed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "identity.json").write_text(json.dumps({"issuer": ISSUER}))
            ids = {"person": PERSON, "agent": AGENT, "owner": OWNER}
            with patch("upgrade_legacy.issuer_person", return_value="legacy-member-subject"), \
                    patch("upgrade_legacy.Browser", return_value=self.caller("member")):
                return seed(self.caller("admin"), root, ids,
                            {"budgets": self.budgets, "teams": self.teams})[0]

    def sent(self, method, path):
        return [(who, body, status) for who, verb, at, body, status in self.calls
                if (verb, at) == (method, path)]


class SeedShapeTests(unittest.TestCase):
    def test_a_per_measure_release_is_sent_each_measure_and_the_members_own_edits(self):
        release = OldRelease("per_measure", "unguarded")
        fixture = release.seed()
        week, day = {"length": "week", "zone": "UTC"}, {"length": "day", "zone": "UTC"}
        def body(version, measure, limit, period, act):
            return {"version": version, "measure": measure, "limit": limit, "period": period,
                    "act": act}
        self.assertEqual(release.sent("PUT", f"/budgets/person/{PERSON}"), [
            ("member", body(0, "context_percent", 60, None, "tell"), 200),
            ("admin", body(0, "tokens", 100, week, "stop"), 200),
            ("member", body(1, "tokens", 200, week, "stop"), 200),
            ("admin", body(0, "running_ms", 1000, week, "stop"), 200),
            ("member", body(1, "running_ms", 1000, day, "stop"), 200),
        ])
        self.assertEqual([who for who, body, status in release.sent("POST", f"/teams/{TEAM}/members")],
                         ["member"] * 3)
        self.assertEqual(fixture["release"], {"budgets": "per_measure", "teams": "unguarded"})
        self.assertEqual(fixture["refusals"], {})
        self.assertEqual(set(fixture["original"]), {"tokens", "running_ms"})

    def test_a_limits_release_is_sent_the_same_limits_once_and_refuses_the_self_edit(self):
        release = OldRelease("limits", "guarded")
        fixture = release.seed()
        limits = [{"unit": "context_percent", "amount": 60, "period": None, "act": "tell"},
                  {"unit": "tokens", "amount": 100, "period": "week", "zone": "UTC", "act": "stop"},
                  {"unit": "running_ms", "amount": 1000, "period": "week", "zone": "UTC",
                   "act": "stop"}]
        edit = [limits[0], dict(limits[1], amount=200), dict(limits[2], period="day")]
        self.assertEqual(release.sent("PUT", f"/budgets/person/{PERSON}"), [
            ("admin", {"version": 0, "limits": limits, "warn_at": None}, 200),
            ("member", {"version": 1, "limits": edit, "warn_at": None}, 403),
        ])
        self.assertEqual([(who, body["member"], status) for who, body, status
                          in release.sent("POST", f"/teams/{TEAM}/members")], [
            ("member", PERSON, 200), ("member", AGENT, 403), ("member", OWNER, 403),
            ("admin", AGENT, 200), ("admin", OWNER, 200)])
        self.assertEqual(fixture["refusals"], {"agent": "not_permitted", "person": "NotAdmitted",
                                               "self_edit": "not_permitted"})
        self.assertEqual(fixture["limits"], limits)
        self.assertNotIn("first", fixture)

    def test_a_guarded_release_that_accepts_a_foreign_add_is_refused(self):
        release = OldRelease("limits", "guarded")
        release.guards_teams = False
        with self.assertRaisesRegex(RuntimeError, "HTTP 200"):
            release.seed()

    def test_a_refusal_under_another_name_is_refused(self):
        for case in ("agent", "person", "budget"):
            names = {"agent": "not_permitted", "person": "NotAdmitted", "budget": "not_permitted",
                     case: "SomethingElse"}
            with self.subTest(case=case), self.assertRaisesRegex(RuntimeError, "SomethingElse"):
                OldRelease("limits", "guarded", names).seed()

    def test_an_unknown_model_is_refused_by_name(self):
        with self.assertRaisesRegex(RuntimeError, "team model"):
            OldRelease("limits", "other").seed()
        with self.assertRaisesRegex(RuntimeError, "budget model"):
            OldRelease("other", "unguarded").seed()


def limits_fixture():
    """A limits release's own answers before upgrade, and the candidate's after."""
    team = {"id": TEAM, "owner": PERSON, "parent": None, "lead": None, "name": "Legacy team",
            "description": "Upgrade authority proof", "members": [PERSON, AGENT, OWNER], "held": [],
            "state": "active", "created_by": LOGIN, "created_at": 980, "retired_at": None}
    budgets = {
        "holder": {"kind": "person", "id": PERSON}, "limits": copy.deepcopy(PERSON_LIMITS),
        "warn_at": None, "zone": ZONE, "version": 1, "by": OWNER, "at": 1001,
        "used": [{"unit": "context_percent", "period": None, "figure": None, "since_ms": None,
                  "unavailable": "no live session has reported its context"},
                 {"unit": "tokens", "period": "week", "figure": 0, "since_ms": 5, "unavailable": None},
                 {"unit": "running_ms", "period": "week", "figure": 0, "since_ms": 5,
                  "unavailable": None}],
        "unavailable": [{"unit": "dollars", "reason": "no source reports dollars"}],
        "within": [], "unconfirmed": [],
    }
    before = {"release": {"budgets": "limits", "teams": "guarded"}, "team": TEAM, "person": PERSON,
              "foreign": [AGENT, OWNER], "added": {}, "limits": copy.deepcopy(PERSON_LIMITS),
              "refusals": {"agent": "not_permitted", "person": "NotAdmitted",
                           "self_edit": "not_permitted"},
              "team_before": copy.deepcopy(team), "budgets_before": copy.deepcopy(budgets)}
    configuration = {"organisation": {"zone": ZONE, "version": 1, "by": "host_setup", "at": 900}}
    return before, Reader(team, budgets, configuration)


class LimitsReleaseProofTests(unittest.TestCase):
    def test_nothing_is_held_and_every_limit_is_kept(self):
        before, reader = limits_fixture()
        self.assertEqual(verify(reader, before), {"held_members": 0, "unconfirmed_budgets": 0})

    def test_each_changed_value_is_refused(self):
        mutations = [
            lambda read: read.team["members"].remove(AGENT),
            lambda read: read.team["held"].append({"member": AGENT, "reason": "held"}),
            lambda read: read.team.update(parent=TEAM),
            lambda read: read.team.update(lead=PERSON),
            lambda read: read.team.update(name="Renamed"),
            lambda read: read.team.update(unexpected=True),
            lambda read: read.budgets["limits"][1].update(amount=200),
            lambda read: read.budgets["limits"][2].update(period="day"),
            lambda read: read.budgets["limits"][1].update(zone="Australia/Melbourne"),
            lambda read: read.budgets["limits"][0].update(act="stop"),
            lambda read: read.budgets["limits"].pop(),
            lambda read: read.budgets.update(unconfirmed=[{"requested": {}, "effective": {},
                                                           "reason": "held"}]),
            lambda read: read.budgets.update(effective_limits=copy.deepcopy(PERSON_LIMITS)),
            lambda read: read.budgets.update(version=2),
            lambda read: read.budgets.update(by=PERSON),
            lambda read: read.budgets.update(at=1),
            lambda read: read.budgets.update(warn_at=80),
            lambda read: read.budgets.update(zone="UTC"),
            lambda read: read.budgets["used"][1].update(figure=3),
            lambda read: read.budgets["unavailable"].pop(),
            lambda read: read.budgets.update(within=[{"team": TEAM}]),
            lambda read: read.budgets.pop("unconfirmed"),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(case=index):
                before, reader = limits_fixture()
                mutate(reader)
                with self.assertRaises(RuntimeError):
                    verify(reader, before)

    def test_a_seed_the_release_did_not_hold_is_refused(self):
        before, reader = limits_fixture()
        before["limits"][1]["amount"] = 200
        with self.assertRaisesRegex(RuntimeError, "differ from those it was given"):
            verify(reader, before)

    def test_a_held_or_nested_seed_cannot_stand_for_a_guarded_release(self):
        for change in ({"held": [{"member": AGENT}]}, {"parent": TEAM}, {"members": [PERSON]}):
            with self.subTest(change=change):
                before, reader = limits_fixture()
                before["team_before"].update(change)
                reader.team.update(copy.deepcopy(change))
                with self.assertRaisesRegex(RuntimeError, "guarded release"):
                    verify(reader, before)

    def test_an_unconfirmed_budget_before_upgrade_is_refused(self):
        before, reader = limits_fixture()
        held = [{"requested": {}, "effective": {}, "reason": "held"}]
        before["budgets_before"]["unconfirmed"] = held
        reader.budgets["unconfirmed"] = copy.deepcopy(held)
        with self.assertRaisesRegex(RuntimeError, "already held"):
            verify(reader, before)


if __name__ == "__main__":
    unittest.main()
