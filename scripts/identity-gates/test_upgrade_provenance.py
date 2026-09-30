"""The proof must reject changed signed bytes, actor, version, key and recorded digest."""

import copy
import hashlib
import unittest

from upgrade_provenance import verify


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
