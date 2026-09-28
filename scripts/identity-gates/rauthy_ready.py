#!/usr/bin/env python3
"""The rauthy-ready demand leg: start the pinned Rauthy against a scratch PostgreSQL.

    usage: python3 scripts/identity-gates/rauthy_ready.py [--versions-file <path>]

    exit 0  Rauthy answered /auth/v1/health with db_healthy and cache_healthy true,
            and everything this run created was removed
    exit 1  refused; one line 'refused: <name>: <detail>' on stderr per refusal

The leg is a check, not a provisioning engine. It reads the vendor/rauthy pin with
git, reads two members of the versions file (deploy/identity/versions.json unless
--versions-file names another), and checks the container runtime, both images and
the Rauthy image's org.opencontainers.image.revision label before it creates
anything, so a refusal among those checks leaves nothing behind.

Invariants, each stated so a reader can hold the code to it:

- Nothing is pulled, built, tagged or relabelled. Every `docker run` carries
  `--pull never`, and the Rauthy image is named only by its local image ID.
- An image with no revision label is refused; no other evidence stands in for it.
- The only port probed is the one `docker port` reports for this run's own Rauthy
  container, on 127.0.0.1.
- Every object this run creates carries the name lys-rauthy-ready-<run>-<role> and
  the label io.lys.gate.rauthy-ready=<run>; removal goes by those exact names, on
  every exit path including SIGINT and SIGTERM, containers with their anonymous
  volumes first, then the network, then the private temporary directory.
- The five secret values are drawn from the secrets module per run and reach the
  runtime only through --env-file files of mode 0600 in the private temporary
  directory. The runtime's own output is captured and never printed, so no secret,
  container log or container inspect output reaches this script's output.
- Nothing is written outside the private temporary directory; the versions file is
  only read.
"""

import base64
import json
import os
import re
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_VERSIONS_FILE = REPO_ROOT / "deploy" / "identity" / "versions.json"

LABEL_KEY = "io.lys.gate.rauthy-ready"
NAME_PREFIX = "lys-rauthy-ready-"
REVISION_LABEL = "org.opencontainers.image.revision"
READINESS_BOUND_SECONDS = 60
RAUTHY_HTTP_PORT = "8080/tcp"
DB_NAME = "rauthy"
DB_USER = "rauthy"

IMAGE_ID_FORM = re.compile(r"^sha256:[0-9a-f]{64}$")
DIGEST_REFERENCE_FORM = re.compile(r"^[^@\s]+@sha256:[0-9a-f]{64}$")
PIN_FORM = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
LOOPBACK_BINDING = re.compile(r"^127\.0\.0\.1:([0-9]{1,5})$")


class Refusal(Exception):
    """A named refusal: printed as 'refused: <name>: <detail>' and exit 1."""

    def __init__(self, name, detail):
        super().__init__(name)
        self.name = name
        self.detail = detail

    def line(self):
        return "refused: {}: {}".format(self.name, self.detail)


class Interrupted(Exception):
    """Raised from the SIGINT and SIGTERM handlers so the cleanup always runs."""

    def __init__(self, signum):
        super().__init__(signum)
        self.signum = signum


def build_command(pin):
    return "docker buildx build --label {}={} --load .".format(REVISION_LABEL, pin)


def parse_arguments(argv):
    """The one optional argument is --versions-file <path>; nothing else is taken."""
    if not argv:
        return DEFAULT_VERSIONS_FILE
    if len(argv) == 2 and argv[0] == "--versions-file" and argv[1]:
        return Path(argv[1])
    raise Refusal(
        "arguments_invalid",
        "the only argument taken is --versions-file <path>; got {}".format(json.dumps(argv)),
    )


def read_pin():
    """(1) The commit git rev-parse HEAD:vendor/rauthy prints; no submodule init."""
    try:
        done = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD:vendor/rauthy"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except OSError as error:
        raise Refusal(
            "rauthy_pin_unreadable",
            "git could not be run to read HEAD:vendor/rauthy in {} ({})".format(
                REPO_ROOT, error.strerror
            ),
        )
    pin = done.stdout.decode("utf-8", "replace").strip()
    if done.returncode != 0 or not PIN_FORM.match(pin):
        raise Refusal(
            "rauthy_pin_unreadable",
            "git rev-parse HEAD:vendor/rauthy in {} exited {} without printing a commit".format(
                REPO_ROOT, done.returncode
            ),
        )
    return pin


def read_versions(path):
    """(2) rauthy_image_id and postgres_image, each in the form the leg requires."""
    try:
        with open(str(path), "r", encoding="utf-8") as handle:
            document = json.load(handle)
    except (OSError, ValueError) as error:
        raise Refusal(
            "versions_unreadable",
            "{} could not be read as JSON ({})".format(path, type(error).__name__),
        )
    if not isinstance(document, dict):
        raise Refusal("versions_unreadable", "{} is not a JSON object".format(path))
    forms = (
        ("rauthy_image_id", IMAGE_ID_FORM, "sha256:<64 lowercase hex>"),
        ("postgres_image", DIGEST_REFERENCE_FORM, "<repository>@sha256:<64 lowercase hex>"),
    )
    bad = []
    for member, form, spelled in forms:
        value = document.get(member)
        if not isinstance(value, str) or not form.match(value):
            bad.append("{} (missing, not a string, or not {})".format(member, spelled))
    if bad:
        raise Refusal("versions_unreadable", "{}: {}".format(path, "; ".join(bad)))
    return document["rauthy_image_id"], document["postgres_image"]


def docker(args):
    """Run one docker command; its stdout is returned and never printed."""
    try:
        done = subprocess.run(
            ["docker"] + args,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except OSError as error:
        raise Refusal(
            "container_runtime_missing",
            "the docker client could not be run ({})".format(error.strerror),
        )
    return done.returncode, done.stdout.decode("utf-8", "replace")


def docker_step(step, args):
    """A runtime command whose non-zero exit is refused as runtime_step_failed."""
    status, out = docker(args)
    if status != 0:
        raise Refusal("runtime_step_failed", "{} exited {}".format(step, status))
    return out


def check_runtime():
    """(3) The docker client is on PATH and docker version reaches its daemon."""
    if shutil.which("docker") is None:
        raise Refusal("container_runtime_missing", "no docker client on PATH")
    status, _ = docker(["version", "--format", "{{.Server.Version}}"])
    if status != 0:
        raise Refusal(
            "container_runtime_missing",
            "docker version could not reach the daemon (exit {})".format(status),
        )


def check_images(rauthy_image_id, postgres_image, pin):
    """(4) Both images present locally, Rauthy first; (5) its revision label is the pin."""
    status, _ = docker(["image", "inspect", "--format", "{{.Id}}", rauthy_image_id])
    if status != 0:
        raise Refusal(
            "image_missing",
            "rauthy image {} is not present locally; it is built from the vendor/rauthy pin {}: "
            "in the Rauthy source checked out at {} run `{}`, then read the ID back with "
            "`docker image inspect --format '{{{{.Id}}}}' {}`".format(
                rauthy_image_id, pin, pin, build_command(pin), rauthy_image_id
            ),
        )
    status, _ = docker(["image", "inspect", "--format", "{{.Id}}", postgres_image])
    if status != 0:
        raise Refusal(
            "image_missing",
            "postgres image {} is not present locally; pull it before running: "
            "docker pull {}".format(postgres_image, postgres_image),
        )
    out = docker_step(
        "docker image inspect of the rauthy image's labels",
        ["image", "inspect", "--format", "{{json .Config.Labels}}", rauthy_image_id],
    )
    try:
        labels = json.loads(out)
    except ValueError:
        raise Refusal(
            "runtime_step_failed",
            "docker image inspect of the rauthy image's labels printed no JSON",
        )
    revision = labels.get(REVISION_LABEL) if isinstance(labels, dict) else None
    if not isinstance(revision, str) or not revision:
        raise Refusal(
            "rauthy_revision_mismatch",
            "rauthy image {} carries no {} label (absent), so nothing shows it was built from "
            "the vendor/rauthy pin {}; build a labelled image in the Rauthy source checked out "
            "at {} with `{}`".format(
                rauthy_image_id, REVISION_LABEL, pin, pin, build_command(pin)
            ),
        )
    if revision != pin:
        raise Refusal(
            "rauthy_revision_mismatch",
            "rauthy image {} has {}={} but the vendor/rauthy pin is {}".format(
                rauthy_image_id, REVISION_LABEL, json.dumps(revision), pin
            ),
        )


def write_env_file(directory, name, entries):
    """An --env-file created with mode 0600; its contents are never printed."""
    path = os.path.join(directory, name)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
        for key, value in entries:
            handle.write("{}={}\n".format(key, value))
    return path


class Run:
    """Everything one run creates, recorded by exact name so cleanup removes only it."""

    def __init__(self):
        self.run_id = secrets.token_hex(8)
        self.label = "{}={}".format(LABEL_KEY, self.run_id)
        self.tempdir = None
        self.network = None
        self.containers = []
        self.attempted = []

    def name(self, role):
        return "{}{}-{}".format(NAME_PREFIX, self.run_id, role)

    def make_tempdir(self):
        self.tempdir = tempfile.mkdtemp(prefix=NAME_PREFIX)

    def create_network(self):
        name = self.name("network")
        docker_step(
            "docker network create {}".format(name),
            ["network", "create", "--label", self.label, name],
        )
        self.network = name

    def start(self, role, env_file, image, publish):
        name = self.name(role)
        args = [
            "run", "--detach", "--pull", "never", "--name", name, "--label", self.label,
            "--network", self.network, "--env-file", env_file,
        ]
        if publish:
            args += ["--publish", "127.0.0.1::{}".format(RAUTHY_HTTP_PORT)]
        self.attempted.append(name)
        status, _ = docker(args + [image])
        if status != 0:
            raise Refusal("runtime_step_failed", "docker run {} exited {}".format(name, status))
        self.containers.append(name)
        return name

    def cleanup(self):
        """Remove containers with their anonymous volumes, then the network, then the
        temporary directory. Returns a refusal for every removal that failed."""
        failures = []

        def remove(step, args):
            try:
                status, _ = docker(args)
            except Refusal as refusal:
                failures.append(refusal)
                return
            if status != 0:
                failures.append(
                    Refusal("runtime_step_failed", "{} exited {}".format(step, status))
                )

        for name in self.attempted:
            if name in self.containers:
                continue
            # A docker run that failed may still have created the container.
            try:
                status, _ = docker(["container", "inspect", "--format", "{{.Id}}", name])
            except Refusal as refusal:
                failures.append(refusal)
                continue
            if status == 0:
                self.containers.append(name)
        for name in reversed(self.containers):
            remove("docker stop {}".format(name), ["stop", name])
            remove("docker rm {}".format(name), ["rm", "--volumes", name])
        if self.network is not None:
            remove("docker network rm {}".format(self.network), ["network", "rm", self.network])
        if self.tempdir is not None:
            try:
                shutil.rmtree(self.tempdir)
            except OSError as error:
                failures.append(Refusal(
                    "runtime_step_failed",
                    "removing the temporary directory {} failed ({})".format(
                        self.tempdir, error.strerror
                    ),
                ))
        return failures


def draw_secrets():
    """The five secret values, drawn per run and never printed."""
    key_id = secrets.token_hex(8)
    return {
        "db_password": secrets.token_hex(24),
        "enc_key_id": key_id,
        "enc_keys": "{}/{}".format(key_id, base64.b64encode(secrets.token_bytes(32)).decode()),
        "admin_password": secrets.token_urlsafe(24),
        "hql_raft": secrets.token_hex(32),
        "hql_api": secrets.token_hex(32),
    }


def wait_for_postgres(postgres):
    """(7) pg_isready over TCP inside the PostgreSQL container, within the bound."""
    deadline = time.monotonic() + READINESS_BOUND_SECONDS
    while True:
        status, _ = docker(
            ["exec", postgres, "pg_isready", "-q", "-h", "127.0.0.1", "-U", DB_USER, "-d", DB_NAME]
        )
        if status == 0:
            return
        if time.monotonic() >= deadline:
            raise Refusal(
                "readiness_timeout",
                "postgres container {} did not answer pg_isready within {} seconds".format(
                    postgres, READINESS_BOUND_SECONDS
                ),
            )
        time.sleep(1)


def published_port(rauthy):
    """(8) The host port the runtime picked for this run's own Rauthy container."""
    out = docker_step("docker port {}".format(rauthy), ["port", rauthy, RAUTHY_HTTP_PORT])
    for line in out.splitlines():
        match = LOOPBACK_BINDING.match(line.strip())
        if match:
            return int(match.group(1))
    raise Refusal(
        "runtime_step_failed",
        "docker port {} reported no 127.0.0.1 binding for {}".format(rauthy, RAUTHY_HTTP_PORT),
    )


def answered_ready(opener, url, remaining):
    """(9) Only HTTP 200 with db_healthy and cache_healthy both true counts."""
    try:
        with opener.open(url, timeout=max(remaining, 0.1)) as response:
            if response.status != 200:
                return False
            body = json.loads(response.read().decode("utf-8", "replace"))
    except (urllib.error.URLError, OSError, ValueError):
        return False
    return (
        isinstance(body, dict)
        and body.get("db_healthy") is True
        and body.get("cache_healthy") is True
    )


def wait_for_rauthy(port):
    url = "http://127.0.0.1:{}/auth/v1/health".format(port)
    # No proxy from the environment is consulted: the probe goes to loopback only.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    deadline = time.monotonic() + READINESS_BOUND_SECONDS
    while True:
        if answered_ready(opener, url, deadline - time.monotonic()):
            return url
        if time.monotonic() >= deadline:
            raise Refusal(
                "readiness_timeout",
                "rauthy did not answer ready at {} within {} seconds".format(
                    url, READINESS_BOUND_SECONDS
                ),
            )
        time.sleep(1)


def start_and_probe(run, rauthy_image_id, postgres_image):
    """(6) to (9): create the network and both containers, and wait for ready."""
    values = draw_secrets()
    run.make_tempdir()
    postgres_env = write_env_file(run.tempdir, "postgres.env", [
        ("POSTGRES_USER", DB_USER),
        ("POSTGRES_DB", DB_NAME),
        ("POSTGRES_PASSWORD", values["db_password"]),
    ])
    postgres_name = run.name("postgres")
    rauthy_env = write_env_file(run.tempdir, "rauthy.env", [
        ("HIQLITE", "false"),
        ("PG_HOST", postgres_name),
        ("PG_PORT", "5432"),
        ("PG_DB_NAME", DB_NAME),
        ("PG_USER", DB_USER),
        ("PG_PASSWORD", values["db_password"]),
        ("PG_TLS", "false"),
        ("PG_TLS_NO_VERIFY", "false"),
        ("HEALTH_CHECK_DELAY_SECS", "0"),
        ("HQL_NODE_ID", "1"),
        ("HQL_NODES", "1 localhost:8100 localhost:8200"),
        ("HQL_DATA_DIR", "/app/data"),
        ("HQL_SECRET_RAFT", values["hql_raft"]),
        ("HQL_SECRET_API", values["hql_api"]),
        ("ENC_KEYS", values["enc_keys"]),
        ("ENC_KEY_ACTIVE", values["enc_key_id"]),
        ("LISTEN_SCHEME", "http"),
        ("PUB_URL", "localhost:8080"),
        ("RP_ID", "localhost"),
        ("RP_ORIGIN", "http://localhost:8080"),
        ("PROXY_MODE", "false"),
        ("BOOTSTRAP_ADMIN_EMAIL", "admin@rauthy-ready.invalid"),
        ("BOOTSTRAP_ADMIN_PASSWORD_PLAIN", values["admin_password"]),
        ("LOG_LEVEL", "info"),
    ])
    run.create_network()
    postgres = run.start("postgres", postgres_env, postgres_image, publish=False)
    wait_for_postgres(postgres)
    rauthy = run.start("rauthy", rauthy_env, rauthy_image_id, publish=True)
    return wait_for_rauthy(published_port(rauthy))


def on_signal(signum, _frame):
    raise Interrupted(signum)


def main(argv):
    signal.signal(signal.SIGINT, on_signal)
    signal.signal(signal.SIGTERM, on_signal)
    run = Run()
    refusals = []
    ready = None
    try:
        versions_file = parse_arguments(argv)
        pin = read_pin()
        rauthy_image_id, postgres_image = read_versions(versions_file)
        check_runtime()
        check_images(rauthy_image_id, postgres_image, pin)
        url = start_and_probe(run, rauthy_image_id, postgres_image)
        ready = "ready: rauthy image {} built from vendor/rauthy pin {} answered {} with " \
            "db_healthy and cache_healthy true".format(rauthy_image_id, pin, url)
    except Refusal as refusal:
        refusals.append(refusal)
    except OSError as error:
        refusals.append(Refusal(
            "runtime_step_failed",
            "preparing the private temporary directory failed ({})".format(error.strerror),
        ))
    except Interrupted as interruption:
        refusals.append(Refusal(
            "interrupted", "signal {} received; the run was stopped".format(interruption.signum)
        ))
    finally:
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        refusals.extend(run.cleanup())
    if refusals:
        for refusal in refusals:
            sys.stderr.write(refusal.line() + "\n")
        return 1
    if ready is None:
        sys.stderr.write("refused: runtime_step_failed: the run ended without a ready answer\n")
        return 1
    sys.stdout.write(ready + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
