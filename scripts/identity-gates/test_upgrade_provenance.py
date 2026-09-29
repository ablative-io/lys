"""The proof must reject changed signed bytes, actor, version, key and recorded digest."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from upgrade_provenance import seed, verify


class Reader:
    def __init__(self):
        self.calls = []
        self.vector = {"leaf": 3, "message": "010203", "actor": {"authentication": "service_account_bearer", "service_account": "op-fixture"}, "public_key": "ab", "sha256": hashlib.sha256(bytes.fromhex("010203")).hexdigest()}
        self.answer = {"message": "010203", "receipt": {"version": 2, "actor": copy.deepcopy(self.vector["actor"])}}
        self.key = "ab"

    def ask(self, method, path):
        self.calls.append((method, path))
        return {"ed25519": self.key} if path == "/service-key" else self.answer


class Provenance(unittest.TestCase):
    def test_seed_records_the_selected_old_binary_not_a_fixed_baseline(self):
        reader = Reader()
        class ImportReader:
            def ask(self, method, path, body=None, **options):
                if path == "/identity/import":
                    receipt = dict(reader.answer["receipt"], log={"index": reader.vector["leaf"]})
                    return {"completed": [{"result": {"receipt": receipt}}]}
                return reader.ask(method, path)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            credential = root / "credential"
            credential.write_text("fixture-only")
            baseline = "8c064b62a0c77f0874c203189a2ed3238b7ba57a"
            vector = seed(ImportReader(), root, {"import_credential_file": str(credential)}, root, baseline)
            self.assertEqual(vector["source_commit"], baseline)
            self.assertEqual(json.loads((root / "old-service-account-vector.json").read_text()), vector)

    def test_reads_the_exact_leaf_and_service_key(self):
        reader = Reader()
        verify(reader, reader.vector)
        self.assertEqual(reader.calls, [("GET", "/receipts/3"), ("GET", "/service-key")])

    def test_rejects_each_changed_axis(self):
        cases = [
            lambda reader: reader.answer.update(message="040506"),
            lambda reader: reader.answer["receipt"].update(version=1),
            lambda reader: reader.answer["receipt"]["actor"].update(authentication="operator"),
            lambda reader: reader.answer["receipt"]["actor"].update(service_account="op-other"),
            lambda reader: setattr(reader, "key", "cd"),
            lambda reader: reader.vector.update(sha256="wrong"),
        ]
        self.assertEqual(len(cases), 6)
        for mutate in cases:
            reader = Reader()
            mutate(reader)
            with self.assertRaises(RuntimeError):
                verify(reader, reader.vector)


if __name__ == "__main__":
    unittest.main()
