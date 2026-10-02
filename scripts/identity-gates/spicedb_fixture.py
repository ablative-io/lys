"""Run an explicit live target against a disposable permission database."""

import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]


def docker(*arguments):
    result = subprocess.run(["docker", *arguments], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f"spicedb_container_failed: docker {arguments[0]} exit {result.returncode}")
    return result.stdout.strip()


def ready(container):
    logger = subprocess.Popen(
        ["docker", "logs", "--follow", container],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        for line in logger.stdout:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("message") == "http server started serving" and event.get("service") == "http":
                return
        raise RuntimeError("spicedb_not_ready: container ended before its HTTP ready line")
    finally:
        if logger.poll() is None:
            logger.terminate()
        logger.wait()
        logger.stdout.close()


def run(command):
    if not command:
        raise ValueError("spicedb_command_missing: an explicit test command is required")
    docker("info")
    versions = json.loads((ROOT / "deploy/identity/versions.json").read_text())
    image = versions["spicedb"]["image"] + "@" + versions["spicedb"]["image_index_digest"]
    with tempfile.TemporaryDirectory(prefix="lys-spicedb-") as temporary:
        directory = Path(temporary)
        key = secrets.token_hex(32)
        key_file = directory / "key"
        environment_file = directory / "environment"
        for path, text in [(key_file, key), (environment_file, f"SPICEDB_GRPC_PRESHARED_KEY={key}\n")]:
            with open(path, "x", opener=lambda name, flags: os.open(name, flags, 0o600)) as output:
                output.write(text)
        container = docker(
            "run", "--detach", "--publish", "127.0.0.1::8443", "--env-file", str(environment_file),
            image, "serve", "--datastore-engine=memory", "--http-enabled", "--log-format=json",
        )
        if not re.fullmatch("[0-9a-f]{64}", container):
            raise RuntimeError("spicedb_container_invalid: docker did not return an owned container id")
        try:
            ready(container)
            endpoint = docker("port", container, "8443/tcp")
            if not re.fullmatch(r"127\.0\.0\.1:[0-9]{1,5}", endpoint):
                raise RuntimeError("spicedb_endpoint_invalid: no single loopback HTTP endpoint")
            environment = dict(os.environ, LYS_SPICEDB_ENDPOINT=endpoint, LYS_SPICEDB_KEY_FILE=str(key_file))
            print(f"spicedb_fixture_ready: {container[:12]} {endpoint}", flush=True)
            return subprocess.run(command, env=environment).returncode
        finally:
            docker("rm", "--force", container)


def main(arguments):
    try:
        return run(arguments[1:] if arguments[:1] == ["--"] else arguments)
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"spicedb_fixture_failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
