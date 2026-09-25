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
exit "$status"
