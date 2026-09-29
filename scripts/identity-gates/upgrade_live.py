"""Upgrade a disposable real 1b568cd9 install; never operate a person's live root."""

import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys

from upgrade_fixture import Browser, admitted_after_upgrade, observe, populate, same_records
from upgrade_legacy import seed as seed_legacy, verify as verify_legacy
from upgrade_window import MARKER, legacy_files, pending, profile_file, unchanged
from upgrade_negative import exercise as exercise_negative
from upgrade_restart import settle as settle_restart
from upgrade_provenance import seed as seed_provenance, verify as verify_provenance

OLD_COMMIT = "1b568cd90578f5ed5d7d438e628b23724eef7f12"
PROGRAMS = ("lys", "lys-identity-server", "lys-secrets")


def run(command, log, env=None, expected=0):
    with log.open("wb") as output:
        completed = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, env=env)
    if completed.returncode != expected:
        raise RuntimeError(f"{command[0]} exited {completed.returncode}, expected {expected}; evidence: {log}")


def stamp(directory, expected):
    versions = {}
    for name in PROGRAMS:
        output = subprocess.check_output([str(directory / name), "--version"], text=True).strip()
        if f"({expected})" not in output or "dirty" in output:
            raise RuntimeError(f"{directory / name} must be a clean build of {expected}: {output}")
        versions[name] = {"version": output,
            "sha256": hashlib.sha256((directory / name).read_bytes()).hexdigest()}
    return versions


def app_leaves(root, config):
    directory = Path(config["grant_log_dir"]).with_name("apps")
    if not directory.is_relative_to(root):
        raise RuntimeError(f"fixture app log leaves its root: {directory}")
    leaves = {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
              for path in (directory / "leaves").iterdir()
              if re.fullmatch(r"[0-9]{20}", path.name)}
    if len(leaves) < 3:
        raise RuntimeError("old app log does not hold model, registration and approval")
    return leaves


def preserve_leaves(root, before):
    for name, digest in before.items():
        if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"upgrade changed old app leaf {name}")


def port():
    with socket.socket() as held:
        held.bind(("127.0.0.1", 0))
        return held.getsockname()[1]


def require_free(number):
    with socket.socket() as held:
        held.bind(("127.0.0.1", number))


def installation(root, old_source, project):
    layout = (old_source / "crates/lys/src/identity/install/layout.rs").read_text()
    ports = {}
    for name in ["SERVICE_PORT", "BROKER_PORT"]:
        match = re.search(rf"pub const {name}: u16 = (\d+);", layout)
        if match is None:
            raise RuntimeError(f"the old install declares no {name}")
        ports[name] = int(match.group(1))
        require_free(ports[name])
    networks = subprocess.check_output(["docker", "network", "ls", "-q"], text=True).split()
    held = []
    if networks:
        held = json.loads(subprocess.check_output(["docker", "network", "inspect", *networks]))
    used = {entry["Subnet"] for network in held
            for entry in network.get("IPAM", {}).get("Config", []) if "Subnet" in entry}
    network = next((f"172.29.{i}.0/24" for i in range(48, 255)
                    if all(not ipaddress.ip_network(f"172.29.{i}.0/24").overlaps(
                        ipaddress.ip_network(subnet)) for subnet in used
                        if ipaddress.ip_network(subnet).version == 4)), None)
    if network is None:
        raise RuntimeError("no unused fixture network in the identity test range")
    text = (old_source / "crates/lys/src/identity/install/deployment.template.toml").read_text()
    text = text.replace("{{admin_email_line}}", "")
    text = text.replace("{{service_port}}", str(ports["SERVICE_PORT"]))
    text = text.replace("{{rauthy_port}}", str(port()))
    text = text.replace('project = "lys-identity"', f'project = "{project}"')
    for old in ["55432", "58051", "58443"]:
        text = text.replace(old, str(port()))
    text = text.replace("172.29.47.", network.rsplit(".", 1)[0] + ".")
    (root / "deployment.toml").write_text(text)
    return ports["SERVICE_PORT"]


def stop_fixture(root, project, evidence):
    errors = []
    for name in ["runner.pid", "identity.pid", "secrets.pid"]:
        path = root / "run" / name
        if not path.exists():
            continue
        pid = int(path.read_text().strip())
        process = subprocess.run(["ps", "-p", str(pid), "-o", "command="],
                                 capture_output=True, text=True)
        if process.returncode:
            continue
        if str(root) not in process.stdout:
            errors.append(f"refuse to stop {pid}: it does not name fixture root {root}")
            continue
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    compose = root / "deploy/compose.yaml"
    if compose.exists():
        try:
            run(["docker", "compose", "-f", str(compose), "--env-file",
                 str(root / "state/compose.env"), "-p", project, "--profile", "bundled-db",
                 "down", "-v", "--remove-orphans"], evidence / "teardown.log")
        except (OSError, RuntimeError) as error:
            errors.append(str(error))
    if errors:
        raise RuntimeError("; ".join(errors))


def exercise(args):
    os.umask(0o077)
    args.work.mkdir(mode=0o700)
    evidence = args.work / "evidence"
    evidence.mkdir()
    root = args.work / "install"
    root.mkdir()
    (root / ".upgrade-proof").write_text(MARKER)
    old_head = subprocess.check_output(
        ["git", "-C", str(args.old_source), "rev-parse", "HEAD"], text=True).strip()
    if old_head != OLD_COMMIT:
        raise RuntimeError(f"old source is {old_head}, expected {OLD_COMMIT}")
    dirty = subprocess.check_output(
        ["git", "-C", str(args.old_source), "status", "--porcelain"], text=True)
    if dirty:
        raise RuntimeError("old source must be clean, including its deployment template")
    surfaces = {}
    for name, path, commit in [("old", args.old_surface, OLD_COMMIT),
                               ("candidate", args.candidate_surface, args.candidate_commit)]:
        manifest_path = path / "surface-manifest.json"
        manifest = json.loads(manifest_path.read_text())
        if manifest["commit"] != commit:
            raise RuntimeError(f"{name} surface does not name {commit}")
        surfaces[name] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    versions = {"old": stamp(args.old_bin, OLD_COMMIT),
                "candidate": stamp(args.candidate_bin, args.candidate_commit)}
    driver = args.candidate_bin / "examples/upgrade_window"
    driver_version = subprocess.check_output([str(driver), "--version"], text=True).strip()
    if f"({args.candidate_commit})" not in driver_version or "dirty" in driver_version:
        raise RuntimeError(f"upgrade driver must be a clean build of {args.candidate_commit}: {driver_version}")
    versions["driver"] = {"version": driver_version,
                          "sha256": hashlib.sha256(driver.read_bytes()).hexdigest()}
    project = "lys-upgrade-proof-" + str(os.getpid())
    browser = Browser(installation(root, args.old_source, project))
    headless = args.work / "headless"
    headless.mkdir()
    for name in ["open", "xdg-open"]:
        executable = headless / name
        executable.write_text("#!/bin/sh\nexit 1\n")
        executable.chmod(0o700)
    env = dict(os.environ, PATH=str(headless) + os.pathsep + os.environ["PATH"])
    installed = [str(args.old_bin / "lys"), "identity", "install", "--root", str(root),
                 "--surface", str(args.old_surface)]
    primary = None
    try:
        run(installed, evidence / "install-old.log", env)
        ids = populate(browser, root)
        legacy, legacy_browser = seed_legacy(browser, root, ids)
        (evidence / "legacy-before.json").write_text(json.dumps(legacy, indent=2))
        config_file = root / "identity.json"
        config = json.loads(config_file.read_text())
        bridge = {"url": f"http://127.0.0.1:{port()}",
                  "bindings": [{"participant": "upgrade-fixture", "identity": ids["owner"]}]}
        config["cambium_messages"] = bridge
        config_file.write_text(json.dumps(config))
        run(installed, evidence / "reinstall-old.log", env)
        old_config = json.loads(config_file.read_text())
        provenance = seed_provenance(browser, root, old_config, evidence)
        original_config = config_file.read_bytes()
        before = observe(browser, ids)
        leaves = app_leaves(root, old_config)
        (evidence / "app-leaves.json").write_text(json.dumps(leaves, indent=2))
        (evidence / "before.json").write_text(json.dumps(before, indent=2))
        files = legacy_files(root, old_config)
        profile = profile_file(root, old_config)
        context = {"provenance": provenance, "original_config_sha256": hashlib.sha256(original_config).hexdigest(), "files": files, "legacy": legacy, "port": browser.port, "cookie": browser.cookie,
                   "provisioning": before["provisioning"]}
        (root / ".upgrade-window.json").write_text(json.dumps(context))
        run([str(driver), "--root", str(root), "window", "--from", str(args.candidate_bin),
             "--surface", str(args.candidate_surface), "--verifier",
             str(Path(__file__).with_name("upgrade_window.py"))],
            evidence / "window-driver.log", env, expected=75)
        pending(root)
        unchanged(files, legacy_files(root, json.loads(config_file.read_text())))
        if args.negative_control:
            receipt = exercise_negative(root, evidence, driver, installed, env, files,
                                        run, stamp, OLD_COMMIT)
        else:
            # Re-enter the real old installer. Its existing recovery runs before installation.
            intent_bytes = (root / "install/upgrade.json").read_bytes()
            (evidence / "candidate-intent-read-by-old.json").write_bytes(intent_bytes)
            if (root / "config.previous/identity.json").read_bytes() != original_config:
                raise RuntimeError("candidate backup differs from old original config")
            run(installed, evidence / "recover-old.log", env)
            if config_file.read_bytes() != original_config:
                raise RuntimeError("old installer did not restore exact original config")
            verify_provenance(browser, provenance)
            (evidence / "old-intent-parser.json").write_text(json.dumps({
                "reader_commit": OLD_COMMIT, "intent_sha256": hashlib.sha256(intent_bytes).hexdigest(),
                "old_installer_recovered": True, "configuration_restored_byte_for_byte": True,
            }, indent=2))
            if (root / "install/upgrade.json").exists():
                raise RuntimeError("old installer recovery left the upgrade intent standing")
            stamp(root / "bin", OLD_COMMIT)
            same_records(before, observe(browser, ids))
            for path, expected in [
                (f"/teams/{legacy['team']}", legacy["team_before"]),
                (f"/budgets/person/{legacy['person']}", legacy["budgets_before"]),
            ]:
                if browser.ask("GET", path) != expected:
                    raise RuntimeError(f"old binary after rollback did not read original {path}")
            if legacy_browser.ask("GET", "/me")["person"]["id"] != ids["person"]:
                raise RuntimeError("rollback changed the ordinary person's original session")
            preserve_leaves(root, leaves)
            unchanged(profile, profile_file(root, json.loads(config_file.read_text())))
            (evidence / "rollback.json").write_text(json.dumps({
                "passed": True, "old_binary": OLD_COMMIT, "domains": list(before),
                "team": legacy["team"], "personal_budgets": 3,
            }, indent=2))
            # Independently exercise the candidate's own production recovery path.
            run([str(driver), "--root", str(root), "window", "--from", str(args.candidate_bin),
                 "--surface", str(args.candidate_surface), "--verifier",
                 str(Path(__file__).with_name("upgrade_window.py"))],
                evidence / "candidate-back-window.log", env, expected=75)
            if (root / "config.previous/identity.json").read_bytes() != original_config:
                raise RuntimeError("candidate back-path backup differs from old config")
            run([str(driver), "--root", str(root), "recover"],
                evidence / "candidate-recover.log", env)
            if config_file.read_bytes() != original_config:
                raise RuntimeError("candidate back path did not restore original config bytes")
            stamp(root / "bin", OLD_COMMIT)
            same_records(before, observe(browser, ids))
            verify_provenance(browser, provenance)
            (evidence / "candidate-back.json").write_text(json.dumps({
                "passed": True, "candidate": args.candidate_commit,
                "old_binary": OLD_COMMIT, "configuration_restored_byte_for_byte": True,
            }, indent=2))
            post_rollback_files = legacy_files(root, json.loads(config_file.read_text()))
            run([str(args.candidate_bin / "lys"), "identity", "upgrade", "--root", str(root),
                 "--from", str(args.candidate_bin), "--surface", str(args.candidate_surface)],
                evidence / "upgrade.log", env)
            verify_provenance(browser, provenance)
            operator_config = json.loads(config_file.read_text())
            operator_path = Path(operator_config["operator_token_file"]).resolve()
            if not operator_path.is_relative_to(root):
                raise RuntimeError("operator token leaves fixture after commit")
            held_proof = json.loads((evidence / "window.json").read_text())
            if hashlib.sha256(operator_path.read_bytes()).hexdigest() != held_proof["operator_token_sha256"]:
                raise RuntimeError("operator token changed between refusal and post-clear control")
            operator_people = browser.ask("GET", "/directory/people",
                                          operator=operator_path.read_text().strip())
            if operator_people != browser.ask("GET", "/directory/people"):
                raise RuntimeError("valid post-clear operator read differs from administrator read")
            after = observe(browser, ids)
            (evidence / "after.json").write_text(json.dumps(after, indent=2))
            same_records(before, after)
            legacy_counts = verify_legacy(browser, legacy)
            if legacy_browser.ask("GET", "/me")["person"]["id"] != ids["person"]:
                raise RuntimeError("the ordinary legacy person's session changed during upgrade")
            preserve_leaves(root, leaves)
            new_config = json.loads(config_file.read_text())
            unchanged(profile, profile_file(root, new_config))
            expected = {key: value for key, value in old_config.items() if key != "cambium_messages"}
            expected["message_service"] = dict(bridge, cookie="cambium_session")
            for key, value in expected.items():
                if new_config.get(key) != value:
                    raise RuntimeError(f"upgrade changed or removed configuration member {key}")
            if "cambium_messages" in new_config:
                raise RuntimeError("upgrade retained the legacy message key")
            unchanged(post_rollback_files, legacy_files(root, new_config))
            migration = settle_restart(root, evidence, driver, env, browser, legacy, run)
            unchanged(profile, profile_file(root, new_config))
            admitted_person = admitted_after_upgrade(browser)
            receipt = {"registered_admin_wrote": admitted_person, "old": OLD_COMMIT, "candidate": args.candidate_commit, "versions": versions,
                       "surfaces": surfaces, "domains": list(before), "old_app_leaves": len(leaves),
                       "config_keys_preserved": list(expected), "legacy": legacy_counts,
                       "reversible_window": True, "old_binary_rollback": True,
                       "normal_cli_upgrade": True, "passed": True}
            receipt["post_commit_migration"] = migration
    except BaseException as error:
        primary = error
        raise
    finally:
        try:
            stop_fixture(root, project, evidence)
        except (OSError, RuntimeError) as error:
            if primary is None:
                raise
            print(f"Fixture cleanup also failed: {error}", file=sys.stderr)

    (evidence / "receipt.json").write_text(json.dumps(receipt, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["old-source", "old-bin", "candidate-bin", "old-surface", "candidate-surface", "work"]:
        parser.add_argument("--" + name, type=lambda value: Path(value).resolve(), required=True)
    parser.add_argument("--candidate-commit", required=True)
    args = parser.parse_args()
    if re.fullmatch(r"[0-9a-f]{40}", args.candidate_commit) is None:
        parser.error("candidate-commit must be a full commit id")
    os.umask(0o077)
    args.work.mkdir(mode=0o700)
    for name, negative in [("positive", False), ("negative", True)]:
        leg = argparse.Namespace(**vars(args))
        leg.work = args.work / name
        leg.negative_control = negative
        exercise(leg)
    (args.work / "receipt.json").write_text(json.dumps({
        "passed": True, "candidate": args.candidate_commit,
        "positive": "positive/evidence/receipt.json",
        "negative": "negative/evidence/receipt.json",
    }, indent=2))


if __name__ == "__main__":
    main()
