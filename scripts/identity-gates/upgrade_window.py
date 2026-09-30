"""Verify held authority and unchanged legacy bytes before Started is committed."""

import argparse
import hashlib
import json
from pathlib import Path

from upgrade_fixture import Browser, operation
from upgrade_provenance import verify as verify_provenance
from upgrade_legacy import verify

MARKER = "lys-disposable-upgrade-proof/v1"


def family_files(root, config):
    """Hash every file, including snapshots and pins, in both migration families."""
    result = {}
    for key in ("teams_dir", "budgets_dir"):
        directory = Path(config[key]).resolve()
        if not directory.is_relative_to(root) or not directory.is_dir():
            raise RuntimeError(f"fixture {key} is not a directory under {root}: {directory}")
        found = False
        for path in sorted(directory.rglob("*")):
            if path.is_symlink():
                raise RuntimeError(f"fixture family contains a symbolic link: {path}")
            if path.is_file():
                found = True
                result[str(path.relative_to(root))] = hashlib.sha256(path.read_bytes()).hexdigest()
        if not found:
            raise RuntimeError(f"empty fixture family cannot prove rollback: {directory}")
    return result


def profile_file(root, config):
    """An ordinary old profile must keep its exact file representation."""
    path = Path(config["provisioning_file"]).resolve()
    if not path.is_relative_to(root) or not path.is_file():
        raise RuntimeError(f"fixture profile file is absent or leaves its root: {path}")
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()}


def legacy_files(root, config):
    return {**family_files(root, config), **profile_file(root, config)}


def unchanged(before, after):
    for path in sorted(before.keys() | after.keys()):
        if before.get(path) != after.get(path):
            raise RuntimeError(f"reversible upgrade changed legacy record {path}")


def pending(root):
    path = root / "install/upgrade.json"
    intent = json.loads(path.read_text())
    if "started" in intent["steps"] or "binaries_placed" not in intent["steps"]:
        raise RuntimeError(f"{path} is not the new build's reversible window")
    return intent


def refused_writes(fixture, provisioning):
    """Exercise every new-format writer while the actual old-reader rollback is possible."""
    team = f"/teams/{fixture['team']}/members/{fixture['foreign'][0]}/confirm"
    budgets = f"/budgets/person/{fixture['person']}/confirm"
    tokens = next(value for value in fixture["budgets_before"]["budgets"]
                  if value["measure"] == "tokens")
    profile = provisioning["profile"]
    path = f"/agents/{provisioning['agent']}/provisioning"
    change = {name: profile[name] for name in (
        "model_access", "tools", "skills", "mcp_servers", "instructions", "note")}
    change.update(operation=operation(), from_version=profile["version"])
    return [
        (team, {"operation": operation()}, "TeamsUnavailable"),
        (budgets, {"measure": "tokens", "version": tokens["version"]}, "BudgetsUnavailable"),
        (path, change, "ProvisioningUnavailable"),
        (f"{path}/{profile['version']}/review", {"operation": operation()}, "ProvisioningUnavailable"),
        ("/skills", {"name": "upgrade-fixture-skill", "text": "# Held during upgrade\n"},
         "ProvisioningUnavailable"),
    ]


def check(root):
    if (root / ".upgrade-proof").read_text() != MARKER:
        raise RuntimeError(f"{root} is not a disposable upgrade fixture")
    context = json.loads((root / ".upgrade-window.json").read_text())
    config = json.loads((root / "identity.json").read_text())
    pending(root)
    browser = Browser(context["port"])
    browser.cookie = context["cookie"]
    verify_provenance(browser, context["provenance"])
    backup = root / "config.previous/identity.json"
    if hashlib.sha256(backup.read_bytes()).hexdigest() != context["original_config_sha256"]:
        raise RuntimeError("reversible window backup differs from the old original config")
    fixture = context["legacy"]
    counts = verify(browser, fixture)
    unchanged(context["files"], legacy_files(root, config))
    writes = refused_writes(fixture, context["provisioning"])
    for path, body, kind in writes:
        refusal = browser.ask("POST", path, body, expected_status=503)
        if refusal.get("refusal") != kind or "upgrade_pending" not in refusal.get("reason", ""):
            raise RuntimeError(f"{path} did not refuse {kind} naming upgrade_pending")
        unchanged(context["files"], legacy_files(root, config))
    # A refused confirmation must also leave every legacy byte alone.
    unchanged(context["files"], legacy_files(root, config))
    verify(browser, fixture)
    pending(root)
    receipt = {"passed": True, "legacy_files": len(context["files"]),
               "writes_refused": len(writes), "refused_routes": [path for path, body, kind in writes],
               **counts}
    (root.parent / "evidence/window.json").write_text(json.dumps(receipt, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=lambda value: Path(value).resolve(), required=True)
    check(parser.parse_args().root)


if __name__ == "__main__":
    main()
