"""Directory history and the old owner's snapshot must survive a real upgrade."""

import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock

import upgrade_fixture
import upgrade_live


def cbor(value):
    if value is None:
        return b"\xf6"
    if isinstance(value, int):
        major, count, content = 0, value, b""
    elif isinstance(value, bytes):
        major, count, content = 2, len(value), value
    elif isinstance(value, str):
        content = value.encode()
        major, count = 3, len(content)
    else:
        major, count, content = 4, len(value), b"".join(map(cbor, value))
    if count < 24:
        return bytes([major * 32 + count]) + content
    if count < 256:
        return bytes([major * 32 + 24, count]) + content
    return bytes([major * 32 + 25]) + count.to_bytes(2, "big") + content


def frame(value):
    return len(value).to_bytes(8, "big") + value


class DirectoryUpgradeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.directory = self.root / "data/directory"
        self.leaves = self.directory / "leaves"
        self.leaves.mkdir(parents=True)
        self.config = {"log_dir": str(self.directory), "log_origin": "fixture"}
        self.admin = {
            "person": {"id": "person-" + "11" * 16, "state": "active"},
            "signed_in": {"issuer": "fixture", "subject": "owner"},
            "sign_in_identities": [{"issuer": "fixture", "subject": "owner"}],
        }
        for index in range(3):
            (self.leaves / f"{index:020}").write_bytes(bytes([index, 0, 255]))

    def inventory(self):
        return upgrade_live.directory_leaves(self.root, self.config)

    def snapshot(self, version=3, folded=1, wrap=1, owner=None, origin=b"fixture"):
        administrator = [
            [1, bytes.fromhex("11" * 16)],
            "Owner",
            2,
            None,
            [["fixture", "owner"]],
            ["fixture", "owner"],
            [0],
            None,
        ]
        parts, fields = upgrade_live.OWNER_SHAPES.get(version, (3, 8))
        administrator = administrator[:fields]
        if owner is None:
            projection = [[administrator]] + [[] for _ in range(parts - 1)]
            owner = cbor([version, folded, projection])
        state = cbor([wrap, [b""], owner])
        root = hashlib.sha256(b"\x00" + bytes([0, 0, 255])).digest()
        body = (
            frame(b"lys/log-snapshot/v1")
            + frame(b"lys/identity-directory/v1")
            + frame(origin)
            + (1).to_bytes(8, "big")
            + root
            + frame(root)
            + frame(state)
        )
        sealed = frame(body) + frame(b"s" * 64)
        (self.directory / "snapshot.bin").write_bytes(sealed)
        return sealed

    def old_snapshot(self):
        return upgrade_live.old_directory_snapshot(
            self.root, self.config, self.inventory(), self.admin
        )

    def test_inventory_retains_every_byte_in_index_order(self):
        held = self.inventory()
        self.assertEqual(list(held), [f"data/directory/leaves/{index:020}" for index in range(3)])
        self.assertEqual(list(held.values()), [bytes([index, 0, 255]).hex() for index in range(3)])

    def test_rewritten_leaf_is_rejected(self):
        before = self.inventory()
        (self.leaves / f"{1:020}").write_bytes(b"replacement")
        with self.assertRaisesRegex(RuntimeError, "changed old directory leaf"):
            upgrade_live.preserve_directory(before, self.inventory())

    def test_removed_leaf_is_rejected(self):
        before = self.inventory()
        (self.leaves / f"{2:020}").unlink()
        with self.assertRaisesRegex(RuntimeError, "missing old directory leaf"):
            upgrade_live.preserve_directory(before, self.inventory())

    def test_only_contiguous_new_tail_is_allowed(self):
        before = self.inventory()
        (self.leaves / f"{3:020}").write_bytes(b"new event")
        self.assertEqual(
            upgrade_live.preserve_directory(before, self.inventory()),
            {"before": 3, "after": 4, "preserved": 3, "appended": 1},
        )

    def test_gap_is_rejected(self):
        (self.leaves / f"{1:020}").unlink()
        with self.assertRaisesRegex(RuntimeError, "contiguous"):
            self.inventory()

    def test_empty_directory_is_rejected(self):
        for path in self.leaves.iterdir():
            path.unlink()
        with self.assertRaisesRegex(RuntimeError, "empty directory"):
            self.inventory()

    def test_invalid_leaf_name_is_rejected(self):
        (self.leaves / "unexpected").write_bytes(b"history")
        with self.assertRaisesRegex(RuntimeError, "leaf name"):
            self.inventory()

    def test_hidden_uncommitted_temporary_file_is_not_history(self):
        (self.leaves / ".1-00000000000000000003-0.tmp").write_bytes(b"uncommitted")
        self.assertEqual(len(self.inventory()), 3)

    def test_symlinked_leaf_is_rejected(self):
        target = self.root / "outside"
        target.write_bytes(b"outside")
        (self.leaves / f"{2:020}").unlink()
        (self.leaves / f"{2:020}").symlink_to(target)
        with self.assertRaisesRegex(RuntimeError, "symlink"):
            self.inventory()

    def test_directory_outside_root_is_rejected(self):
        self.config["log_dir"] = str(self.root.parent)
        with self.assertRaisesRegex(RuntimeError, "leaves its root"):
            self.inventory()

    def test_symlinked_directory_inside_root_is_rejected(self):
        alias = self.root / "alias"
        alias.symlink_to(self.directory, target_is_directory=True)
        self.config["log_dir"] = str(alias)
        with self.assertRaisesRegex(RuntimeError, "symlink"):
            self.inventory()

    def test_snapshot_reads_nested_owner_version_and_administrator(self):
        sealed = self.snapshot()
        proof = self.old_snapshot()
        self.assertEqual(proof["owner_version"], 3)
        self.assertEqual(proof["folded"], 1)
        self.assertEqual(proof["administrator"], self.admin["person"]["id"])
        self.assertEqual(proof["sha256"], hashlib.sha256(sealed).hexdigest())

    def test_every_released_owner_version_is_read_by_its_own_shape(self):
        for version in (2, 3, 4, 5):
            with self.subTest(version=version):
                self.snapshot(version=version)
                proof = self.old_snapshot()
                self.assertEqual(proof["owner_version"], version)
                self.assertEqual(proof["administrator"], self.admin["person"]["id"])

    def test_an_owner_version_no_release_wrote_is_refused_by_name(self):
        self.snapshot(version=6)
        with self.assertRaisesRegex(RuntimeError, "owner version 6"):
            self.old_snapshot()

    def test_a_version_in_another_versions_shape_is_refused(self):
        administrator = [
            [1, bytes.fromhex("11" * 16)],
            "Owner",
            2,
            None,
            [["fixture", "owner"]],
            ["fixture", "owner"],
            [0],
            None,
        ]
        self.snapshot(owner=cbor([5, 1, [[administrator], [], []]]))
        with self.assertRaisesRegex(RuntimeError, "projection has the wrong shape"):
            self.old_snapshot()

    def test_wrong_wrapper_is_rejected(self):
        self.snapshot(wrap=3)
        with self.assertRaisesRegex(RuntimeError, "wrapper"):
            self.old_snapshot()

    def test_wrong_folded_count_is_rejected(self):
        self.snapshot(folded=0)
        with self.assertRaisesRegex(RuntimeError, "folded"):
            self.old_snapshot()

    def test_foreign_origin_is_rejected(self):
        self.snapshot(origin=b"another")
        with self.assertRaisesRegex(RuntimeError, "origin"):
            self.old_snapshot()

    def test_trailing_snapshot_bytes_are_rejected(self):
        sealed = self.snapshot()
        (self.directory / "snapshot.bin").write_bytes(sealed + b"trailing")
        with self.assertRaisesRegex(RuntimeError, "trailing"):
            self.old_snapshot()

    def test_noncanonical_owner_is_rejected(self):
        self.snapshot(owner=b"\x83\x18\x03\x01\x83\x80\x80\x80")
        with self.assertRaisesRegex(RuntimeError, "canonical"):
            self.old_snapshot()

    def test_snapshot_without_administrator_is_rejected(self):
        self.snapshot(owner=cbor([3, 1, [[], [], []]]))
        with self.assertRaisesRegex(RuntimeError, "administrator"):
            self.old_snapshot()

    def test_root_must_match_the_actual_historical_leaf_bytes(self):
        self.snapshot()
        (self.leaves / f"{0:020}").write_bytes(b"changed signed history")
        with self.assertRaisesRegex(RuntimeError, "root"):
            self.old_snapshot()

    def test_truncated_snapshot_is_rejected(self):
        sealed = self.snapshot()
        (self.directory / "snapshot.bin").write_bytes(sealed[:-1])
        with self.assertRaisesRegex(RuntimeError, "truncated"):
            self.old_snapshot()

    def test_short_signature_is_rejected(self):
        sealed = self.snapshot()
        reader = upgrade_live.SnapshotBytes(sealed)
        body = reader.frame()
        (self.directory / "snapshot.bin").write_bytes(frame(body) + frame(b"s" * 63))
        with self.assertRaisesRegex(RuntimeError, "signature"):
            self.old_snapshot()

    def test_snapshot_without_admin_login_binding_is_rejected(self):
        self.snapshot()
        self.admin["signed_in"]["subject"] = "different"
        with self.assertRaisesRegex(RuntimeError, "administrator"):
            self.old_snapshot()

    def test_cbor_allocation_and_recursion_are_bounded_by_the_input(self):
        for malformed in (b"\x9b" + b"\xff" * 8, b"\x81" * 34 + b"\x00", b"\x7f"):
            with self.subTest(malformed=malformed), self.assertRaises(RuntimeError):
                upgrade_live.snapshot_value(malformed)

    def test_original_active_administrator_can_write_and_read(self):
        browser = Mock()
        browser.ask.side_effect = [
            self.admin,
            {"person": "new"},
            {"id": "new", "display_name": "Created After Upgrade"},
            self.admin,
        ]
        self.assertEqual(upgrade_fixture.admitted_after_upgrade(browser, self.admin), "new")
        self.assertEqual(
            [call.args[:2] for call in browser.ask.call_args_list],
            [("GET", "/me"), ("POST", "/people"), ("GET", "/identities/new"), ("GET", "/me")],
        )

    def test_substituted_administrator_is_rejected_before_write(self):
        browser = Mock()
        browser.ask.return_value = dict(self.admin, person={"id": "different", "state": "active"})
        with self.assertRaisesRegex(RuntimeError, "administrator"):
            upgrade_fixture.admitted_after_upgrade(browser, self.admin)
        browser.ask.assert_called_once_with("GET", "/me")

    def test_inactive_old_administrator_is_rejected(self):
        browser = Mock()
        inactive = dict(self.admin, person=dict(self.admin["person"], state="retired"))
        with self.assertRaisesRegex(RuntimeError, "active"):
            upgrade_fixture.admitted_after_upgrade(browser, inactive)
        browser.ask.assert_not_called()

    def test_administrator_cannot_change_during_the_write(self):
        browser = Mock()
        changed = dict(self.admin, signed_in={"issuer": "fixture", "subject": "different"})
        browser.ask.side_effect = [
            self.admin,
            {"person": "new"},
            {"id": "new", "display_name": "Created After Upgrade"},
            changed,
        ]
        with self.assertRaisesRegex(RuntimeError, "administrator"):
            upgrade_fixture.admitted_after_upgrade(browser, self.admin)


if __name__ == "__main__":
    unittest.main()
