"""Crash only the disposable service after commit, then prove migration occurs once."""

import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess

from upgrade_legacy import verify
from upgrade_window import legacy_files, unchanged


def crash(root):
    """Wait on the installer's real exit lock, never on a timer or PID poll."""
    if (root / "install/upgrade.json").exists():
        raise RuntimeError("post-commit crash proof still has an upgrade intent")
    pid = int((root / "run/identity.pid").read_text().strip())
    command = subprocess.check_output(["ps", "-p", str(pid), "-o", "command="], text=True)
    if str(root / "identity.json") not in command or "lys-identity-server" not in command:
        raise RuntimeError(f"refuse to crash {pid}: not this fixture's identity service")
    with (root / "run/identity.exit").open("rb") as watch:
        os.kill(pid, signal.SIGKILL)
        fcntl.flock(watch, fcntl.LOCK_SH)
        fcntl.flock(watch, fcntl.LOCK_UN)


def team_rows(lines, fixture, teams):
    """The rows the first start after commit owes: an unguarded release's two holds
    and one check, and nothing for a guarded release, which checked at its own start."""
    model = fixture["release"]["teams"]
    if model == "guarded":
        if lines:
            raise RuntimeError(f"restart wrote team rows in {teams} for a release with no legacy membership")
        return {"new_hold_rows": 0, "new_check_rows": 0}
    if model != "unguarded":
        raise RuntimeError(f"no restart proof for the {model} team model")
    held = [line for line in lines if line.get("line") == "held"]
    checked = [line for line in lines if line.get("line") == "checked"]
    if (len(lines) != 3 or len(checked) != 1 or len(held) != 2
            or {line["member"] for line in held} != set(fixture["foreign"])
            or any(line["team"] != fixture["team"] for line in held)):
        raise RuntimeError(f"restart did not persist exactly the two holds and one check in {teams}")
    return {"new_hold_rows": 2, "new_check_rows": 1}


def budget_family(before, after, root, budgets, model):
    """Old budget leaves never change. A per-measure release's store is sealed again in
    the current format; a limits release's store is already current and stays byte-equal."""
    def family(files, prefix):
        return {key: value for key, value in files.items() if key.startswith(prefix)}
    leaves = str((budgets / "leaves").relative_to(root)) + "/"
    unchanged(family(before, leaves), family(after, leaves))
    whole = str(budgets.relative_to(root)) + "/"
    if model == "per_measure":
        if family(before, whole) == family(after, whole):
            raise RuntimeError(f"restart did not persist the v2 budget snapshot in {budgets}")
    elif model == "limits":
        if family(before, whole) != family(after, whole):
            raise RuntimeError(f"restart rewrote the already current budget store in {budgets}")
    else:
        raise RuntimeError(f"no restart proof for the {model} budget model")


def settle(root, evidence, driver, env, browser, fixture, run):
    config = json.loads((root / "identity.json").read_text())
    before = legacy_files(root, config)
    teams = Path(config["teams_dir"])
    old_leaves = {str(path.relative_to(root)) for path in (teams / "leaves").iterdir()}
    crash(root)
    unchanged(before, legacy_files(root, config))
    run([str(driver), "--root", str(root), "restart"], evidence / "restart-after-clear.log", env)
    verify(browser, fixture)
    after = legacy_files(root, config)
    added = [path for path in (teams / "leaves").iterdir()
             if str(path.relative_to(root)) not in old_leaves]
    lines = [json.loads(path.read_text()) for path in added]
    rows = team_rows(lines, fixture, teams)
    for name in old_leaves:
        if before.get(name) != after.get(name):
            raise RuntimeError(f"restart rewrote old team leaf {name}")
    budget_family(before, after, root, Path(config["budgets_dir"]), fixture["release"]["budgets"])
    crash(root)
    run([str(driver), "--root", str(root), "restart"], evidence / "restart-idempotent.log", env)
    verify(browser, fixture)
    unchanged(after, legacy_files(root, config))
    receipt = {"passed": True, **rows,
               "old_leaf_bytes_preserved": True, "second_restart_byte_equal": True}
    (evidence / "migration-restart.json").write_text(json.dumps(receipt, indent=2))
    return receipt
