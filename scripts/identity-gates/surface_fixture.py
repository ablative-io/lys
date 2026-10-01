"""Prepare one committed surface for test processes sharing Cargo's scratch area."""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tarfile
import tempfile


def output(argv, cwd):
    return subprocess.check_output(argv, cwd=cwd, text=True).strip()


def prepared(root, scratch):
    commit = output(["git", "rev-parse", "HEAD"], root)
    changed = output([
        "git", "status", "--porcelain", "--untracked-files=all", "--",
        "surface/identity", "docs",
    ], root)
    if changed:
        raise RuntimeError("surface_fixture_source_dirty: commit the surface and its documents")
    identity = {
        "commit": commit,
        "node": output(["node", "--version"], root),
        "npm": output(["npm", "--version"], root),
        "platform": platform.system(),
        "machine": platform.machine(),
        "service": os.environ.get("LYS_IDENTITY_SERVICE"),
        "preparer": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    def build(stage):
        archive = subprocess.Popen([
            "git", "archive", "--format=tar", commit, "surface/identity", "docs",
        ], cwd=root, stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-f", "-", "-C", str(stage)], stdin=archive.stdout, check=True)
        archive.stdout.close()
        if archive.wait() != 0:
            raise RuntimeError(f"surface_fixture_archive_failed: git archive exited {archive.returncode}")
        surface = stage / "surface/identity"
        for argv in [["npm", "ci", "--include=dev"], ["npm", "run", "build"]]:
            subprocess.run(argv, cwd=surface, stdout=sys.stderr, check=True)

    return publish(scratch, identity, build)


def publish(scratch, identity, build):
    key = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
    cache = scratch / "identity-surface"
    cache.mkdir(parents=True, exist_ok=True)
    destination = cache / key
    with (cache / (key + ".lock")).open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if destination.exists():
            recorded = json.loads((destination / "fixture.json").read_text())
            if recorded != identity:
                raise RuntimeError("surface_fixture_identity_mismatch: prepared inputs differ")
            validate(destination)
            return destination
        with tempfile.TemporaryDirectory(prefix=key + ".", dir=cache) as temporary:
            stage = Path(temporary) / "fixture"
            stage.mkdir()
            build(stage)
            validate(stage)
            (stage / "fixture.json").write_text(json.dumps(identity, sort_keys=True))
            stage.rename(destination)
    return destination


def validate(directory):
    for relative in [
        "surface/identity/dist/index.html",
        "surface/identity/node_modules/vite/bin/vite.js",
    ]:
        if not (directory / relative).is_file():
            raise RuntimeError("surface_fixture_incomplete: missing " + relative)


def main():
    root = Path(__file__).resolve().parents[2]
    if len(sys.argv) == 2:
        scratch = Path(sys.argv[1]).resolve()
    elif len(sys.argv) == 1:
        metadata = json.loads(output([
            "cargo", "metadata", "--offline", "--no-deps", "--format-version=1",
        ], root))
        scratch = Path(metadata["target_directory"]) / "tmp"
    else:
        raise RuntimeError("surface_fixture_arguments: expected only Cargo's test scratch directory")
    print(prepared(root, scratch) / "surface/identity")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError, tarfile.TarError) as error:
        sys.stderr.write("surface_fixture_failed: " + str(error) + "\n")
        sys.exit(1)
