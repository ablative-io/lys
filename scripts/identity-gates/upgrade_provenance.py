"""Capture a real old-binary bearer receipt and require its signed bytes through rollback and upgrade."""

import hashlib
import json
from pathlib import Path


def seed(browser, root, config, evidence, old_commit):
    """The real old import endpoint signs this record; this code never encodes or signs it."""
    credential = Path(config["import_credential_file"]).resolve()
    if not credential.is_relative_to(root) or not credential.is_file():
        raise RuntimeError("old loader credential is absent or outside the disposable root")
    imported = browser.ask("POST", "/identity/import",
                           {"agents": [{"display_name": "Old bearer provenance fixture"}]},
                           bearer=credential.read_text().strip())
    entries = imported["completed"]
    if len(entries) != 1:
        raise RuntimeError("old bearer seed must create exactly one agent record")
    receipt = entries[0]["result"]["receipt"]
    actor = receipt["actor"]
    if receipt["version"] != 2 or actor.get("authentication") != "service_account_bearer":
        raise RuntimeError("old endpoint did not record v2 service-account authentication")
    index = receipt["log"]["index"]
    signed = browser.ask("GET", f"/receipts/{index}")
    vector = {"source_commit": old_commit, "leaf": index, "actor": actor,
              "public_key": browser.ask("GET", "/service-key")["ed25519"],
              "message": signed["message"]}
    vector["sha256"] = hashlib.sha256(bytes.fromhex(vector["message"])).hexdigest()
    (evidence / "old-service-account-vector.json").write_text(json.dumps(vector, indent=2))
    verify(browser, vector)
    return vector


def verify(browser, vector):
    """A new reader and the old reader must return the original signed message and actor."""
    answer = browser.ask("GET", f"/receipts/{vector['leaf']}")
    if answer["message"] != vector["message"]:
        raise RuntimeError("old service-account signed bytes changed")
    if answer["receipt"]["version"] != 2 or answer["receipt"]["actor"] != vector["actor"]:
        raise RuntimeError("old service-account identity or version changed")
    if browser.ask("GET", "/service-key")["ed25519"] != vector["public_key"]:
        raise RuntimeError("service key changed across the upgrade")
    if hashlib.sha256(bytes.fromhex(answer["message"])).hexdigest() != vector["sha256"]:
        raise RuntimeError("old service-account receipt digest changed")
