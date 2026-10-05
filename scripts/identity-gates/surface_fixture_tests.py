"""A failed builder cannot publish; concurrent consumers share one complete build."""

from concurrent.futures import ThreadPoolExecutor
import io
from pathlib import Path
import tarfile
import tempfile
from threading import Event
import unittest

from surface_fixture import extract, publish


def archive_of(path, member, data=b""):
    with tarfile.open(path, "w") as archive:
        member.size = len(data)
        archive.addfile(member, io.BytesIO(data))
    return path


def link(name, linkname):
    member = tarfile.TarInfo(name)
    member.type = tarfile.SYMTYPE
    member.linkname = linkname
    return member


def build(stage):
    for relative in [
        "surface/identity/dist/index.html",
        "surface/identity/node_modules/vite/bin/vite.js",
    ]:
        path = stage / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("fixture")


class FixtureTests(unittest.TestCase):
    def test_extraction_refuses_members_outside_the_stage_on_every_python(self):
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary)
            stage = scratch / "stage"
            stage.mkdir()
            good = archive_of(scratch / "good.tar", tarfile.TarInfo("surface/index.html"), b"fixture")
            extract(good, stage)
            self.assertEqual((stage / "surface/index.html").read_text(), "fixture")
            setuid = tarfile.TarInfo("surface/tool")
            setuid.mode = 0o4755
            extract(archive_of(scratch / "setuid.tar", setuid, b"x"), stage)
            self.assertEqual((stage / "surface/tool").stat().st_mode & 0o7000, 0)
            extract(archive_of(scratch / "absolute.tar", tarfile.TarInfo("/absolute"), b"x"), stage)
            self.assertEqual((stage / "absolute").read_text(), "x")
            for name, member in [
                ("escape.tar", tarfile.TarInfo("../escape")),
                ("link.tar", link("surface/out", "../../outside")),
                ("rooted.tar", link("surface/rooted", "/etc")),
            ]:
                with self.assertRaisesRegex(RuntimeError, "surface_fixture_unsafe_member"):
                    extract(archive_of(scratch / name, member, b"x"), stage)
            self.assertFalse((scratch / "escape").exists())
            self.assertFalse((stage / "surface/out").is_symlink())
            self.assertFalse((stage / "surface/rooted").is_symlink())

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
