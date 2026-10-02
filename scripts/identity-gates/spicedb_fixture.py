"""Run an explicit live target against a disposable permission database."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import secrets
import signal
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]


def docker(*arguments):
    result = subprocess.run(["docker", *arguments], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f"spicedb_container_failed: docker {arguments[0]} exit {result.returncode}")
    return result.stdout.strip()


class Cancelled(RuntimeError):
    pass


def cancel(signum, frame):
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    raise Cancelled(f"spicedb_cancelled: signal {signum}")


@contextmanager
def owned_process(arguments, **options):
    previous = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGTERM})
    try:
        process = subprocess.Popen(arguments, **options)
        try:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)
            yield process
        finally:
            if process.poll() is None:
                process.terminate()
            process.wait()
            for stream in (process.stdin, process.stdout, process.stderr):
                if stream is not None:
                    stream.close()
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, previous)


def ready(container):
    with owned_process(
        ["docker", "logs", "--follow", container],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    ) as logger:
        for line in logger.stdout:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("message") == "http server started serving" and event.get("service") == "http":
                return
        raise RuntimeError("spicedb_not_ready: container ended before its HTTP ready line")


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
        previous = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGTERM})
        try:
            container = docker(
                "run", "--detach", "--publish", "127.0.0.1::8443", "--env-file", str(environment_file),
                image, "serve", "--datastore-engine=memory", "--http-enabled", "--log-format=json",
            )
            if not re.fullmatch("[0-9a-f]{64}", container):
                raise RuntimeError("spicedb_container_invalid: docker did not return an owned container id")
            try:
                signal.pthread_sigmask(signal.SIG_SETMASK, previous)
                ready(container)
                endpoint = docker("port", container, "8443/tcp")
                if not re.fullmatch(r"127\.0\.0\.1:[0-9]{1,5}", endpoint):
                    raise RuntimeError("spicedb_endpoint_invalid: no single loopback HTTP endpoint")
                environment = dict(os.environ, LYS_SPICEDB_ENDPOINT=endpoint, LYS_SPICEDB_KEY_FILE=str(key_file))
                print(f"spicedb_fixture_ready: {container[:12]} {endpoint}", flush=True)
                with owned_process(command, env=environment) as process:
                    return process.wait()
            finally:
                docker("rm", "--force", container)
        finally:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)


def main(arguments):
    previous = signal.signal(signal.SIGTERM, cancel)
    try:
        return run(arguments[1:] if arguments[:1] == ["--"] else arguments)
    except Cancelled as error:
        print(str(error), file=sys.stderr)
        return 128 + signal.SIGTERM
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"spicedb_fixture_failed: {error}", file=sys.stderr)
        return 1
    finally:
        signal.signal(signal.SIGTERM, previous)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
