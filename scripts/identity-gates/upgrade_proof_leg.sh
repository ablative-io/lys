#!/bin/sh
# The real old-install upgrade proof, as a gate leg. For each old release the
# proof is required against (the HOME-037 and DIRECTORY-069 baseline, an
# earlier install, the 2 October 14:35 release, and the 2 October 16:20
# release that was live before the evening's install), it builds that release clean from its own
# commit, packages its screens, builds this tree's candidate the same way, and
# runs upgrade_live.py: preparation first, then the real upgrade of a private
# disposable install. Every build and fixture lives under one private folder
# that is removed when the leg ends, whatever its result; evidence of a failed
# run is copied to the gate's log first.
set -eu

baselines="1b568cd90578f5ed5d7d438e628b23724eef7f12 8c064b62a0c77f0874c203189a2ed3238b7ba57a cced4195169ff5b458278dd0eb6dae607fde63a2 255a3bca78995cc31e26f9e3107b2e494fe5bc3a"
root=$(git rev-parse --show-toplevel)
candidate_commit=$(git -C "$root" rev-parse HEAD)
if [ -n "$(git -C "$root" status --porcelain)" ]; then
  echo "upgrade_proof_dirty: the candidate tree has uncommitted changes; the proof needs a clean commit"
  exit 1
fi
for tool in cargo npm python3 docker; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "upgrade_proof_tool_missing: the upgrade proof needs $tool on the PATH"
    exit 1
  fi
done

# Under /tmp, never the seat's TMPDIR: a fixture's socket path must stay
# inside the 104 bytes macOS allows, and a per-user TMPDIR on macOS is long
# enough to break it before any install starts.
private=$(mktemp -d /tmp/lys-upgrade-proof.XXXXXX)
chmod 700 "$private"
cleanup() {
  for tree in "$private"/old-*; do
    [ -d "$tree" ] && git -C "$root" worktree remove --force "$tree" >/dev/null 2>&1
  done
  git -C "$root" worktree prune
  rm -rf "$private"
}
trap cleanup EXIT INT TERM

package_surface() {
  # $1 source tree, $2 where the package is written
  (cd "$1/surface/identity" && npm ci --no-audit --no-fund >/dev/null && npm run build && npm run package -- "$2")
}

echo "upgrade_proof: building the candidate $candidate_commit"
(cd "$root" && cargo build --locked --release -p lys -p lys-identity-server -p lys-secrets \
  && cargo build --locked --release -p lys --example upgrade_window)
package_surface "$root" "$private/candidate-surface"

for old_commit in $baselines; do
  old_tree="$private/old-$old_commit"
  echo "upgrade_proof: building the old release $old_commit"
  git -C "$root" worktree add --detach "$old_tree" "$old_commit" >/dev/null
  git -C "$old_tree" submodule update --init vendor/rauthy >/dev/null
  (cd "$old_tree" && cargo build --locked --release -p lys -p lys-identity-server -p lys-secrets)
  package_surface "$old_tree" "$private/old-surface-$old_commit"
  set -- \
    --old-source "$old_tree" \
    --old-commit "$old_commit" \
    --old-bin "$old_tree/target/release" \
    --old-surface "$private/old-surface-$old_commit" \
    --candidate-bin "$root/target/release" \
    --candidate-surface "$private/candidate-surface" \
    --candidate-commit "$candidate_commit" \
    --work "$private/w-$(printf %s "$old_commit" | cut -c1-8)"
  python3 -B "$root/scripts/identity-gates/upgrade_live.py" "$@" --prepare-only
  if ! python3 -B "$root/scripts/identity-gates/upgrade_live.py" "$@"; then
    echo "upgrade_proof_failed: the upgrade from $old_commit to $candidate_commit failed; evidence follows"
    find "$private/w-$(printf %s "$old_commit" | cut -c1-8)" -name '*.log' -type f -exec tail -n 40 {} \;
    exit 1
  fi
  echo "upgrade_proof: $old_commit upgrades to $candidate_commit with every record kept"
done
