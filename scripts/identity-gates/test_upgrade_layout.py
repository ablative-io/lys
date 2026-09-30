"""Installer declarations decide binary readback; absent or changed path producers refuse preparation."""

from pathlib import Path
import tempfile
import unittest

from upgrade_layout import binary_names, harness_inventory, harness_paths, inventory, resolve_layout


class LayoutTests(unittest.TestCase):
    def test_binary_list_is_read_from_the_installer_not_the_artifact_package(self):
        self.assertEqual(binary_names('pub const BINARIES: [&str; 2] = ["lys-secrets", "lys-identity-server"];'),
                         ["lys-secrets", "lys-identity-server"])
        for bad in ('', 'pub const BINARIES: [&str; 3] = ["lys-secrets", "lys-identity-server"];'):
            with self.subTest(bad=bad), self.assertRaisesRegex(RuntimeError, "BINARIES"):
                binary_names(bad)

    def test_nested_paths_resolve_and_missing_getters_refuse(self):
        text = ('pub fn run_dir(&self) -> PathBuf { self.root.join("run") }\n'
                'pub fn runner_socket(&self) -> PathBuf { self.run_dir().join("runner.sock") }')
        self.assertEqual(resolve_layout(text)["runner_socket"], Path("run/runner.sock"))
        with self.assertRaisesRegex(RuntimeError, "missing.*run_dir"):
            resolve_layout(text.split('\n')[1])

    def test_actual_source_inventory_refuses_a_changed_log_path_before_install(self):
        root = Path(__file__).resolve().parents[2]
        def read(name):
            return (root / name).read_text()
        result = inventory(read, "test", candidate=True)
        self.assertEqual(result["installed_binaries"], ["lys-secrets", "lys-identity-server"])
        def changed(name):
            return read(name).replace('join("identity.log")', 'join("moved.log")')
        with self.assertRaisesRegex(RuntimeError, "upgrade.rs.*identity.log"):
            inventory(changed, "test", candidate=True)

    def test_missing_recursive_verifier_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "upgrade_live.py").write_text('from upgrade_missing import check\n')
            with self.assertRaisesRegex(RuntimeError, "upgrade_missing.py"):
                harness_inventory(root)

    def test_a_new_unproduced_fixed_root_path_cannot_disappear_from_preparation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "upgrade_live.py"
            path.write_text('wrong = root / "bin/lys"\n')
            with self.assertRaisesRegex(RuntimeError, "no installer.*bin/lys"):
                harness_paths(harness_inventory(root), {"old": {"paths": []}})


if __name__ == "__main__":
    unittest.main()
