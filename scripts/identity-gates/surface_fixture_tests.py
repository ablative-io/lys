"""A failed builder cannot publish; concurrent consumers share one complete build."""

from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import tempfile
from threading import Event
import unittest

from surface_fixture import publish


def build(stage):
    for relative in [
        "surface/identity/dist/index.html",
        "surface/identity/node_modules/vite/bin/vite.js",
    ]:
        path = stage / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("fixture")


class FixtureTests(unittest.TestCase):
    def test_failure_publishes_nothing_and_retry_builds(self):
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary)

            def failed(stage):
                build(stage)
                raise RuntimeError("builder failed after writing output")

            with self.assertRaisesRegex(RuntimeError, "builder failed"):
                publish(scratch, {"commit": "one"}, failed)
            self.assertEqual(list((scratch / "identity-surface").glob("*/fixture.json")), [])
            self.assertEqual(len(list((scratch / "identity-surface").iterdir())), 1)
            ready = publish(scratch, {"commit": "one"}, build)
            self.assertTrue((ready / "fixture.json").is_file())

    def test_concurrent_consumers_share_one_complete_build(self):
        started, release, entered = Event(), Event(), Event()
        calls = []
        with tempfile.TemporaryDirectory() as temporary, ThreadPoolExecutor(2) as pool:
            scratch = Path(temporary)

            def held(stage):
                calls.append(stage)
                started.set()
                release.wait()
                build(stage)

            def second():
                entered.set()
                return publish(scratch, {"commit": "one"}, held)

            first = pool.submit(publish, scratch, {"commit": "one"}, held)
            started.wait()
            other = pool.submit(second)
            entered.wait()
            release.set()
            self.assertEqual(first.result(), other.result())
            self.assertEqual(len(calls), 1)
            self.assertTrue((other.result() / "fixture.json").is_file())

    def test_changed_inputs_build_separately_and_damage_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary)
            first = publish(scratch, {"commit": "one"}, build)
            second = publish(scratch, {"commit": "two"}, build)
            self.assertNotEqual(first, second)
            (first / "surface/identity/dist/index.html").unlink()
            with self.assertRaisesRegex(RuntimeError, "surface_fixture_incomplete"):
                publish(scratch, {"commit": "one"}, build)


if __name__ == "__main__":
    unittest.main()
