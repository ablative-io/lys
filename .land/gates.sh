#!/bin/sh
# The gates lys runs before any commit: CLAUDE.md's "Gates before any commit" list,
# and the ast-grep scan of sgconfig.yml's rules. repo_land runs this file as the whole gate when it stands here. Every leg
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
# The identity leg runs the container-backed identity targets, which are declared
# test = false so cargo test --workspace stays hermetic. It runs on every lys landing
# and is never scoped away. Without a container runtime it fails by name; it never
# skips. It lints every identity target before it runs any of them.
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
  cargo test -p lys --all-features --no-fail-fast --test 'identity_*'
}
# The surface leg installs dependencies, checks types and runs the identity tests:
# npm ci, npm run typecheck and npm test,
# with surface/identity as the package directory, so the shell's tests and the
# mock-up's run on every lys landing. It is never scoped away by a changed-path
# filter. Without npm it fails by name; it never skips.
surface_leg() {
  if ! command -v npm >/dev/null 2>&1; then
    echo "surface_npm_missing: the surface leg needs npm on the PATH to run npm ci and npm test in surface/identity"
    return 1
  fi
  (cd surface/identity && npm ci && npm run typecheck && npm test)
}
leg sh scripts/design/gate.sh
leg cargo fmt --check
leg cargo clippy --all-targets --all-features -- -D warnings
leg cargo clippy --all-targets -- -D warnings
leg cargo test --workspace --all-features --no-fail-fast
leg cargo doc --no-deps --all-features
leg cargo doc --no-deps
leg ast-grep scan --config sgconfig.yml
leg sh scripts/file-length.sh
leg identity_leg
leg surface_leg
exit "$status"
