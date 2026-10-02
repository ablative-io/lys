"""Old setup projects Active; a freshly registered grant holder must be activated by its API."""

import tempfile
import unittest
from pathlib import Path

from upgrade_fixture import populate

PER_MEASURE = {"budgets": "per_measure", "teams": "unguarded"}
LIMITS = {"budgets": "limits", "teams": "guarded"}


class OldApi:
    def __init__(self, state="active", responsible="owner"):
        self.state = state
        self.responsible = responsible
        self.calls = []
        self.active = False

    def ask(self, method, path, body=None):
        self.calls.append((method, path, body))
        if path == "/me":
            return {"person": {"id": "owner", "state": self.state}}
        if path == "/people":
            return {"person": "member"}
        if path == "/identities/member/transitions":
            if body["transition"] != "activate" or not body["operation"] or not body["reason"]:
                raise RuntimeError("invalid transition")
            self.active = True
        if path == "/identities/member":
            return {"id": "member", "state": "active" if self.active else "registered"}
        if path == "/agents":
            return {"agent": "agent", "responsible": self.responsible}
        if path == "/grants/roots":
            if not self.active:
                raise RuntimeError("grant holder is not active")
            return {"grant": "grant"}
        return {}


class SeedTests(unittest.TestCase):
    def exercise(self, browser, release=PER_MEASURE):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "setup-code").write_text("test-only-code")
            return populate(browser, root, release)

    def budget_request(self, release):
        browser = OldApi()
        self.exercise(browser, release)
        return [(method, body) for method, path, body in browser.calls
                if path == "/budgets/agent/agent"]

    def test_a_per_measure_release_is_sent_one_measure_with_its_zoned_period(self):
        self.assertEqual(self.budget_request(PER_MEASURE), [("PUT", {
            "version": 0, "measure": "tokens", "limit": 100,
            "period": {"length": "day", "zone": "Australia/Melbourne"}, "act": "tell"})])

    def test_a_limits_release_is_sent_the_same_limit_as_its_whole_collection(self):
        self.assertEqual(self.budget_request(LIMITS), [("PUT", {
            "version": 0, "warn_at": None,
            "limits": [{"unit": "tokens", "amount": 100, "period": "day",
                        "zone": "Australia/Melbourne", "act": "tell"}]})])

    def test_an_unknown_budget_model_is_refused_before_any_budget_is_sent(self):
        browser = OldApi()
        with self.assertRaisesRegex(RuntimeError, "budget model"):
            self.exercise(browser, {"budgets": "unknown", "teams": "guarded"})
        self.assertNotIn("/budgets/agent/agent", [path for method, path, body in browser.calls])

    def test_supported_setup_and_holder_activation(self):
        browser = OldApi()
        self.assertEqual(self.exercise(browser)["person"], "member")
        paths = [path for method, path, body in browser.calls]
        self.assertLess(paths.index("/identities/member/transitions"), paths.index("/grants/roots"))

    def test_registered_setup_is_not_misreported_as_a_valid_fixture(self):
        with self.assertRaisesRegex(RuntimeError, "Active administrator"):
            self.exercise(OldApi("registered"))

    def test_an_agent_registered_under_another_person_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, "wrong responsible person"):
            self.exercise(OldApi(responsible="member"))


if __name__ == "__main__":
    unittest.main()
