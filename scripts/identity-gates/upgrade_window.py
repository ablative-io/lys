"""Verify held authority and unchanged legacy bytes before Started is committed."""

import argparse
import hashlib
import json
from pathlib import Path
import secrets

from upgrade_fixture import Browser, operation
from upgrade_provenance import verify as verify_provenance
from upgrade_legacy import verify

MARKER = "lys-disposable-upgrade-proof/v1"
# The name a development install's operator token file has under state/.
OPERATOR_TOKEN_NAME = "operator-token"


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
    held = fixture["budgets_before"]
    if fixture["release"]["budgets"] == "per_measure":
        version = next(value for value in held["budgets"] if value["measure"] == "tokens")["version"]
    elif fixture["release"]["budgets"] == "limits":
        # A limits release keeps one version for the holder's whole collection.
        version = held["version"]
    else:
        raise RuntimeError(f"no budget confirmation for the {fixture['release']['budgets']} model")
    profile = provisioning["profile"]
    path = f"/agents/{provisioning['agent']}/provisioning"
    change = {name: profile[name] for name in (
        "model_access", "tools", "skills", "mcp_servers", "instructions", "note")}
    change.update(operation=operation(), from_version=profile["version"])
    return [
        (team, {"operation": operation()}, "TeamsUnavailable"),
        (budgets, {"measure": "tokens", "version": version}, "BudgetsUnavailable"),
        (path, change, "ProvisioningUnavailable"),
        (f"{path}/{profile['version']}/review", {"operation": operation()}, "ProvisioningUnavailable"),
        ("/skills", {"name": "upgrade-fixture-skill", "text": "# Held during upgrade\n"},
         "ProvisioningUnavailable"),
    ]



def no_operator_token(root, config):
    """A service install keeps no standing operator token: its configuration names none, and
    no operator token file exists anywhere under the install."""
    if "operator_token_file" not in config or config["operator_token_file"] is not None:
        raise RuntimeError("the installed configuration must name operator_token_file as null")
    held = [str(path) for path in root.rglob(OPERATOR_TOKEN_NAME)]
    if held:
        raise RuntimeError(f"a service install holds an operator token file: {held}")


def refuse_operator(browser, root, config):
    """A presented operator token cannot emit a v1 record while old-reader rollback is possible.
    With a token file the install's own token is presented; a service install names none, so its
    absence is proved and a token of the same shape is presented instead. Answers the token
    file's digest, or None when the install keeps none."""
    named = config.get("operator_token_file")
    if named is None:
        no_operator_token(root, config)
        token = None
        presented = secrets.token_hex(32)
    else:
        token = Path(named).resolve()
        if not token.is_relative_to(root):
            raise RuntimeError("operator proof paths leave the disposable root")
        presented = token.read_text().strip()
    directory = Path(config["log_dir"]).resolve()
    if not directory.is_relative_to(root):
        raise RuntimeError("operator proof paths leave the disposable root")
    def leaves():
        return {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                for path in (directory / "leaves").iterdir() if path.is_file()}
    before = leaves()
    if not before:
        raise RuntimeError("operator proof requires a nonempty identity log")
    answer = browser.ask("POST", "/agents", {
        "operation": operation(), "display_name": "Must not be admitted during rollback",
    }, expected_status=401, operator=presented)
    if answer.get("refusal") != "OperatorRefused" or "reversible" not in answer.get("reason", ""):
        raise RuntimeError("operator write did not name the reversible upgrade refusal")
    unchanged(before, leaves())
    return None if token is None else hashlib.sha256(token.read_bytes()).hexdigest()


def check(root):
    if (root / ".upgrade-proof").read_text() != MARKER:
        raise RuntimeError(f"{root} is not a disposable upgrade fixture")
    context = json.loads((root / ".upgrade-window.json").read_text())
    config = json.loads((root / "identity.json").read_text())
    pending(root)
    browser = Browser(context["port"])
    browser.cookie = context["cookie"]
    verify_provenance(browser, context["provenance"])
    operator_digest = refuse_operator(browser, root, config)
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
    receipt = {"passed": True, "operator_token_sha256": operator_digest, "legacy_files": len(context["files"]),
               "operator_write_refused": True, "writes_refused": len(writes), "refused_routes": [path for path, body, kind in writes],
               **counts}
    (root.parent / "evidence/window.json").write_text(json.dumps(receipt, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=lambda value: Path(value).resolve(), required=True)
    check(parser.parse_args().root)


if __name__ == "__main__":
    main()
