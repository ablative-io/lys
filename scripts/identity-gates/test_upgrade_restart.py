"""The first start after commit owes legacy rows only to a release that could hold legacy."""

from pathlib import Path
import unittest

from upgrade_restart import budget_family, team_rows

AGENT = "agent-" + "22" * 16
OWNER = "person-" + "33" * 16
TEAM = "op-" + "44" * 16


def fixture(teams):
    return {"release": {"budgets": "limits", "teams": teams}, "team": TEAM,
            "foreign": [AGENT, OWNER]}


class TeamRowTests(unittest.TestCase):
    def test_an_unguarded_release_owes_two_holds_and_one_check(self):
        lines = [{"line": "held", "team": TEAM, "member": AGENT},
                 {"line": "held", "team": TEAM, "member": OWNER}, {"line": "checked"}]
        self.assertEqual(team_rows(lines, fixture("unguarded"), "teams"),
                         {"new_hold_rows": 2, "new_check_rows": 1})
        with self.assertRaisesRegex(RuntimeError, "two holds and one check"):
            team_rows(lines[1:], fixture("unguarded"), "teams")

    def test_a_guarded_release_owes_no_row(self):
        self.assertEqual(team_rows([], fixture("guarded"), "teams"),
                         {"new_hold_rows": 0, "new_check_rows": 0})
        for lines in ([{"line": "checked"}], [{"line": "held", "team": TEAM, "member": AGENT}]):
            with self.subTest(lines=lines), self.assertRaisesRegex(RuntimeError, "no legacy"):
                team_rows(lines, fixture("guarded"), "teams")

    def test_an_unknown_team_model_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, "team model"):
            team_rows([], fixture("other"), "teams")


class BudgetFamilyTests(unittest.TestCase):
    root = Path("/fixture")
    budgets = Path("/fixture/data/budgets")
    before = {"data/budgets/leaves/0": "a", "data/budgets/snapshot.bin": "v1",
              "data/teams/leaves/0": "t"}

    def test_a_per_measure_store_must_be_sealed_again(self):
        after = dict(self.before, **{"data/budgets/snapshot.bin": "v3"})
        budget_family(self.before, after, self.root, self.budgets, "per_measure")
        with self.assertRaisesRegex(RuntimeError, "did not persist"):
            budget_family(self.before, dict(self.before), self.root, self.budgets, "per_measure")

    def test_a_limits_store_must_stay_byte_equal(self):
        budget_family(self.before, dict(self.before), self.root, self.budgets, "limits")
        after = dict(self.before, **{"data/budgets/snapshot.bin": "v3"})
        with self.assertRaisesRegex(RuntimeError, "already current"):
            budget_family(self.before, after, self.root, self.budgets, "limits")

    def test_an_old_leaf_never_changes_in_either_model(self):
        after = dict(self.before, **{"data/budgets/leaves/0": "rewritten",
                                     "data/budgets/snapshot.bin": "v3"})
        for model in ("per_measure", "limits"):
            with self.subTest(model=model), self.assertRaisesRegex(RuntimeError, "leaves/0"):
                budget_family(self.before, after, self.root, self.budgets, model)

    def test_an_unknown_budget_model_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, "budget model"):
            budget_family(self.before, dict(self.before), self.root, self.budgets, "other")


if __name__ == "__main__":
    unittest.main()
