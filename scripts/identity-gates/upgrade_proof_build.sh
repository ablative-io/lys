#!/bin/sh
# The build half of the real old-install upgrade proof. It runs on the build
# machine, never where a person's Lys is installed. For this tree's clean
# candidate and each old release the proof is required against, it builds the
# release programs and packages the screens, and copies only those into one
# artifact folder that upgrade_proof_run.sh reads:
#
#   <artifacts>/candidate/bin        the candidate's release programs and examples/upgrade_window
#   <artifacts>/candidate/surface    the candidate's packaged screens
#   <artifacts>/old-<sha>/bin        that release's programs
#   <artifacts>/old-<sha>/surface    that release's packaged screens
#
# Each old release is built from its own clone, one folder per commit under
# <clones>, never from a worktree of this checkout. A clone is kept between
# runs, so a second build of the same release reuses its target.
#
# Usage: upgrade_proof_build.sh <artifacts> <clones>
set -eu

if [ "$#" -ne 2 ]; then
  echo "upgrade_proof_usage: upgrade_proof_build.sh <artifacts> <clones>"
  exit 2
fi
artifacts=$1
clones=$2
here=$(cd "$(dirname "$0")" && pwd)
. "$here/upgrade_proof_baselines.sh"
root=$(git rev-parse --show-toplevel)
candidate_commit=$(git -C "$root" rev-parse HEAD)
if [ -n "$(git -C "$root" status --porcelain)" ]; then
  echo "upgrade_proof_dirty: the candidate tree has uncommitted changes; the proof needs a clean commit"
  exit 1
fi
for tool in cargo npm git; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "upgrade_proof_tool_missing: the build half needs $tool on the PATH"
    exit 1
  fi
done
if [ -e "$artifacts" ]; then
  echo "upgrade_proof_artifacts_exist: $artifacts already exists; the build half writes a new folder"
  exit 1
fi
mkdir -p "$artifacts" "$clones"
chmod 700 "$artifacts"

package_surface() {
  # $1 source tree, $2 where the package is written
  (cd "$1/surface/identity" && npm ci --no-audit --no-fund >/dev/null && npm run build && npm run package -- "$2")
}

# Copy the release programs only: every executable file at the top of
# target/release, which is where cargo puts the built binaries.
copy_programs() {
  # $1 target/release, $2 destination
  mkdir -p "$2"
  for program in "$1"/*; do
    if [ -f "$program" ] && [ -x "$program" ]; then
      cp -p "$program" "$2/"
    fi
  done
}

echo "upgrade_proof: building the candidate $candidate_commit"
(cd "$root" && cargo build --locked --release -p lys -p lys-identity-server -p lys-secrets \
  && cargo build --locked --release -p lys --example upgrade_window)
copy_programs "$root/target/release" "$artifacts/candidate/bin"
mkdir -p "$artifacts/candidate/bin/examples"
cp -p "$root/target/release/examples/upgrade_window" "$artifacts/candidate/bin/examples/"
package_surface "$root" "$artifacts/candidate/surface"
printf '%s\n' "$candidate_commit" > "$artifacts/candidate/commit"

origin=$(git -C "$root" remote get-url origin)
for old_commit in $baselines; do
  clone="$clones/old-$old_commit"
  echo "upgrade_proof: building the old release $old_commit in $clone"
  if [ ! -d "$clone/.git" ]; then
    git clone --quiet "$origin" "$clone"
  fi
  git -C "$clone" fetch --quiet origin
  git -C "$clone" checkout --quiet --detach "$old_commit"
  if [ -n "$(git -C "$clone" status --porcelain)" ]; then
    echo "upgrade_proof_dirty: the old release clone $clone has uncommitted changes"
    exit 1
  fi
  git -C "$clone" submodule update --init vendor/rauthy >/dev/null
  (cd "$clone" && cargo build --locked --release -p lys -p lys-identity-server -p lys-secrets)
  copy_programs "$clone/target/release" "$artifacts/old-$old_commit/bin"
  package_surface "$clone" "$artifacts/old-$old_commit/surface"
done
echo "upgrade_proof: artifacts for $candidate_commit and $(echo $baselines | wc -w | tr -d ' ') old releases are in $artifacts"
