"""The old release's shapes are read from its own source, never from a list of commits."""

from pathlib import Path
import subprocess
import unittest

from upgrade_release import (
    BUDGETS_API, INTENT, TEAMS_API, budget_model, put_back_model, zone_model, zoned, limits_body, old_release, per_measure_body, struct_fields,
    team_model,
)

PER_MEASURE_SOURCE = '''
/// A budget to set on a holder.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BudgetBody {
    measure: Measure,
    limit: u64,
    #[serde(default)]
    period: Option<Period>,
    act: Act,
    /// The version read before this change; 0 for a budget not yet set.
    version: u64,
}
'''
LIMITS_SOURCE = '''
pub(crate) struct BudgetBody {
    limits: Vec<Limit>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    warn_at: Option<serde_json::Number>,
    /// The version read before this change; 0 for a budget not yet set.
    version: u64,
}
'''
TEAM_SOURCE = '''
pub struct TeamView {
    /// The team, named by the operation id it was created with.
    pub id: String,
    pub members: Vec<String>,
    /// Memberships excluded from team actions until an administrator confirms them.
    pub held: Vec<crate::teams_state::Hold>,
    pub created_by: Login,
}
'''


class ReleaseShapeTests(unittest.TestCase):
    def test_fields_are_read_without_comments_attributes_or_paths(self):
        self.assertEqual(struct_fields(LIMITS_SOURCE, "BudgetBody", "x"),
                         ["limits", "warn_at", "version"])
        self.assertEqual(struct_fields(TEAM_SOURCE, "TeamView", "x"),
                         ["id", "members", "held", "created_by"])

    def test_each_budget_request_struct_names_its_model(self):
        self.assertEqual(budget_model(PER_MEASURE_SOURCE), "per_measure")
        self.assertEqual(budget_model(LIMITS_SOURCE), "limits")

    def test_an_unknown_or_absent_budget_request_is_refused_by_name(self):
        with self.assertRaisesRegex(RuntimeError, "warn_at"):
            budget_model(LIMITS_SOURCE.replace("    version: u64,\n", ""))
        with self.assertRaisesRegex(RuntimeError, "no struct BudgetBody"):
            budget_model("pub struct Other {\n    a: u8,\n}\n")

    def test_a_team_answer_with_held_names_a_guarded_release(self):
        self.assertEqual(team_model(TEAM_SOURCE), "guarded")
        unguarded = TEAM_SOURCE.replace("    pub held: Vec<crate::teams_state::Hold>,\n", "")
        self.assertEqual(team_model(unguarded), "unguarded")

    def test_each_baseline_is_read_from_its_own_tree(self):
        root = Path(__file__).resolve().parents[2]
        leg = (root / "scripts/identity-gates/upgrade_proof_leg.sh").read_text()
        baselines = leg.split('baselines="', 1)[1].split('"', 1)[0].split()
        self.assertEqual(len(baselines), 4)
        found = {}
        for commit in baselines:
            def read(path, commit=commit):
                return subprocess.check_output(
                    ["git", "-C", str(root), "show", f"{commit}:{path}"], text=True)
            found[commit[:8]] = old_release(read)
        self.assertEqual(found, {
            "1b568cd9": {"budgets": "per_measure", "teams": "unguarded", "put_back": "no_data", "zones": "explicit"},
            "8c064b62": {"budgets": "per_measure", "teams": "unguarded", "put_back": "no_data", "zones": "explicit"},
            "cced4195": {"budgets": "limits", "teams": "guarded", "put_back": "keeps_data", "zones": "organisation"},
            "255a3bca": {"budgets": "limits", "teams": "guarded", "put_back": "keeps_data", "zones": "organisation"},
        })
        self.assertEqual((BUDGETS_API, TEAMS_API, INTENT), (
            "crates/lys-identity-server/src/budgets_api.rs",
            "crates/lys-identity-server/src/teams_api.rs",
            "crates/lys/src/identity/upgrade/intent.rs"))

    def test_a_release_that_refuses_named_zones_takes_new_limits_without_them(self):
        self.assertEqual(zone_model('refusal: "BudgetZoneRefused",'), "organisation")
        self.assertEqual(zone_model("fn validate() {}"), "explicit")
        limits = [{"unit": "tokens", "period": "week", "zone": "UTC"}, {"unit": "context_percent"}]
        self.assertEqual(zoned(limits, "explicit"), limits)
        self.assertEqual(zoned(limits, "organisation"),
                         [{"unit": "tokens", "period": "week"}, {"unit": "context_percent"}])
        self.assertEqual(limits[0]["zone"], "UTC", "the shared limits are never changed")
        with self.assertRaisesRegex(RuntimeError, "no limits"):
            zoned(limits, "other")

    def test_a_record_that_knows_the_kept_data_step_names_a_release_that_puts_data_back(self):
        keeps = "pub enum Step {\n    /// Stopped.\n    Stopped,\n    DataKept,\n    Started,\n}\n"
        self.assertEqual(put_back_model(keeps), "keeps_data")
        self.assertEqual(put_back_model(keeps.replace("    DataKept,\n", "")), "no_data")
        self.assertEqual(put_back_model(keeps.replace("    DataKept,\n", "    /// DataKept\n")),
                         "no_data")
        with self.assertRaisesRegex(RuntimeError, "no enum Step"):
            put_back_model("pub struct Intent {}\n")


class RequestBodyTests(unittest.TestCase):
    def test_a_periodic_limit_keeps_its_zone_in_the_per_measure_period(self):
        limit = {"unit": "tokens", "amount": 100, "period": "week", "zone": "UTC", "act": "stop"}
        self.assertEqual(per_measure_body(limit, 1), {
            "version": 1, "measure": "tokens", "limit": 100,
            "period": {"length": "week", "zone": "UTC"}, "act": "stop"})

    def test_a_level_has_no_period_in_either_shape(self):
        limit = {"unit": "context_percent", "amount": 60, "period": None, "act": "tell"}
        self.assertEqual(per_measure_body(limit, 0), {
            "version": 0, "measure": "context_percent", "limit": 60, "period": None, "act": "tell"})
        self.assertEqual(limits_body([limit], 0),
                         {"version": 0, "limits": [limit], "warn_at": None})

    def test_a_zone_without_a_period_or_a_period_without_a_zone_is_refused(self):
        for limit in ({"unit": "tokens", "amount": 1, "period": "day", "act": "stop"},
                      {"unit": "context_percent", "amount": 1, "period": None, "zone": "UTC",
                       "act": "tell"}):
            with self.subTest(limit=limit), self.assertRaisesRegex(RuntimeError, "zone"):
                per_measure_body(limit, 0)

    def test_the_collection_body_copies_each_limit(self):
        limit = {"unit": "tokens", "amount": 1, "period": "day", "act": "stop"}
        body = limits_body([limit], 3)
        body["limits"][0]["amount"] = 2
        self.assertEqual(limit["amount"], 1)


if __name__ == "__main__":
    unittest.main()
