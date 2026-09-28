#!/bin/sh
# The identity-release leg: a demand leg in docs/design/project.json, run by the identity release
# card (DIRECTORY-030) and never by .land/gates.sh. It gates the exact pushed lys commit and
# nothing else, so every refusal fires before any leg runs and none falls back to a local build.
#
# Refusals, checked in this order, the first that fires the only one reported (exit 1, stdout
# empty): an unclean working tree; a failed fetch of origin in the repository or of ablative in
# vendor/rauthy; a vendor/rauthy pin that is not an ancestor of origin/ablative; a HEAD that no
# remote branch contains. The script does its own fetching; the caller fetches nothing.
#
# Otherwise the six lys legs of CLAUDE.md's "Gates before any commit" run in its order, every leg
# even after a red one. Each leg's own output goes to stderr; stdout carries exactly one JSON line
# per leg with the keys repository, ref, kind, command and exit_status, and nothing else. Exit 0
# only when every leg exited 0.
#
# Writes nothing in either working tree: the only writes are the refs and objects the two
# fetches store under git's own directories and the build output under the ignored cargo target
# directory. Runs from the repository root in a fresh clone with vendor/rauthy initialised.
set -u
set -f
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)" || exit 2
root=$(pwd -P)

refuse() {
  echo "identity-release: $*" >&2
  exit 1
}

if [ -n "$(git status --porcelain)" ]; then
  refuse "working tree is not clean; commit or remove every change before gating the release"
fi

git fetch origin >&2 || refuse "fetch failed: git fetch origin"
# Without an initialised submodule, git -C vendor/rauthy would resolve to the parent repository
# and fetch there; that is a failed submodule fetch, not a fetch to fall through to.
sub_top=$(git -C vendor/rauthy rev-parse --show-toplevel 2>/dev/null)
if [ "$sub_top" != "$root/vendor/rauthy" ]; then
  refuse "fetch failed: git -C vendor/rauthy fetch origin ablative (vendor/rauthy is not an initialised submodule)"
fi
git -C vendor/rauthy fetch origin ablative >&2 ||
  refuse "fetch failed: git -C vendor/rauthy fetch origin ablative"

pin=$(git ls-tree HEAD vendor/rauthy | awk '{print $3}')
[ -n "$pin" ] || refuse "vendor/rauthy has no pinned commit at HEAD, so it is not on ablative"
if ! git -C vendor/rauthy merge-base --is-ancestor "$pin" refs/remotes/origin/ablative 2>/dev/null; then
  refuse "vendor/rauthy pin $pin is not on ablative"
fi

head=$(git rev-parse HEAD) || refuse "cannot read HEAD"
if [ -z "$(git branch -r --contains "$head")" ]; then
  refuse "HEAD $head is not pushed to any remote branch"
fi

status=0
leg() {
  kind=$1
  shift
  cmd=$*
  $cmd >&2
  code=$?
  [ "$code" -eq 0 ] || status=1
  printf '{"repository": "lys", "ref": "%s", "kind": "%s", "command": "%s", "exit_status": %d}\n' \
    "$head" "$kind" "$cmd" "$code"
}
leg gate cargo fmt --check
leg gate cargo clippy --all-targets --all-features -- -D warnings
leg gate cargo clippy --all-targets -- -D warnings
leg test cargo test --workspace --all-features
leg gate cargo doc --no-deps --all-features
leg gate cargo doc --no-deps
exit "$status"
