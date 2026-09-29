"""Old setup projects Active; a freshly registered grant holder must be activated by its API."""

import tempfile
import unittest
from pathlib import Path

from upgrade_fixture import populate


class OldApi:
    def __init__(self, state="active"):
        self.state = state
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
            return {"agent": "agent"}
        if path == "/grants/roots":
            if not self.active:
                raise RuntimeError("grant holder is not active")
            return {"grant": "grant"}
        return {}


class SeedTests(unittest.TestCase):
    def exercise(self, browser):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "setup-code").write_text("test-only-code")
            return populate(browser, root)

    def test_supported_setup_and_holder_activation(self):
        browser = OldApi()
        self.assertEqual(self.exercise(browser)["person"], "member")
        paths = [path for method, path, body in browser.calls]
        self.assertLess(paths.index("/identities/member/transitions"), paths.index("/grants/roots"))

    def test_registered_setup_is_not_misreported_as_a_valid_fixture(self):
        with self.assertRaisesRegex(RuntimeError, "Active administrator"):
            self.exercise(OldApi("registered"))


if __name__ == "__main__":
    unittest.main()
