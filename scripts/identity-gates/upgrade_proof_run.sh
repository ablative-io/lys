#!/bin/sh
# The run half of the real old-install upgrade proof. It compiles nothing: it
# reads the artifact folder upgrade_proof_build.sh wrote and, for each old
# release, runs upgrade_live.py, preparation first, then the real upgrade of a
# private disposable install under Docker. The old and candidate sources are
# read from this repository at their commits, so it needs the commits, not a
# tree of each. Every fixture lives under one private folder that is removed
# when the leg ends, whatever its result; evidence of a failed run is printed
# first.
#
# The old releases bind their service and broker ports, 8490 and 8472, which
# are compiled into them; upgrade_live.py refuses by name when either is held.
#
# Usage: upgrade_proof_run.sh <artifacts>
set -eu

if [ "$#" -ne 1 ]; then
  echo "upgrade_proof_usage: upgrade_proof_run.sh <artifacts>"
  exit 2
fi
artifacts=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
. "$here/upgrade_proof_baselines.sh"
root=$(git -C "$here" rev-parse --show-toplevel)
for tool in python3 docker git; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "upgrade_proof_tool_missing: the run half needs $tool on the PATH"
    exit 1
  fi
done
candidate_commit=$(cat "$artifacts/candidate/commit")
for commit in "$candidate_commit" $baselines; do
  if ! git -C "$root" cat-file -e "$commit^{commit}" 2>/dev/null; then
    echo "upgrade_proof_commit_missing: $commit is not in $root; fetch it first"
    exit 1
  fi
done

# Under /tmp, never the seat's TMPDIR: a fixture's socket path must stay
# inside the 104 bytes macOS allows, and a per-user TMPDIR on macOS is long
# enough to break it before any install starts.
private=$(mktemp -d /tmp/lys-upgrade-proof.XXXXXX)
chmod 700 "$private"
# Cleanup never replaces the leg's own exit code.
cleanup() {
  status=$?
  rm -rf "$private"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

for old_commit in $baselines; do
  work="$private/w-$(printf %s "$old_commit" | cut -c1-8)"
  set -- \
    --old-commit "$old_commit" \
    --old-bin "$artifacts/old-$old_commit/bin" \
    --old-surface "$artifacts/old-$old_commit/surface" \
    --candidate-bin "$artifacts/candidate/bin" \
    --candidate-surface "$artifacts/candidate/surface" \
    --candidate-commit "$candidate_commit" \
    --work "$work"
  python3 -B "$root/scripts/identity-gates/upgrade_live.py" "$@" --prepare-only
  if ! python3 -B "$root/scripts/identity-gates/upgrade_live.py" "$@"; then
    echo "upgrade_proof_failed: the upgrade from $old_commit to $candidate_commit failed; evidence follows"
    find "$work" -name '*.log' -type f -exec tail -n 40 {} \;
    exit 1
  fi
  echo "upgrade_proof: $old_commit upgrades to $candidate_commit with every record kept"
done
