#!/bin/sh
# Every leg finishes so a failure carries every independent finding.
# Independent checks run together after the shared compile checks.
set -u
PATH="$(pwd -P)/target/land-tools/bin:$PATH"
export PATH
status=0
leg() {
  started=$(date +%s)
  echo "--- $* ---"
  "$@"
  code=$?
  echo "--- status $code: $* ---"
  echo "--- seconds $(( $(date +%s) - started )): $* ---"
  [ "$code" -eq 0 ] || status=1
}
# Container-backed targets are selected explicitly because the workspace suite
# excludes them by default. Missing prerequisites fail rather than skip a test.
identity_leg() {
  if ! docker info >/dev/null 2>&1; then
    echo "container_runtime_missing: the identity leg needs a container runtime answering docker info"
    return 1
  fi
  # The pin-clone test reads vendor/rauthy as a recursive clone checks it out.
  if ! init_error=$(git submodule update --init vendor/rauthy 2>&1); then
    echo "submodule_init_failed: git submodule update --init vendor/rauthy: $init_error"
    return 1
  fi
  if ! cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings; then
    echo "identity_lint_failed: an identity target has a lint warning; no identity test was run"
    return 1
  fi
  identity_status=0
  cargo nextest run -p lys --all-features --test 'identity_*' --no-fail-fast --retries 0 --no-tests fail || identity_status=1
  python3 -B scripts/identity-gates/spicedb_fixture.py -- cargo nextest run --locked -p lys-identity-server --all-features --test identity_spicedb --no-fail-fast --retries 0 --no-tests fail || identity_status=1
  return "$identity_status"
}
# The compiled surface and dependencies are shared with the install fixtures.
# Its type checks and every surface test still run on each gate.
surface_leg() {
  if ! command -v npm >/dev/null 2>&1; then
    echo "surface_npm_missing: the surface leg needs npm on the PATH to run npm ci and npm test in surface/identity"
    return 1
  fi
  prepared_surface=$(python3 scripts/identity-gates/surface_fixture.py) || return 1
  (cd "$prepared_surface" && npm run typecheck && npm test)
}
parallel() {
  job_index=$((job_index + 1))
  (status=0; leg "$@"; exit "$status") > "$gate_logs/$job_index" 2>&1 &
  gate_pids="$gate_pids $!"
}
finish_parallel() {
  finished_index=0
  for gate_pid in $gate_pids; do
    finished_index=$((finished_index + 1))
    if wait "$gate_pid"; then
      :
    else
      status=1
    fi
    if ! cat "$gate_logs/$finished_index"; then
      echo "gate_log_unreadable: parallel leg $finished_index did not yield its log"
      status=1
    fi
  done
}
source_changed() {
  if ! git diff --quiet HEAD --; then
    echo "source_changed: formatting changed tracked source"
    return 1
  fi
}
leg cargo fmt --all
leg source_changed
leg cargo clippy --all-targets --all-features -- -D warnings
leg cargo clippy --all-targets -- -D warnings
mkdir -p target || exit 1
gate_logs=$(mktemp -d target/land-logs.XXXXXX) || exit 1
trap 'rm -rf "$gate_logs"' EXIT
job_index=0
gate_pids=""
parallel sh scripts/design/gate.sh
parallel cargo doc --no-deps --all-features
parallel cargo doc --no-deps
parallel ast-grep scan --config sgconfig.yml
parallel sh scripts/file-length.sh
parallel python3 -B -m unittest discover -s scripts/identity-gates -p surface_fixture_tests.py
parallel python3 -B -m unittest discover -s scripts/identity-gates -p 'test_*.py'
parallel surface_leg
leg cargo nextest run --workspace --all-features --no-fail-fast --retries 0 --no-tests fail
leg cargo test --doc --workspace --all-features
leg identity_leg
leg sh scripts/identity-gates/upgrade_proof_leg.sh
finish_parallel
exit "$status"
