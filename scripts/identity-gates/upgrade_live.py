"""Upgrade a disposable real old install; never operate a person's live root."""

import argparse
from functools import partial
from itertools import islice
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys

from upgrade_fixture import Browser, admitted_after_upgrade, observe, populate, same_records
from upgrade_legacy import seed as seed_legacy, verify as verify_legacy
from upgrade_window import MARKER, legacy_files, pending, profile_file, unchanged
from upgrade_negative import exercise as exercise_negative
from upgrade_restart import settle as settle_restart
from upgrade_provenance import seed as seed_provenance, verify as verify_provenance

from upgrade_preflight import socket_paths
from upgrade_teardown import terminate_fixture
from upgrade_layout import inventory, executables, harness_inventory, harness_paths
from upgrade_release import old_release

PROGRAMS = ("lys", "lys-identity-server", "lys-secrets")


def run(command, log, env=None, expected=0):
    with log.open("wb") as output:
        completed = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, env=env)
    if completed.returncode != expected:
        raise RuntimeError(
            f"{command[0]} exited {completed.returncode}, expected {expected}; evidence: {log}"
        )


def refuse_old_put_back(installed, log, env, root):
    """An old installer that keeps no data must refuse the record by name and touch nothing.

    It cannot put back the data the candidate wrote, which its own build cannot
    read, so it must refuse the record on the step it does not know before it
    stops or moves anything: the record and the kept binaries stand for the
    candidate's installer, which puts the kept data back, to finish.
    """
    with log.open("wb") as output:
        completed = subprocess.run(installed, stdout=output, stderr=subprocess.STDOUT, env=env)
    if completed.returncode == 0:
        raise RuntimeError(f"old installer reported a put-back it cannot make; evidence: {log}")
    if "unknown variant `data_kept`" not in log.read_text(errors="replace"):
        raise RuntimeError(f"old installer did not refuse the record by its kept-data step; evidence: {log}")
    if not (root / "install/upgrade.json").is_file():
        raise RuntimeError(f"old installer's refusal removed the upgrade record; evidence: {log}")
    if not (root / "bin.previous").is_dir():
        raise RuntimeError(f"old installer moved the kept binaries before refusing; evidence: {log}")


def stamp(directory, expected, programs=PROGRAMS):
    versions = {}
    for name in programs:
        output = subprocess.check_output([str(directory / name), "--version"], text=True).strip()
        if f"({expected})" not in output or "dirty" in output:
            raise RuntimeError(f"{directory / name} must be a clean build of {expected}: {output}")
        versions[name] = {
            "version": output,
            "sha256": hashlib.sha256((directory / name).read_bytes()).hexdigest(),
        }
    return versions


def app_leaves(root, config):
    directory = Path(config["grant_log_dir"]).with_name("apps")
    if not directory.is_relative_to(root):
        raise RuntimeError(f"fixture app log leaves its root: {directory}")
    leaves = {
        str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in (directory / "leaves").iterdir()
        if re.fullmatch(r"[0-9]{20}", path.name)
    }
    if len(leaves) < 3:
        raise RuntimeError("old app log does not hold model, registration and approval")
    return leaves


def preserve_leaves(root, before):
    for name, digest in before.items():
        if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"upgrade changed old app leaf {name}")


def directory_path(root, config):
    original_root = root.absolute()
    root = root.resolve()
    directory = Path(config["log_dir"])
    if not directory.is_absolute() or not directory.resolve().is_relative_to(root):
        raise RuntimeError("fixture directory log leaves its root")
    if directory.is_relative_to(original_root):
        relative = directory.relative_to(original_root)
    elif directory.is_relative_to(root):
        relative = directory.relative_to(root)
    else:
        raise RuntimeError("fixture directory log leaves its root")
    if ".." in relative.parts:
        raise RuntimeError("fixture directory log leaves its root")
    directory = root / relative
    current = directory
    while current != root:
        if current.is_symlink():
            raise RuntimeError("fixture directory log contains a symlink")
        current = current.parent
    return directory


def directory_leaves(root, config):
    """Keep complete bytes, rather than treating equal digests as byte equality."""
    directory = directory_path(root, config) / "leaves"
    if directory.is_symlink():
        raise RuntimeError("fixture directory leaves contain a symlink")
    leaves = {}
    for path in sorted(directory.iterdir()):
        if re.fullmatch(r"\.\d+-\d{20}-\d+\.tmp", path.name):
            continue
        if re.fullmatch(r"[0-9]{20}", path.name) is None:
            raise RuntimeError(f"unexpected directory leaf name {path.name}")
        if path.is_symlink() or not path.is_file():
            raise RuntimeError(f"directory leaf is a symlink or not a file: {path.name}")
        if int(path.name) != len(leaves):
            raise RuntimeError("directory leaf indexes are not contiguous from zero")
        leaves[str(path.relative_to(root.resolve()))] = path.read_bytes().hex()
    if not leaves:
        raise RuntimeError("empty directory cannot prove an upgrade")
    return leaves


def preserve_directory(before, after):
    if not before:
        raise RuntimeError("empty directory cannot prove preservation")
    for name, content in before.items():
        if name not in after:
            raise RuntimeError(f"upgrade missing old directory leaf {name}")
        if after[name] != content:
            raise RuntimeError(f"upgrade changed old directory leaf {name}")
    return {
        "before": len(before),
        "after": len(after),
        "preserved": len(before),
        "appended": len(after) - len(before),
    }


class SnapshotBytes:
    def __init__(self, data):
        self.data = data
        self.offset = 0

    def take(self, size):
        if size > len(self.data) - self.offset:
            raise RuntimeError("truncated directory snapshot")
        value = self.data[self.offset : self.offset + size]
        self.offset += size
        return value

    def frame(self):
        return self.take(int.from_bytes(self.take(8), "big"))

    def finish(self):
        if self.offset != len(self.data):
            raise RuntimeError("trailing directory snapshot bytes")

    def cbor(self, depth=0):
        if depth > 32:
            raise RuntimeError("directory snapshot CBOR nesting is too deep")
        tag = self.take(1)[0]
        if tag == 0xF6:
            return None
        major, additional = divmod(tag, 32)
        if major not in (0, 2, 3, 4) or additional > 27:
            raise RuntimeError("directory snapshot CBOR has an unsupported value")
        if additional < 24:
            count = additional
        else:
            width = 1 << (additional - 24)
            count = int.from_bytes(self.take(width), "big")
            if count < (24 if width == 1 else 1 << (8 * (width // 2))):
                raise RuntimeError("directory snapshot CBOR is not canonical")
        if major == 0:
            return count
        if major in (2, 3):
            value = self.take(count)
            if major == 2:
                return value
            try:
                return value.decode("utf-8")
            except UnicodeDecodeError as error:
                raise RuntimeError("directory snapshot text is not UTF-8") from error
        if count > len(self.data) - self.offset:
            raise RuntimeError("truncated directory snapshot array")
        values = []
        while len(values) < count:
            values.append(self.cbor(depth + 1))
        return values


def ones(number):
    """Set bits, as int.bit_count counts them; the gate's python3 is 3.9."""
    return bin(number).count("1")


def snapshot_value(data):
    reader = SnapshotBytes(data)
    value = reader.cbor()
    reader.finish()
    return value


# The directory state each released version writes, as (projection parts,
# fields in a record), from crates/lys-identity/src/directory_state.rs and
# projection_state.rs at the release: version 2 has no
# reports_to field, version 3 added it, and versions 4 and 5 hold a fourth
# projection part. Every version keeps a record's first five fields (identity,
# name, state, responsible, logins), which is all this proof reads.
OWNER_SHAPES = {2: (3, 7), 3: (3, 8), 4: (4, 8), 5: (4, 8)}


def old_directory_snapshot(root, config, leaves, administrator):
    """Inspect the actual old writer's sealed owner state, never synthesize it."""
    path = directory_path(root, config) / "snapshot.bin"
    if path.is_symlink():
        raise RuntimeError("directory snapshot is a symlink")
    sealed = path.read_bytes()
    file = SnapshotBytes(sealed)
    body, signature = file.frame(), file.frame()
    file.finish()
    if len(signature) != 64:
        raise RuntimeError("directory snapshot lacks its Ed25519 signature")
    reader = SnapshotBytes(body)
    if reader.frame() != b"lys/log-snapshot/v1":
        raise RuntimeError("wrong directory snapshot format")
    if reader.frame() != b"lys/identity-directory/v1":
        raise RuntimeError("wrong directory snapshot domain")
    if reader.frame() != config["log_origin"].encode():
        raise RuntimeError("wrong directory snapshot origin")
    size = int.from_bytes(reader.take(8), "big")
    root_hash, frontier, state = reader.take(32), reader.frame(), reader.frame()
    reader.finish()
    if size == 0 or size > len(leaves):
        raise RuntimeError("old snapshot must fold nonempty fixture leaves")
    if len(frontier) != ones(size) * 32:
        raise RuntimeError("wrong directory snapshot frontier")
    nodes = []
    for index, content in enumerate(islice(leaves.values(), size)):
        node = hashlib.sha256(b"\x00" + bytes.fromhex(content)).digest()
        count = index
        while count & 1:
            node = hashlib.sha256(b"\x01" + nodes.pop() + node).digest()
            count >>= 1
        nodes.append(node)
    expected_root = nodes[-1]
    for node in reversed(nodes[:-1]):
        expected_root = hashlib.sha256(b"\x01" + node + expected_root).digest()
    if b"".join(nodes) != frontier or expected_root != root_hash:
        raise RuntimeError("directory snapshot root does not match old leaf bytes")
    wrapped = snapshot_value(state)
    if not isinstance(wrapped, list) or len(wrapped) != 3 or wrapped[0] != 1:
        raise RuntimeError("wrong directory snapshot wrapper")
    checkpoints, owner = wrapped[1:]
    if (
        not isinstance(checkpoints, list)
        or len(checkpoints) != size // 1024 + 1
        or any(
            not isinstance(nodes, bytes) or len(nodes) != ones(slot * 1024) * 32
            for slot, nodes in enumerate(checkpoints)
        )
    ):
        raise RuntimeError("wrong directory snapshot checkpoints")
    if not isinstance(owner, bytes):
        raise RuntimeError("directory snapshot owner is not bytes")
    owner = snapshot_value(owner)
    if not isinstance(owner, list) or len(owner) != 3 or type(owner[0]) is not int:
        raise RuntimeError("old directory snapshot owner has the wrong shape")
    version = owner[0]
    if version not in OWNER_SHAPES:
        raise RuntimeError(
            f"old directory snapshot owner version {version} is not one this proof reads"
        )
    parts, fields = OWNER_SHAPES[version]
    if type(owner[1]) is not int or owner[1] != size:
        raise RuntimeError("directory snapshot folded count differs from its tree size")
    projection = owner[2]
    if (
        not isinstance(projection, list)
        or len(projection) != parts
        or not isinstance(projection[0], list)
    ):
        raise RuntimeError("old directory snapshot projection has the wrong shape")
    identifier = administrator["person"]["id"]
    if re.fullmatch(r"person-[0-9a-f]{32}", identifier) is None:
        raise RuntimeError("old administrator identifier is malformed")
    records = [
        record
        for record in projection[0]
        if isinstance(record, list)
        and len(record) == fields
        and record[0] == [1, bytes.fromhex(identifier[7:])]
    ]
    # /me names a login's issuer URL as `provider`, exactly as the directory
    # record keeps it in each [issuer, subject] pair.
    login = administrator["signed_in"]
    if (
        len(records) != 1
        or records[0][2] != 2
        or not isinstance(records[0][4], list)
        or [login["provider"], login["subject"]] not in records[0][4]
    ):
        raise RuntimeError("old snapshot does not hold the active bound administrator")
    return {
        "owner_version": version,
        "folded": size,
        "administrator": identifier,
        "sha256": hashlib.sha256(sealed).hexdigest(),
        "root": root_hash.hex(),
    }


def port():
    with socket.socket() as held:
        held.bind(("127.0.0.1", 0))
        return held.getsockname()[1]


def require_free(number):
    with socket.socket() as held:
        # Match Tokio's Unix listener: TIME_WAIT is not an active listener.
        held.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            held.bind(("127.0.0.1", number))
            held.listen()
        except OSError as error:
            raise RuntimeError(
                f"fixture service port 127.0.0.1:{number} unavailable: {error}"
            ) from error


def network_subnets(networks):
    """Docker host/none networks have no IPAM allocation to reserve."""
    used = set()
    for network in networks:
        ipam = network.get("IPAM")
        if ipam is None:
            continue
        configurations = ipam.get("Config")
        if configurations is None:
            continue
        for entry in configurations:
            subnet = entry.get("Subnet")
            if subnet is not None:
                ipaddress.ip_network(subnet)
                used.add(subnet)
    return used


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
    used = network_subnets(held)
    network = next(
        (
            f"172.29.{i}.0/24"
            for i in range(48, 255)
            if all(
                not ipaddress.ip_network(f"172.29.{i}.0/24").overlaps(ipaddress.ip_network(subnet))
                for subnet in used
                if ipaddress.ip_network(subnet).version == 4
            )
        ),
        None,
    )
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
        process = subprocess.run(
            ["ps", "-p", str(pid), "-o", "command="], capture_output=True, text=True
        )
        if process.returncode:
            continue
        if str(root) not in process.stdout:
            errors.append(f"refuse to stop {pid}: it does not name fixture root {root}")
            continue
        try:
            terminate_fixture(pid)
        except (OSError, RuntimeError) as error:
            errors.append(f"fixture {root} process {pid}: {error}")
    compose = root / "deploy/compose.yaml"
    if compose.exists():
        try:
            run(
                [
                    "docker",
                    "compose",
                    "-f",
                    str(compose),
                    "--env-file",
                    str(root / "state/compose.env"),
                    "-p",
                    project,
                    "--profile",
                    "bundled-db",
                    "down",
                    "-v",
                    "--remove-orphans",
                ],
                evidence / "teardown.log",
            )
        except (OSError, RuntimeError) as error:
            errors.append(str(error))
    if errors:
        raise RuntimeError("; ".join(errors))


def prepare(args):
    """Validate copied artifacts and kernel paths without using service ports."""
    if args.work.exists():
        raise RuntimeError(f"fixture work directory already exists: {args.work}")
    old_head = subprocess.check_output(
        ["git", "-C", str(args.old_source), "rev-parse", "HEAD"], text=True
    ).strip()
    if old_head != args.old_commit:
        raise RuntimeError(f"old source is {old_head}, expected {args.old_commit}")
    dirty = subprocess.check_output(
        ["git", "-C", str(args.old_source), "status", "--porcelain"], text=True
    )
    if dirty:
        raise RuntimeError("old source must be clean, including its deployment template")

    def old_text(path):
        return (args.old_source / path).read_text()

    layouts = {"old": inventory(old_text, args.old_commit)}
    release = old_release(old_text)
    repository = Path(__file__).resolve().parents[2]

    def candidate_source(path):
        return subprocess.check_output(
            ["git", "-C", str(repository), "show", args.candidate_commit + ":" + path], text=True
        )

    layouts["candidate"] = inventory(candidate_source, args.candidate_commit, candidate=True)
    installed_binaries = layouts["old"]["installed_binaries"]
    if installed_binaries != layouts["candidate"]["installed_binaries"]:
        raise RuntimeError(
            "old and candidate installed binary layouts differ; proof needs a new contract"
        )
    surfaces = {}
    for name, path, commit in [
        ("old", args.old_surface, args.old_commit),
        ("candidate", args.candidate_surface, args.candidate_commit),
    ]:
        manifest_path = path / "surface-manifest.json"
        manifest = json.loads(manifest_path.read_text())
        if manifest["commit"] != commit:
            raise RuntimeError(f"{name} surface does not name {commit}")
        for entry in manifest["files"]:
            artifact = (path / entry["path"]).resolve()
            if not artifact.is_relative_to(path) or not artifact.is_file():
                raise RuntimeError(
                    f"{name} surface artifact absent or outside its root: {artifact}"
                )
            data = artifact.read_bytes()
            if len(data) != entry["bytes"] or hashlib.sha256(data).hexdigest() != entry["sha256"]:
                raise RuntimeError(f"{name} surface artifact differs from its manifest: {artifact}")
        surfaces[name] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    versions = {
        "old": stamp(args.old_bin, args.old_commit),
        "candidate": stamp(args.candidate_bin, args.candidate_commit),
    }
    driver = args.candidate_bin / "examples/upgrade_window"
    driver_version = subprocess.check_output([str(driver), "--version"], text=True).strip()
    if f"({args.candidate_commit})" not in driver_version or "dirty" in driver_version:
        raise RuntimeError(
            f"upgrade driver must be a clean build of {args.candidate_commit}: {driver_version}"
        )
    versions["driver"] = {
        "version": driver_version,
        "sha256": hashlib.sha256(driver.read_bytes()).hexdigest(),
    }
    paths = socket_paths(args.work)
    verifiers = harness_inventory(Path(__file__).resolve().parent)
    return {
        "old": args.old_commit,
        "candidate": args.candidate_commit,
        "versions": versions,
        "surfaces": surfaces,
        "socket_paths": paths,
        "layouts": layouts,
        "release": release,
        "executables": executables(),
        "verifiers": verifiers,
        "harness_paths": harness_paths(verifiers, layouts),
    }


def exercise(args):
    os.umask(0o077)
    args.work.mkdir(mode=0o700)
    evidence = args.work / "evidence"
    evidence.mkdir()
    root = args.work / "install"
    root.mkdir()
    (root / ".upgrade-proof").write_text(MARKER)
    versions = args.prepared["versions"]
    surfaces = args.prepared["surfaces"]
    driver = args.candidate_bin / "examples/upgrade_window"
    installed_stamp = partial(stamp, programs=args.prepared["layouts"]["old"]["installed_binaries"])
    project = "lys-upgrade-proof-" + str(os.getpid())
    browser = Browser(installation(root, args.old_source, project))
    headless = args.work / "headless"
    headless.mkdir()
    for name in ["open", "xdg-open"]:
        executable = headless / name
        executable.write_text("#!/bin/sh\nexit 1\n")
        executable.chmod(0o700)
    env = dict(
        os.environ,
        PATH=str(headless) + os.pathsep + os.environ["PATH"],
        PYTHONDONTWRITEBYTECODE="1",
    )
    installed = [
        str(args.old_bin / "lys"),
        "identity",
        "install",
        "--root",
        str(root),
        "--surface",
        str(args.old_surface),
    ]
    primary = None
    try:
        run(installed, evidence / "install-old.log", env)
        release = args.prepared["release"]
        ids = populate(browser, root, release)
        legacy, legacy_browser = seed_legacy(browser, root, ids, release)
        (evidence / "legacy-before.json").write_text(json.dumps(legacy, indent=2))
        config_file = root / "identity.json"
        config = json.loads(config_file.read_text())
        bridge = {
            "url": f"http://127.0.0.1:{port()}",
            "bindings": [{"participant": "upgrade-fixture", "identity": ids["owner"]}],
        }
        config["cambium_messages"] = bridge
        config_file.write_text(json.dumps(config))
        # Rebuilding with the real old writer puts the nonempty directory in its snapshot.
        old_snapshot = directory_path(root, config) / "snapshot.bin"
        if old_snapshot.is_symlink():
            raise RuntimeError("old directory snapshot is a symlink")
        old_snapshot.unlink()
        run(installed, evidence / "reinstall-old.log", env)
        old_config = json.loads(config_file.read_text())
        provenance = seed_provenance(browser, root, old_config, evidence, args.old_commit)
        original_config = config_file.read_bytes()
        before = observe(browser, ids)
        leaves = app_leaves(root, old_config)
        directory_before = directory_leaves(root, old_config)
        snapshot = old_directory_snapshot(root, old_config, directory_before, before["me"])
        snapshot["writer_commit"] = args.old_commit
        (evidence / "directory-before.json").write_text(json.dumps(directory_before, indent=2))
        (evidence / "old-directory-v3.json").write_text(json.dumps(snapshot, indent=2))
        (evidence / "old-directory-snapshot.bin").write_bytes(
            (directory_path(root, old_config) / "snapshot.bin").read_bytes()
        )
        (evidence / "app-leaves.json").write_text(json.dumps(leaves, indent=2))
        (evidence / "before.json").write_text(json.dumps(before, indent=2))
        files = legacy_files(root, old_config)
        profile = profile_file(root, old_config)
        context = {
            "provenance": provenance,
            "original_config_sha256": hashlib.sha256(original_config).hexdigest(),
            "files": files,
            "legacy": legacy,
            "port": browser.port,
            "cookie": browser.cookie,
            "provisioning": before["provisioning"],
        }
        (root / ".upgrade-window.json").write_text(json.dumps(context))
        run(
            [
                str(driver),
                "--root",
                str(root),
                "window",
                "--from",
                str(args.candidate_bin),
                "--surface",
                str(args.candidate_surface),
                "--verifier",
                str(Path(__file__).with_name("upgrade_window.py")),
            ],
            evidence / "window-driver.log",
            env,
            expected=75,
        )
        pending(root)
        unchanged(files, legacy_files(root, json.loads(config_file.read_text())))
        if args.negative_control:
            receipt = exercise_negative(
                root, evidence, driver, installed, env, files, run, installed_stamp, args.old_commit,
                args.prepared["release"]["teams"],
            )
        else:
            # Re-enter the real old installer. Its existing recovery runs before installation.
            intent_bytes = (root / "install/upgrade.json").read_bytes()
            (evidence / "candidate-intent-read-by-old.json").write_bytes(intent_bytes)
            if (root / "config.previous/identity.json").read_bytes() != original_config:
                raise RuntimeError("candidate backup differs from old original config")
            put_back = args.prepared["release"]["put_back"]
            if put_back == "keeps_data":
                run(installed, evidence / "recover-old.log", env)
            else:
                refuse_old_put_back(installed, evidence / "recover-old.log", env, root)
                run([str(driver), "--root", str(root), "recover"],
                    evidence / "recover-after-old.log", env)
            if config_file.read_bytes() != original_config:
                raise RuntimeError("old installer did not restore exact original config")
            verify_provenance(browser, provenance)
            (evidence / "old-intent-parser.json").write_text(
                json.dumps(
                    {
                        "reader_commit": args.old_commit,
                        "intent_sha256": hashlib.sha256(intent_bytes).hexdigest(),
                        "old_installer_recovered": put_back == "keeps_data",
                        "recovered_by": "old" if put_back == "keeps_data" else "candidate",
                        "configuration_restored_byte_for_byte": True,
                    },
                    indent=2,
                )
            )
            if (root / "install/upgrade.json").exists():
                raise RuntimeError("old installer recovery left the upgrade intent standing")
            installed_stamp(root / "bin", args.old_commit)
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
            (evidence / "rollback.json").write_text(
                json.dumps(
                    {
                        "passed": True,
                        "old_binary": args.old_commit,
                        "domains": list(before),
                        "team": legacy["team"],
                        "personal_budgets": 3,
                    },
                    indent=2,
                )
            )
            # Independently exercise the candidate's own production recovery path.
            run(
                [
                    str(driver),
                    "--root",
                    str(root),
                    "window",
                    "--from",
                    str(args.candidate_bin),
                    "--surface",
                    str(args.candidate_surface),
                    "--verifier",
                    str(Path(__file__).with_name("upgrade_window.py")),
                ],
                evidence / "candidate-back-window.log",
                env,
                expected=75,
            )
            if (root / "config.previous/identity.json").read_bytes() != original_config:
                raise RuntimeError("candidate back-path backup differs from old config")
            run(
                [str(driver), "--root", str(root), "recover"],
                evidence / "candidate-recover.log",
                env,
            )
            if config_file.read_bytes() != original_config:
                raise RuntimeError("candidate back path did not restore original config bytes")
            installed_stamp(root / "bin", args.old_commit)
            same_records(before, observe(browser, ids))
            verify_provenance(browser, provenance)
            (evidence / "candidate-back.json").write_text(
                json.dumps(
                    {
                        "passed": True,
                        "candidate": args.candidate_commit,
                        "old_binary": args.old_commit,
                        "configuration_restored_byte_for_byte": True,
                    },
                    indent=2,
                )
            )
            post_rollback_files = legacy_files(root, json.loads(config_file.read_text()))
            run(
                [
                    str(args.candidate_bin / "lys"),
                    "identity",
                    "upgrade",
                    "--root",
                    str(root),
                    "--from",
                    str(args.candidate_bin),
                    "--surface",
                    str(args.candidate_surface),
                ],
                evidence / "upgrade.log",
                env,
            )
            verify_provenance(browser, provenance)
            operator_config = json.loads(config_file.read_text())
            operator_path = Path(operator_config["operator_token_file"]).resolve()
            if not operator_path.is_relative_to(root):
                raise RuntimeError("operator token leaves fixture after commit")
            held_proof = json.loads((evidence / "window.json").read_text())
            if (
                hashlib.sha256(operator_path.read_bytes()).hexdigest()
                != held_proof["operator_token_sha256"]
            ):
                raise RuntimeError("operator token changed between refusal and post-clear control")
            operator_people = browser.ask(
                "GET", "/directory/people", operator=operator_path.read_text().strip()
            )
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
            directory_after = directory_leaves(root, new_config)
            directory_counts = preserve_directory(directory_before, directory_after)
            (evidence / "directory-after.json").write_text(json.dumps(directory_after, indent=2))
            unchanged(profile, profile_file(root, new_config))
            expected = {
                key: value for key, value in old_config.items() if key != "cambium_messages"
            }
            expected["message_service"] = dict(bridge, cookie="cambium_session")
            for key, value in expected.items():
                if new_config.get(key) != value:
                    raise RuntimeError(f"upgrade changed or removed configuration member {key}")
            if "cambium_messages" in new_config:
                raise RuntimeError("upgrade retained the legacy message key")
            unchanged(post_rollback_files, legacy_files(root, new_config))
            migration = settle_restart(root, evidence, driver, env, browser, legacy, run)
            unchanged(profile, profile_file(root, new_config))
            admitted_person = admitted_after_upgrade(browser, before["me"])
            directory_final = directory_leaves(root, new_config)
            final_counts = preserve_directory(directory_before, directory_final)
            (evidence / "directory-final.json").write_text(json.dumps(directory_final, indent=2))
            receipt = {
                "active_admin_wrote": admitted_person,
                "old": args.old_commit,
                "candidate": args.candidate_commit,
                "versions": versions,
                "surfaces": surfaces,
                "domains": list(before),
                "old_app_leaves": len(leaves),
                "config_keys_preserved": list(expected),
                "legacy": legacy_counts,
                "reversible_window": True,
                "old_binary_rollback": True,
                "normal_cli_upgrade": True,
                "passed": True,
            }
            receipt["post_commit_migration"] = migration
            receipt["old_directory_snapshot"] = snapshot
            receipt["directory_leaves_after_upgrade"] = directory_counts
            receipt["directory_leaves_after_restart_and_admin_write"] = final_counts
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
    for name in [
        "old-source",
        "old-bin",
        "candidate-bin",
        "old-surface",
        "candidate-surface",
        "work",
    ]:
        parser.add_argument("--" + name, type=lambda value: Path(value).resolve(), required=True)
    parser.add_argument("--candidate-commit", required=True)
    parser.add_argument("--old-commit", required=True)
    parser.add_argument("--prepare-only", action="store_true")
    args = parser.parse_args()
    for name in ("old_commit", "candidate_commit"):
        if re.fullmatch(r"[0-9a-f]{40}", getattr(args, name)) is None:
            parser.error(f"{name} must be a full commit id")
    args.prepared = prepare(args)
    if args.prepare_only:
        print(json.dumps(args.prepared, indent=2))
        return
    os.umask(0o077)
    args.work.mkdir(mode=0o700)
    for name, negative in [("positive", False), ("negative", True)]:
        leg = argparse.Namespace(**vars(args))
        leg.work = args.work / name
        leg.negative_control = negative
        exercise(leg)
    (args.work / "receipt.json").write_text(
        json.dumps(
            {
                "passed": True,
                "old": args.old_commit,
                "candidate": args.candidate_commit,
                "positive": "positive/evidence/receipt.json",
                "negative": "negative/evidence/receipt.json",
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
