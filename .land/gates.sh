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
# The identity leg: the container-backed targets declared `test = false` in
# crates/lys/Cargo.toml. It runs on every landing and is never scoped away. With
# no container runtime answering it fails by name rather than skipping; with one,
# it lints the identity targets first and runs them only if the lint is clean.
identity_leg() {
  if ! docker info >/dev/null 2>&1; then
    echo "container_runtime_missing: docker info did not answer; the identity leg does not skip"
    return 1
  fi
  if ! cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings; then
    echo "identity leg: cargo clippy failed on the identity targets; no identity test was run"
    return 1
  fi
  cargo test -p lys --all-features --test 'identity_*'
}
leg identity_leg
exit "$status"
