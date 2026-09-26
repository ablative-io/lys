#!/bin/sh
# The gates lys runs before any commit, exactly as CLAUDE.md "Gates before any commit"
# lists them. repo_land runs this file as the whole gate when it stands here. Every leg
# runs even after a red one, so the log covers all of them; the exit status is red if
# any leg was.
set -u
status=0
leg() {
  echo "--- $* ---"
  "$@"
  code=$?
  echo "--- status $code: $* ---"
  [ "$code" -eq 0 ] || status=1
}
leg sh scripts/design/gate.sh
leg cargo fmt --check
leg cargo clippy --all-targets --all-features -- -D warnings
leg cargo clippy --all-targets -- -D warnings
leg cargo test --workspace --all-features
leg cargo doc --no-deps --all-features
leg cargo doc --no-deps
# The identity leg (DIRECTORY-002 R1): the container-backed identity targets are declared
# test = false, so the legs above never build or run them. It runs on every landing and is
# never scoped away. Without a container runtime it fails by name and runs no identity
# target; with one, it lints the targets first and runs their tests only when the lint is
# clean, so a warning in any of them fails the leg before a test starts.
identity_leg() {
  if ! docker info >/dev/null 2>&1; then
    echo "container_runtime_missing: no container runtime answered docker info; no identity target ran"
    return 1
  fi
  if ! cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings; then
    echo "identity_lint_failed: an identity target has a lint warning or error; no identity test ran"
    return 1
  fi
  cargo test -p lys --all-features --test 'identity_*'
}
leg identity_leg
exit "$status"
